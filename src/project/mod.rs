//! Project: a folder on disk with a `.ymd-project.json` manifest that stores
//! per-project settings and the set of already-downloaded track IDs.
//!
//! Only truly global state (auth credentials, parallel download limit, recent project list)
//! stays in the application config; everything tied to a library lives here.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::CoverSize;
use crate::download::quality::Quality;

/// Name of the project manifest file placed in the project root.
pub const PROJECT_FILE: &str = ".ymd-project.json";

/// Errors reading or writing a project file.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("(de)serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Per-project download and organization settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    /// Download quality (codec + bitrate preference).
    pub quality: Quality,
    /// Resolution of cover art embedded into audio tags.
    pub cover_size: CoverSize,
    /// Organize downloads as `{Artist}/{Album}/Disc N/{index} - {Title}.ext`.
    pub smart_library_organization: bool,
    /// Append the release year to the album folder name: `Album (Year)`.
    pub album_year_in_folder: bool,
    /// Prepend track index prefixes (`01 - `, `02 - `) to file names.
    pub track_indexing: bool,
    /// Save `cover.jpg` inside each album folder; re-downloaded when the CDN URI changes.
    pub download_album_cover: bool,
    /// Save `artist.jpg` inside each artist folder; re-downloaded when the CDN URI changes.
    pub download_artist_image: bool,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            quality: Quality::default(),
            cover_size: CoverSize::default(),
            smart_library_organization: true,
            album_year_in_folder: false,
            track_indexing: false,
            download_album_cover: false,
            download_artist_image: false,
        }
    }
}

/// On-disk representation of the project manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectFile {
    #[serde(flatten)]
    settings: ProjectSettings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    downloaded_track_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    downloaded_image_uris: Vec<String>,
}

/// A loaded project: root directory + settings + already-downloaded ID set.
#[derive(Debug, Clone)]
pub struct Project {
    /// Root directory; all downloads are placed inside this folder.
    pub root: PathBuf,
    pub settings: ProjectSettings,
    /// Composite track IDs already downloaded; used to skip re-downloads.
    pub downloaded_ids: HashSet<String>,
    /// Raw CDN image URIs already downloaded; used to skip re-downloads while still
    /// fetching updated images when the URI changes (e.g. artist updates their photo).
    pub downloaded_image_uris: HashSet<String>,
}

impl Project {
    /// Opens an existing project at `root`, or initializes a fresh one if no manifest exists.
    ///
    /// When the manifest does not yet exist it is written immediately so the folder is
    /// recognised as a valid project on subsequent runs.
    ///
    /// # Errors
    /// Returns an error if the manifest cannot be read, parsed, or (for new projects) written.
    pub fn open(root: PathBuf) -> Result<Self, ProjectError> {
        let file_path = root.join(PROJECT_FILE);
        let (data, existed): (ProjectFile, bool) = match std::fs::read_to_string(&file_path) {
            Ok(s) => (serde_json::from_str(&s)?, true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (ProjectFile::default(), false),
            Err(e) => return Err(e.into()),
        };

        let project = Self {
            root,
            settings: data.settings,
            downloaded_ids: data.downloaded_track_ids.into_iter().collect(),
            downloaded_image_uris: data.downloaded_image_uris.into_iter().collect(),
        };

        if !existed {
            project.save()?;
        }
        Ok(project)
    }

    /// Returns `true` if `root` already contains a project manifest.
    #[must_use]
    pub fn exists_at(root: &std::path::Path) -> bool {
        root.join(PROJECT_FILE).is_file()
    }

    /// Persists current settings and downloaded IDs to `{root}/.ymd-project.json`.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be created or the file cannot be written.
    pub fn save(&self) -> Result<(), ProjectError> {
        std::fs::create_dir_all(&self.root)?;

        let mut ids: Vec<String> = self.downloaded_ids.iter().cloned().collect();
        ids.sort_unstable();

        let mut uris: Vec<String> = self.downloaded_image_uris.iter().cloned().collect();
        uris.sort_unstable();

        let data = ProjectFile {
            settings: self.settings.clone(),
            downloaded_track_ids: ids,
            downloaded_image_uris: uris,
        };
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(self.root.join(PROJECT_FILE), json)?;
        Ok(())
    }

    /// Human-readable name: the last path component of the root directory.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("—")
    }

    /// Records a downloaded track ID and saves the project manifest.
    pub fn record_downloaded(&mut self, full_id: &str) {
        self.downloaded_ids.insert(full_id.to_owned());
        if let Err(err) = self.save() {
            tracing::warn!(%err, "failed to persist downloaded track ID");
        }
    }

    /// Returns `true` if a folder image with this raw URI has already been downloaded.
    #[must_use]
    pub fn has_image_uri(&self, uri: &str) -> bool {
        self.downloaded_image_uris.contains(uri)
    }

    /// Records a downloaded image URI and saves the project manifest.
    pub fn record_image_uri(&mut self, uri: &str) {
        self.downloaded_image_uris.insert(uri.to_owned());
        if let Err(err) = self.save() {
            tracing::warn!(%err, "failed to persist downloaded image URI");
        }
    }
}

/// Returns a short, human-readable name for a project path.
#[must_use]
pub fn path_display_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("—")
}
