//! Writes `ilst` atoms into M4A/AAC files via the `mp4ameta` crate.

use std::path::Path;

use mp4ameta::{Img, Tag};

use super::{TagError, TrackMetadata};

/// Writes metadata and cover art (`covr`) into an M4A/AAC container.
///
/// # Errors
/// Returns an error if the file is not a valid MP4/M4A or cannot be written.
pub fn write(path: &Path, meta: &TrackMetadata) -> Result<(), TagError> {
    let mut tag = Tag::read_from_path(path)?;

    tag.set_title(&meta.title);
    tag.set_artist(meta.joined_artists());

    if let Some(album) = &meta.album {
        tag.set_album(album);
    }
    if let Some(album_artist) = &meta.album_artist {
        tag.set_album_artist(album_artist);
    }
    if let Some(genre) = &meta.genre {
        tag.set_genre(genre);
    }
    if let Some(year) = meta.year {
        tag.set_year(year.to_string());
    }
    if let Some(track) = meta.track_number {
        match meta.total_tracks {
            Some(total) => tag.set_track(track as u16, total as u16),
            None => tag.set_track_number(track as u16),
        }
    }
    if let Some(disc) = meta.disc_number {
        match meta.total_discs {
            Some(total) => tag.set_disc(disc as u16, total as u16),
            None => tag.set_disc_number(disc as u16),
        }
    }
    if let Some(cover) = &meta.cover_bytes {
        tag.set_artwork(Img::jpeg(cover.clone()));
    }

    tag.write_to_path(path)?;
    Ok(())
}
