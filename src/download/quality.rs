//! Mapping of user-selected quality to the `quality`/`codecs` parameters of `get-file-info`.

use serde::{Deserialize, Serialize};

/// Download quality/format selected by the user in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Quality {
    Flac,
    M4aAac,
    #[default]
    Mp3_320,
    Mp3_192,
}

impl Quality {
    /// Value of the `quality` parameter in `get-file-info` requests.
    #[must_use]
    pub const fn api_quality(self) -> &'static str {
        match self {
            Self::Flac => "lossless",
            Self::M4aAac => "hq",
            Self::Mp3_320 => "hq",
            Self::Mp3_192 => "nq",
        }
    }

    /// Ordered list of codecs sent in the `codecs` parameter (first = preferred).
    ///
    /// FLAC includes fallback codecs so a response is always returned even when lossless
    /// is unavailable for a specific track. Other qualities request only the exact codec
    /// to keep the result predictable.
    #[must_use]
    pub const fn api_codecs(self) -> &'static [&'static str] {
        match self {
            Self::Flac => &["flac", "flac-mp4", "aac-mp4", "mp3"],
            Self::M4aAac => &["aac-mp4", "he-aac-mp4"],
            Self::Mp3_320 | Self::Mp3_192 => &["mp3"],
        }
    }

    /// Default file extension for this quality setting (before the actual response codec is known).
    #[must_use]
    #[allow(dead_code)]
    pub const fn default_extension(self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::M4aAac => "m4a",
            Self::Mp3_320 | Self::Mp3_192 => "mp3",
        }
    }

    /// Human-readable label used in the settings UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Flac => "FLAC (lossless)",
            Self::M4aAac => "M4A (AAC)",
            Self::Mp3_320 => "MP3 320 kbps",
            Self::Mp3_192 => "MP3 192 kbps",
        }
    }

    /// All available quality options (used to populate the settings dropdown).
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Flac, Self::M4aAac, Self::Mp3_320, Self::Mp3_192]
    }
}

/// The codec actually returned in the `get-file-info` response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseCodec {
    Flac,
    FlacMp4,
    Mp3,
    Aac,
    HeAac,
    AacMp4,
    HeAacMp4,
}

impl ResponseCodec {
    /// Parses the string value of the `codec` field in the API response.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "flac" => Self::Flac,
            "flac-mp4" => Self::FlacMp4,
            "mp3" => Self::Mp3,
            "aac" => Self::Aac,
            "he-aac" => Self::HeAac,
            "aac-mp4" => Self::AacMp4,
            "he-aac-mp4" => Self::HeAacMp4,
            _ => return None,
        })
    }

    /// Returns `true` if the codec is delivered in an MP4 container (`...-mp4`).
    #[must_use]
    #[allow(dead_code)]
    pub const fn is_mp4_container(self) -> bool {
        matches!(self, Self::FlacMp4 | Self::AacMp4 | Self::HeAacMp4)
    }

    /// File extension for the finished audio file.
    #[must_use]
    pub const fn file_extension(self) -> &'static str {
        match self {
            Self::Flac | Self::FlacMp4 => "flac",
            Self::Mp3 => "mp3",
            Self::Aac | Self::HeAac | Self::AacMp4 | Self::HeAacMp4 => "m4a",
        }
    }

    /// Returns `true` if the codec requires demuxing from its MP4 container into a raw FLAC file.
    #[must_use]
    pub const fn needs_flac_demux(self) -> bool {
        matches!(self, Self::FlacMp4)
    }
}
