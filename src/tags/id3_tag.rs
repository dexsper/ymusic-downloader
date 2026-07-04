//! Writes ID3v2.4 tags into MP3 files via the `id3` crate.

use std::path::Path;

use id3::frame::{Picture, PictureType};
use id3::{Tag, TagLike, Version};

use super::{TagError, TrackMetadata};

/// Writes a full set of ID3v2.4 frames into an MP3 file, including cover art (`APIC`).
///
/// # Errors
/// Returns an error if the file cannot be read or written.
pub fn write(path: &Path, meta: &TrackMetadata) -> Result<(), TagError> {
    let mut tag = Tag::read_from_path(path).unwrap_or_default();

    tag.set_title(meta.title.clone());
    tag.set_artist(meta.joined_artists());

    if let Some(album) = &meta.album {
        tag.set_album(album.clone());
    }
    if let Some(album_artist) = &meta.album_artist {
        tag.set_album_artist(album_artist.clone());
    }
    if let Some(genre) = &meta.genre {
        tag.set_genre(genre.clone());
    }
    if let Some(year) = meta.year {
        tag.set_year(year);
        // TDRC (recording time) is the preferred ID3v2.4 date frame.
        if let Ok(ts) = format!("{year}").parse() {
            tag.set_date_recorded(ts);
        }
    }
    if let Some(track) = meta.track_number {
        tag.set_track(track);
    }
    if let Some(total) = meta.total_tracks {
        tag.set_total_tracks(total);
    }
    if let Some(disc) = meta.disc_number {
        tag.set_disc(disc);
    }
    if let Some(total_discs) = meta.total_discs {
        tag.set_total_discs(total_discs);
    }
    if let Some(label) = &meta.label {
        // TPUB — Publisher / record label.
        tag.set_text("TPUB", label.clone());
    }

    if let Some(cover) = &meta.cover_bytes {
        tag.remove_picture_by_type(PictureType::CoverFront);
        tag.add_frame(Picture {
            mime_type: "image/jpeg".to_owned(),
            picture_type: PictureType::CoverFront,
            description: String::new(),
            data: cover.clone(),
        });
    }

    tag.write_to_path(path, Version::Id3v24)?;
    Ok(())
}
