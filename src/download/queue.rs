//! Download queue manager: link resolution, parallel download scheduling with a semaphore,
//! duplicate filtering via the project's downloaded-ID set, and progress state shared with
//! the UI thread.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use crate::api::client::ApiClient;
use crate::download::pipeline::{self, DownloadConfig, DownloadOutcome, TrackJob};
use crate::project::Project;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// State of a single queue entry as displayed in the UI.
#[derive(Debug, Clone)]
pub enum ItemState {
    /// Waiting to start (held behind the semaphore).
    Queued,
    /// Currently downloading.
    Downloading,
    /// Finished successfully; stores a brief description of the result.
    Done {
        path: String,
        codec: String,
        bitrate: u32,
    },
    /// Finished with an error.
    Failed { error: String },
}

/// A queue entry: track metadata plus its current state.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub state: ItemState,
}

/// Download queue state shared between the UI thread and background tasks.
#[derive(Default)]
pub struct QueueState {
    pub items: Vec<QueueItem>,
    /// Status message from link resolution (e.g. "loading track list…") or an error string.
    pub resolve_status: Option<String>,
    pub resolving: bool,
}

/// Queue handle stored in `YmdApp`; cheaply cloned to share state with background tasks.
#[derive(Clone)]
pub struct DownloadQueue {
    state: Arc<Mutex<QueueState>>,
}

impl Default for DownloadQueue {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(QueueState::default())),
        }
    }
}

impl DownloadQueue {
    /// Returns a reference to the shared queue state (for rendering in the UI).
    #[must_use]
    pub fn state(&self) -> &Arc<Mutex<QueueState>> {
        &self.state
    }

    fn set_item_state(&self, id: u64, new_state: ItemState) {
        if let Ok(mut guard) = self.state.lock()
            && let Some(item) = guard.items.iter_mut().find(|i| i.id == id)
        {
            item.state = new_state;
        }
    }

    /// Submits a user-supplied URL for processing: resolves it to tracks, filters already-
    /// downloaded ones, and starts parallel downloads. Does not block the calling (UI) thread.
    pub fn enqueue_link(
        &self,
        runtime: &tokio::runtime::Runtime,
        client: Arc<ApiClient>,
        config: DownloadConfig,
        project: Arc<Mutex<Project>>,
        input: String,
        ctx: egui::Context,
    ) {
        if let Ok(mut guard) = self.state.lock() {
            guard.resolving = true;
            guard.resolve_status = Some("Загружаем список треков…".to_owned());
        }
        ctx.request_repaint();

        let queue = self.clone();
        runtime.spawn(async move {
            let jobs = match pipeline::resolve_link(&client, &input).await {
                Ok(jobs) => jobs,
                Err(err) => {
                    if let Ok(mut guard) = queue.state.lock() {
                        guard.resolving = false;
                        guard.resolve_status = Some(format!("Ошибка: {err}"));
                    }
                    ctx.request_repaint();
                    return;
                }
            };

            // Filter out tracks that are already recorded in the project.
            let jobs: Vec<TrackJob> = {
                let downloaded = project.lock().map(|g| g.downloaded_ids.clone()).unwrap_or_default();
                jobs.into_iter()
                    .filter(|j| !downloaded.contains(&j.full_id))
                    .collect()
            };

            if jobs.is_empty() {
                if let Ok(mut guard) = queue.state.lock() {
                    guard.resolving = false;
                    guard.resolve_status =
                        Some("Все треки уже есть в проекте.".to_owned());
                }
                ctx.request_repaint();
                return;
            }

            queue.run_jobs(client, config, project, jobs, ctx).await;
        });
    }

    async fn run_jobs(
        &self,
        client: Arc<ApiClient>,
        config: DownloadConfig,
        project: Arc<Mutex<Project>>,
        jobs: Vec<TrackJob>,
        ctx: egui::Context,
    ) {
        let mut ids = Vec::with_capacity(jobs.len());
        if let Ok(mut guard) = self.state.lock() {
            guard.resolving = false;
            guard.resolve_status = Some(format!("Добавлено треков: {}", jobs.len()));
            for job in &jobs {
                let id = next_id();
                ids.push(id);
                guard.items.push(QueueItem {
                    id,
                    title: job.track.full_title(),
                    artist: job.track.artist_names().join(", "),
                    state: ItemState::Queued,
                });
            }
        }
        ctx.request_repaint();

        let permits = config.max_parallel_downloads.max(1);
        let semaphore = Arc::new(Semaphore::new(permits));
        let http = reqwest::Client::new();
        let mut handles = Vec::new();

        for (job, id) in jobs.into_iter().zip(ids) {
            let permit_sem = semaphore.clone();
            let queue = self.clone();
            let client = client.clone();
            let http = http.clone();
            let config = config.clone();
            let project = project.clone();
            let ctx = ctx.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit_sem
                    .acquire_owned()
                    .await
                    .expect("semaphore is never closed");
                queue.set_item_state(id, ItemState::Downloading);
                ctx.request_repaint();

                let full_id = job.full_id.clone();
                let result: Result<DownloadOutcome, _> =
                    pipeline::download_track(client, http, job, config).await;

                match result {
                    Ok(outcome) => {
                        // Persist the downloaded track ID so future sessions skip it.
                        if let Ok(mut guard) = project.lock() {
                            guard.record_downloaded(&full_id);
                        }
                        queue.set_item_state(
                            id,
                            ItemState::Done {
                                path: outcome.path.display().to_string(),
                                codec: format!("{:?}", outcome.codec),
                                bitrate: outcome.bitrate,
                            },
                        );
                    }
                    Err(err) => queue.set_item_state(
                        id,
                        ItemState::Failed {
                            error: err.to_string(),
                        },
                    ),
                }
                ctx.request_repaint();
            }));
        }

        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Removes all completed (succeeded or failed) entries from the queue.
    pub fn clear_finished(&self) {
        if let Ok(mut guard) = self.state.lock() {
            guard
                .items
                .retain(|i| matches!(i.state, ItemState::Queued | ItemState::Downloading));
        }
    }
}
