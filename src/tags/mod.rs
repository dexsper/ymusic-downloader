//! Unified track metadata and tag writing for ID3v2 (mp3), Vorbis Comments (flac), and ilst (m4a).

pub mod flac_tag;
pub mod id3_tag;
pub mod mp4_tag;

use std::path::Path;

use crate::api::models::{Album, Track};
use crate::download::quality::ResponseCodec;

/// Errors from writing tags into an audio file.
#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("ID3 tag write error: {0}")]
    Id3(#[from] id3::Error),
    #[error("FLAC tag write error: {0}")]
    Flac(#[from] metaflac::Error),
    #[error("MP4 tag write error: {0}")]
    Mp4(#[from] mp4ameta::Error),
}

/// Complete set of track metadata populated from the `POST /tracks` response.
#[derive(Debug, Clone, Default)]
pub struct TrackMetadata {
    pub title: String,
    pub artists: Vec<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub label: Option<String>,
    /// Cover art bytes (JPEG) at the configured resolution.
    pub cover_bytes: Option<Vec<u8>>,
}

impl TrackMetadata {
    /// Builds metadata from a full [`Track`] response. `total_tracks_override` is used when
    /// downloading from a playlist and the per-album count is not meaningful.
    #[must_use]
    pub fn from_track(track: &Track, total_tracks_override: Option<u32>) -> Self {
        let album = track.primary_album();
        let position = album.and_then(|a| a.track_position);

        Self {
            title: track.full_title(),
            artists: track.artist_names(),
            album_artist: album
                .and_then(|a| a.artists.first().map(|artist| artist.name.clone()))
                .or_else(|| track.artists.first().map(|a| a.name.clone())),
            album: album.and_then(|a| a.title.clone()),
            year: album.and_then(Album::resolved_year),
            genre: album.and_then(|a| a.genre.clone()),
            disc_number: position.map(|p| p.volume),
            total_discs: album.map(Album::disc_count),
            track_number: position.map(|p| p.index),
            total_tracks: total_tracks_override.or_else(|| album.and_then(|a| a.track_count)),
            label: album.and_then(|a| a.labels.first().map(|l| l.0.clone())),
            cover_bytes: None,
        }
    }

    /// Returns all artist names joined by `", "` (for tag formats that store artists as one string).
    #[must_use]
    pub fn joined_artists(&self) -> String {
        if self.artists.is_empty() {
            "Unknown Artist".to_owned()
        } else {
            self.artists.join(", ")
        }
    }
}

/// Writes tags into the file at `path` according to the actual codec of the downloaded track.
///
/// # Errors
/// Returns an error if the format does not support tagging or the write operation fails.
pub fn write_tags(path: &Path, codec: ResponseCodec, meta: &TrackMetadata) -> Result<(), TagError> {
    match codec {
        ResponseCodec::Mp3 => id3_tag::write(path, meta),
        ResponseCodec::Flac | ResponseCodec::FlacMp4 => flac_tag::write(path, meta),
        ResponseCodec::Aac
        | ResponseCodec::HeAac
        | ResponseCodec::AacMp4
        | ResponseCodec::HeAacMp4 => mp4_tag::write(path, meta),
    }
}
