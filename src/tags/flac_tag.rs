//! Writes Vorbis Comments and a `PICTURE` block into FLAC files via the `metaflac` crate.

use std::path::Path;

use metaflac::Tag;
use metaflac::block::PictureType;

use super::{TagError, TrackMetadata};

/// Writes Vorbis Comments and cover art into a FLAC file.
///
/// # Errors
/// Returns an error if the file is not a valid FLAC or cannot be written.
pub fn write(path: &Path, meta: &TrackMetadata) -> Result<(), TagError> {
    let mut tag = Tag::read_from_path(path)?;

    {
        let comments = tag.vorbis_comments_mut();
        comments.set_title(vec![meta.title.clone()]);
        comments.set_artist(meta.artists.clone());

        if let Some(album) = &meta.album {
            comments.set_album(vec![album.clone()]);
        }
        if let Some(album_artist) = &meta.album_artist {
            comments.set_album_artist(vec![album_artist.clone()]);
        }
        if let Some(genre) = &meta.genre {
            comments.set_genre(vec![genre.clone()]);
        }
        if let Some(year) = meta.year {
            comments.set("DATE", vec![year.to_string()]);
        }
        if let Some(track) = meta.track_number {
            comments.set_track(track);
        }
        if let Some(total) = meta.total_tracks {
            comments.set("TOTALTRACKS", vec![total.to_string()]);
            comments.set("TRACKTOTAL", vec![total.to_string()]);
        }
        if let Some(disc) = meta.disc_number {
            comments.set("DISCNUMBER", vec![disc.to_string()]);
        }
        if let Some(total_discs) = meta.total_discs {
            comments.set("TOTALDISCS", vec![total_discs.to_string()]);
            comments.set("DISCTOTAL", vec![total_discs.to_string()]);
        }
        if let Some(label) = &meta.label {
            comments.set("ORGANIZATION", vec![label.clone()]);
            comments.set("LABEL", vec![label.clone()]);
        }
    }

    if let Some(cover) = &meta.cover_bytes {
        tag.remove_picture_type(PictureType::CoverFront);
        tag.add_picture("image/jpeg", PictureType::CoverFront, cover.clone());
    }

    tag.save()?;
    Ok(())
}
