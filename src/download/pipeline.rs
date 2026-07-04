//! End-to-end download pipeline for a single track: metadata → get-file-info → download →
//! `encraw` decryption → optional `flac-mp4` demux → cover art → file write → tags.
//!
//! Also handles expanding a user-supplied URL (track/album/playlist) into a list of tracks.

use std::path::PathBuf;
use std::sync::Arc;

use crate::api::client::ApiClient;
use crate::api::models::Track;
use crate::api::urls::ResourceLink;
use crate::api::{endpoints, urls};
use crate::config::{CoverSize, Settings};
use crate::download::quality::ResponseCodec;
use crate::download::{decrypt, flac_mp4, naming};
use crate::tags::{self, TrackMetadata};

/// Errors from the download pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error(transparent)]
    Url(#[from] urls::UrlParseError),
    #[error(transparent)]
    Endpoint(#[from] endpoints::EndpointError),
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Decrypt(#[from] decrypt::DecryptError),
    #[error(transparent)]
    FlacMp4(#[from] flac_mp4::FlacMp4Error),
    #[error(transparent)]
    Tag(#[from] tags::TagError),
    #[error("server returned no download URLs for the track")]
    NoDownloadUrl,
    #[error("unknown codec in response: {0}")]
    UnknownCodec(String),
    #[error("no download directory configured")]
    NoDownloadDir,
}

/// A single download job: full track metadata plus the composite `id:albumId` for the API.
#[derive(Debug, Clone)]
pub struct TrackJob {
    pub track: Track,
    /// Composite `id:albumId` for `get-file-info` (matches the desktop client's `trackId`).
    pub full_id: String,
    /// Total track count in the source (album / playlist) used for indexing and `TOTALTRACKS`.
    pub total_tracks: Option<u32>,
}

/// Result of a successfully completed track download.
#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub path: PathBuf,
    pub codec: ResponseCodec,
    pub bitrate: u32,
}

/// Expands a user-supplied URL into a list of tracks with full metadata.
///
/// # Errors
/// Returns an error if the URL is malformed or any network request to the API fails.
pub async fn resolve_link(client: &ApiClient, input: &str) -> Result<Vec<TrackJob>, PipelineError> {
    let link = urls::parse(input)?;
    match link {
        ResourceLink::Track { track_id, album_id } => {
            let full_id = match &album_id {
                Some(album) => format!("{track_id}:{album}"),
                None => track_id.clone(),
            };
            let tracks = endpoints::fetch_tracks(client, std::slice::from_ref(&full_id)).await?;
            Ok(tracks
                .into_iter()
                .map(|track| {
                    let full_id = track.full_id_param(album_id.as_deref());
                    TrackJob {
                        track,
                        full_id,
                        total_tracks: None,
                    }
                })
                .collect())
        }
        ResourceLink::Album { album_id } => {
            let album = endpoints::fetch_album_with_tracks(client, &album_id).await?;
            let volumes = album.volumes.clone().unwrap_or_default();
            let total: u32 = volumes.iter().map(|v| v.len() as u32).sum();
            let mut jobs = Vec::new();
            for disc in volumes {
                for track in disc {
                    let full_id = track.full_id_param(Some(&album_id));
                    jobs.push(TrackJob {
                        track,
                        full_id,
                        total_tracks: Some(total),
                    });
                }
            }
            Ok(jobs)
        }
        ResourceLink::PlaylistByUuid { uuid } => {
            let playlist = endpoints::fetch_playlist_by_uuid(client, &uuid).await?;
            jobs_from_playlist_refs(client, &playlist).await
        }
        ResourceLink::UserPlaylist { login, kind } => {
            let playlist = endpoints::fetch_user_playlist(client, &login, &kind).await?;
            jobs_from_playlist_refs(client, &playlist).await
        }
    }
}

async fn jobs_from_playlist_refs(
    client: &ApiClient,
    playlist: &crate::api::models::Playlist,
) -> Result<Vec<TrackJob>, PipelineError> {
    let full_ids: Vec<String> = playlist
        .tracks
        .iter()
        .map(crate::api::models::TrackRef::full_id_param)
        .collect();
    if full_ids.is_empty() {
        return Ok(Vec::new());
    }
    let tracks = endpoints::fetch_tracks(client, &full_ids).await?;
    let total = tracks.len() as u32;
    Ok(tracks
        .into_iter()
        .map(|track| {
            let full_id = track.full_id_param(None);
            TrackJob {
                track,
                full_id,
                total_tracks: Some(total),
            }
        })
        .collect())
}

/// Downloads a single track and returns the path to the finished file.
///
/// # Errors
/// Returns an error at any stage: get-file-info, download, decryption, demux, file write, or tagging.
pub async fn download_track(
    client: Arc<ApiClient>,
    http: reqwest::Client,
    job: TrackJob,
    settings: Settings,
) -> Result<DownloadOutcome, PipelineError> {
    let quality = settings.quality;
    let download_dir = settings
        .download_dir
        .clone()
        .ok_or(PipelineError::NoDownloadDir)?;

    let info = endpoints::get_file_info(&client, &job.full_id, quality).await?;
    let url = info.urls.first().ok_or(PipelineError::NoDownloadUrl)?;
    let codec = ResponseCodec::parse(&info.codec)
        .ok_or_else(|| PipelineError::UnknownCodec(info.codec.clone()))?;

    let mut bytes = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec();

    decrypt::decrypt_in_place(&mut bytes, &info.key)?;

    if codec.needs_flac_demux() {
        bytes = flac_mp4::demux_to_flac(&bytes)?;
    }

    let mut meta = TrackMetadata::from_track(&job.track, job.total_tracks);
    if let Some(cover_uri) = job.track.cover_uri() {
        match fetch_cover(&http, cover_uri, settings.cover_size).await {
            Ok(cover) => meta.cover_bytes = Some(cover),
            Err(err) => tracing::warn!(%err, "failed to download cover art"),
        }
    }

    let path = naming::build_path(
        &download_dir,
        &meta,
        settings.smart_library_organization,
        settings.album_year_in_folder,
        settings.track_indexing,
        codec.file_extension(),
    );

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(&path, &bytes).await?;
    let tag_path = path.clone();
    let tag_meta = meta.clone();
    tokio::task::spawn_blocking(move || tags::write_tags(&tag_path, codec, &tag_meta))
        .await
        .expect("tagging task must not panic")?;

    if settings.smart_library_organization {
        let album_dir =
            naming::build_album_dir(&download_dir, &meta, settings.album_year_in_folder);
        let artist_dir = naming::build_artist_dir(&download_dir, &meta);

        if settings.download_album_cover {
            if let Some(cover_uri) = job.track.cover_uri() {
                save_folder_image_if_absent(
                    &http,
                    cover_uri,
                    settings.cover_size,
                    &album_dir.join("cover.jpg"),
                )
                .await;
            }
        }

        if settings.download_artist_image {
            if let Some(artist) = job.track.artists.first() {
                if let Some(cover_uri) = artist.cover_uri() {
                    save_folder_image_if_absent(
                        &http,
                        cover_uri,
                        settings.cover_size,
                        &artist_dir.join("artist.jpg"),
                    )
                    .await;
                }
            }
        }
    }

    Ok(DownloadOutcome {
        path,
        codec,
        bitrate: info.bitrate,
    })
}

/// Downloads a folder image (artist photo or album cover) only when the destination file does
/// not yet exist — prevents redundant CDN requests when multiple tracks share the same folder.
async fn save_folder_image_if_absent(
    http: &reqwest::Client,
    cover_uri: &str,
    size: CoverSize,
    dest: &std::path::Path,
) {
    match tokio::fs::try_exists(dest).await {
        Ok(true) => return,
        Ok(false) => {}
        Err(err) => {
            tracing::warn!(%err, path = %dest.display(), "could not stat folder image path");
            return;
        }
    }
    match fetch_cover(http, cover_uri, size).await {
        Ok(bytes) => {
            if let Err(err) = tokio::fs::write(dest, &bytes).await {
                tracing::warn!(%err, path = %dest.display(), "failed to write folder image");
            }
        }
        Err(err) => tracing::warn!(%err, "failed to download folder image"),
    }
}

async fn fetch_cover(
    http: &reqwest::Client,
    cover_uri: &str,
    size: CoverSize,
) -> Result<Vec<u8>, PipelineError> {
    let px = size.pixels();
    let sized = cover_uri.replace("%%", &format!("{px}x{px}"));
    let url = if sized.starts_with("http") {
        sized
    } else {
        format!("https://{sized}")
    };
    let bytes = http
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(bytes.to_vec())
}
