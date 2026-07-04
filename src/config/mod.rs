//! Application settings: download quality, library organization, authentication, and JSON persistence.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::download::quality::Quality;

/// Errors reading or writing the configuration file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not determine the configuration directory")]
    NoConfigDir,
    #[error("I/O error reading or writing the configuration file: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuration (de)serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Resolution of the cover art embedded into audio tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum CoverSize {
    #[serde(rename = "200")]
    Px200,
    #[serde(rename = "400")]
    Px400,
    #[serde(rename = "600")]
    Px600,
    #[serde(rename = "800")]
    #[default]
    Px800,
    #[serde(rename = "1000")]
    Px1000,
}

impl CoverSize {
    /// Side length of the square cover art in pixels.
    #[must_use]
    pub const fn pixels(self) -> u32 {
        match self {
            Self::Px200 => 200,
            Self::Px400 => 400,
            Self::Px600 => 600,
            Self::Px800 => 800,
            Self::Px1000 => 1000,
        }
    }
}


/// Authorization data persisted between application restarts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthState {
    /// OAuth token (entered manually or obtained via Device Code Flow).
    pub token: Option<String>,
    /// Random UUIDv4 sent in the `X-Yandex-Music-Device` header.
    pub device_uuid: Option<String>,
    /// Pseudo-random `device_id` (hex SHA-256) sent in the `X-Yandex-Music-Device` header.
    pub device_id: Option<String>,
}

/// User-configurable application settings persisted in `%APPDATA%/ymusic-downloader/config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Whether the user has accepted the legal disclaimer.
    pub disclaimer_accepted: bool,
    /// Default download quality.
    pub quality: Quality,
    /// Resolution of cover art embedded into tags.
    pub cover_size: CoverSize,
    /// Organize downloads as `{Artist}/{Album (Year)}/Disc N/{index} - {Title}.ext`.
    pub smart_library_organization: bool,
    /// Prepend track index prefixes (`01 - `, `02 - `) to file names.
    pub track_indexing: bool,
    /// Folder where downloaded files are saved.
    pub download_dir: Option<PathBuf>,
    /// Maximum number of concurrent downloads.
    pub max_parallel_downloads: usize,
    /// Authorization data.
    pub auth: AuthState,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            disclaimer_accepted: false,
            quality: Quality::default(),
            cover_size: CoverSize::default(),
            smart_library_organization: true,
            track_indexing: true,
            download_dir: dirs::audio_dir().or_else(dirs::download_dir),
            max_parallel_downloads: 3,
            auth: AuthState::default(),
        }
    }
}

impl Settings {
    fn config_path() -> Result<PathBuf, ConfigError> {
        let mut dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        dir.push("ymusic-downloader");
        Ok(dir.join("config.json"))
    }

    /// Loads settings from disk, or returns defaults if the file does not yet exist.
    ///
    /// # Errors
    /// Returns an error if the configuration directory cannot be determined, the file cannot
    /// be read (other than not existing), or its contents are malformed.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    fn load_from(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(serde_json::from_str(&contents)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err.into()),
        }
    }

    /// Persists settings to disk, creating the configuration directory if needed.
    ///
    /// # Errors
    /// Returns an error if the directory or file cannot be created, or the data cannot be serialized.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
