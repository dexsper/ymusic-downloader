//! Output path construction: smart library organization and track indexing.
//!
//! When smart organization is enabled the path follows
//! `{root}/{Artist}/{Album (Year)}/Disc N/{index} - {Title}.ext`
//! (the `Disc N` segment is omitted for single-disc releases).
//! When disabled the file is placed flat in the root directory.

use std::path::{Path, PathBuf};

use crate::tags::TrackMetadata;

/// Characters forbidden in Windows file names (and problematic on other platforms too).
const FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

const MAX_COMPONENT_LEN: usize = 150;

/// Strips forbidden filesystem characters, collapses repeated spaces, and trims trailing
/// dots/spaces (disallowed on Windows) to produce a safe file or directory name component.
#[must_use]
pub fn sanitize(component: &str) -> String {
    let mut cleaned: String = component
        .chars()
        .map(|c| {
            if FORBIDDEN.contains(&c) || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    while cleaned.contains("  ") {
        cleaned = cleaned.replace("  ", " ");
    }
    let trimmed = cleaned.trim().trim_end_matches('.').trim();
    let mut result: String = trimmed.chars().take(MAX_COMPONENT_LEN).collect();
    if result.is_empty() {
        result.push('_');
    }
    result
}

/// Formats a track number with leading zeros. The width matches the digit count of
/// `total_tracks` (minimum 2) so alphabetic and numeric sort orders agree.
#[must_use]
pub fn format_index(track_number: u32, total_tracks: Option<u32>) -> String {
    let width = total_tracks
        .map(|t| t.to_string().len().max(2))
        .unwrap_or(2);
    format!("{track_number:0width$}")
}

/// Returns the artist directory under `root` using the same name as [`build_path`].
///
/// Used by the pipeline to place `artist.jpg` without duplicating the sanitization logic.
#[must_use]
pub fn build_artist_dir(root: &Path, meta: &TrackMetadata) -> PathBuf {
    let artist = meta
        .album_artist
        .clone()
        .unwrap_or_else(|| meta.joined_artists());
    root.join(sanitize(&artist))
}

/// Returns the album directory under `root` using the same name as [`build_path`].
///
/// Used by the pipeline to place `cover.jpg` without duplicating the sanitization logic.
#[must_use]
pub fn build_album_dir(root: &Path, meta: &TrackMetadata, album_year_in_folder: bool) -> PathBuf {
    let artist_dir = build_artist_dir(root, meta);
    let album_name = meta.album.as_deref().unwrap_or("Unknown Album");
    let album_dir_name = if album_year_in_folder {
        match meta.year {
            Some(year) => format!("{album_name} ({year})"),
            None => album_name.to_owned(),
        }
    } else {
        album_name.to_owned()
    };
    artist_dir.join(sanitize(&album_dir_name))
}

/// Builds the full output path for a track according to the current organization settings.
///
/// * `root` — download root directory from settings;
/// * `smart_organization` — arrange into `Artist/Album/Disc N` subdirectories;
/// * `album_year_in_folder` — append the release year to the album folder: `Album (Year)`;
/// * `track_indexing` — prepend a numeric index to the file name;
/// * `extension` — final file extension (`mp3`, `flac`, or `m4a`).
#[must_use]
pub fn build_path(
    root: &Path,
    meta: &TrackMetadata,
    smart_organization: bool,
    album_year_in_folder: bool,
    track_indexing: bool,
    extension: &str,
) -> PathBuf {
    let mut path = root.to_path_buf();

    if smart_organization {
        let artist = meta
            .album_artist
            .clone()
            .unwrap_or_else(|| meta.joined_artists());
        path.push(sanitize(&artist));

        let album_name = meta.album.as_deref().unwrap_or("Unknown Album");
        let album_dir = if album_year_in_folder {
            match meta.year {
                Some(year) => format!("{album_name} ({year})"),
                None => album_name.to_owned(),
            }
        } else {
            album_name.to_owned()
        };
        path.push(sanitize(&album_dir));

        if let (Some(disc), Some(total)) = (meta.disc_number, meta.total_discs)
            && total > 1
        {
            path.push(format!("Disc {disc}"));
        }
    }

    let mut file_name = String::new();
    if track_indexing && let Some(track_number) = meta.track_number {
        file_name.push_str(&format_index(track_number, meta.total_tracks));
        file_name.push_str(" - ");
    }
    file_name.push_str(&sanitize(&meta.title));
    file_name.push('.');
    file_name.push_str(extension);

    path.push(file_name);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> TrackMetadata {
        TrackMetadata {
            title: "The Things You Say".to_owned(),
            artists: vec!["Gigi D'Agostino".to_owned()],
            album_artist: Some("Gigi D'Agostino".to_owned()),
            album: Some("L'Amour Toujours".to_owned()),
            year: Some(1999),
            track_number: Some(3),
            total_tracks: Some(14),
            disc_number: Some(1),
            total_discs: Some(2),
            ..Default::default()
        }
    }

    #[test]
    fn smart_path_with_disc_and_year() {
        let path = build_path(Path::new("/music"), &sample(), true, true, true, "flac");
        let expected = Path::new("/music")
            .join("Gigi D'Agostino")
            .join("L'Amour Toujours (1999)")
            .join("Disc 1")
            .join("03 - The Things You Say.flac");
        assert_eq!(path, expected);
    }

    #[test]
    fn smart_path_without_year_in_folder() {
        let path = build_path(Path::new("/music"), &sample(), true, false, true, "flac");
        let expected = Path::new("/music")
            .join("Gigi D'Agostino")
            .join("L'Amour Toujours")
            .join("Disc 1")
            .join("03 - The Things You Say.flac");
        assert_eq!(path, expected);
    }

    #[test]
    fn flat_path_no_indexing() {
        let path = build_path(Path::new("/music"), &sample(), false, true, false, "mp3");
        assert_eq!(path, Path::new("/music").join("The Things You Say.mp3"));
    }

    #[test]
    fn sanitizes_forbidden_chars() {
        assert_eq!(sanitize("AC/DC: Back?"), "AC DC Back");
    }

    #[test]
    fn single_disc_has_no_disc_folder() {
        let mut meta = sample();
        meta.total_discs = Some(1);
        let path = build_path(Path::new("/music"), &meta, true, true, false, "m4a");
        assert!(!path.to_string_lossy().contains("Disc"));
    }
}
