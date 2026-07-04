//! Yandex Music API data models: `Track`, `Artist`, `Album`, `Playlist`, and related types.
//!
//! Track IDs are strings, album/artist IDs are numbers, and keys use `camelCase`.
//! Some fields are deserialized for schema completeness but are not yet read in code.
#![allow(dead_code)]

use serde::{Deserialize, Deserializer};

/// Accepts either a JSON string or a number and normalizes to `String`, because Yandex Music
/// uses both representations for identifiers across different endpoints.
fn id_as_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Str(String),
        Num(i64),
    }
    Ok(match Repr::deserialize(deserializer)? {
        Repr::Str(s) => s,
        Repr::Num(n) => n.to_string(),
    })
}

fn opt_id_as_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Str(String),
        Num(i64),
    }
    Ok(
        Option::<Repr>::deserialize(deserializer)?.map(|repr| match repr {
            Repr::Str(s) => s,
            Repr::Num(n) => n.to_string(),
        }),
    )
}

/// Record label name. Most responses return a `{id, name, ...}` object, but some endpoints
/// (e.g. search results) return labels as plain strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelName(pub String);

impl<'de> Deserialize<'de> for LabelName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Named { name: String },
            Plain(String),
        }
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Named { name } => Self(name),
            Repr::Plain(name) => Self(name),
        })
    }
}

/// A musical artist.
#[derive(Debug, Clone, Deserialize)]
pub struct Artist {
    #[serde(deserialize_with = "id_as_string")]
    pub id: String,
    pub name: String,
}

/// Position of a track within an album (disc number + track index).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct TrackPosition {
    pub volume: u32,
    pub index: u32,
}

/// An album. The `volumes` field is populated only when the album is fetched in full
/// (`GET /albums/{id}/with-tracks`); when embedded in a track response `track_position`
/// is set instead.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    #[serde(deserialize_with = "id_as_string")]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub cover_uri: Option<String>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub labels: Vec<LabelName>,
    #[serde(default)]
    pub track_position: Option<TrackPosition>,
    #[serde(default)]
    pub track_count: Option<u32>,
    /// Tracks grouped by disc (disc = index + 1). Populated only in `with-tracks` responses.
    #[serde(default)]
    pub volumes: Option<Vec<Vec<Track>>>,
}

impl Album {
    /// Release year: from `year` if present, otherwise parsed from the first four chars of `release_date`.
    #[must_use]
    pub fn resolved_year(&self) -> Option<i32> {
        self.year
            .or_else(|| self.release_date.as_deref()?.get(0..4)?.parse().ok())
    }

    /// Number of discs (length of `volumes`), minimum 1.
    #[must_use]
    pub fn disc_count(&self) -> u32 {
        self.volumes.as_ref().map_or(1, |v| v.len().max(1) as u32)
    }
}

/// A track with full metadata (response from `POST /tracks` or elements of album `volumes`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    #[serde(deserialize_with = "id_as_string")]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(default)]
    pub cover_uri: Option<String>,
    #[serde(default)]
    pub content_warning: Option<String>,
    #[serde(default)]
    pub available: Option<bool>,
}

impl Track {
    /// Full track title including version, e.g. `"The Things You Say (Rework)"`.
    #[must_use]
    pub fn full_title(&self) -> String {
        let title = self.title.as_deref().unwrap_or("Unknown Track");
        match &self.version {
            Some(version) if !version.is_empty() => format!("{title} ({version})"),
            _ => title.to_owned(),
        }
    }

    /// Artist names in the order returned by the API.
    #[must_use]
    pub fn artist_names(&self) -> Vec<String> {
        self.artists.iter().map(|a| a.name.clone()).collect()
    }

    /// Primary album (the first one in the list) — source of cover art, year, genre, and position.
    #[must_use]
    pub fn primary_album(&self) -> Option<&Album> {
        self.albums.first()
    }

    /// Cover URI with a `%%` size placeholder, taken from the track or its primary album.
    #[must_use]
    pub fn cover_uri(&self) -> Option<&str> {
        self.cover_uri.as_deref().or_else(|| {
            self.primary_album()
                .and_then(|album| album.cover_uri.as_deref())
        })
    }

    /// Composite `id:albumId` string expected by the `trackIds` field of `POST /tracks`.
    #[must_use]
    pub fn full_id_param(&self, album_id: Option<&str>) -> String {
        match album_id.or_else(|| self.primary_album().map(|a| a.id.as_str())) {
            Some(album_id) => format!("{}:{album_id}", self.id),
            None => self.id.clone(),
        }
    }

    /// Returns `true` if the track has an explicit content warning.
    #[must_use]
    pub fn is_explicit(&self) -> bool {
        self.content_warning.as_deref() == Some("explicit")
    }
}

/// Lightweight track reference inside a playlist (`{id, albumId}`), used for batch metadata
/// requests via `POST /tracks`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRef {
    #[serde(deserialize_with = "id_as_string")]
    pub id: String,
    #[serde(default, deserialize_with = "opt_id_as_string")]
    pub album_id: Option<String>,
}

impl TrackRef {
    /// Composite `id:albumId` string (or just `id` if the album is unknown) for `POST /tracks`.
    #[must_use]
    pub fn full_id_param(&self) -> String {
        match &self.album_id {
            Some(album_id) => format!("{}:{album_id}", self.id),
            None => self.id.clone(),
        }
    }
}

/// Playlist owner.
#[derive(Debug, Clone, Deserialize)]
pub struct PlaylistOwner {
    pub uid: u64,
    #[serde(default)]
    pub login: Option<String>,
}

/// A playlist (response from `GET /playlist/{uuid}` or `GET /users/{uid}/playlists/{kind}`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub uid: u64,
    pub kind: i64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub track_count: Option<u32>,
    #[serde(default)]
    pub owner: Option<PlaylistOwner>,
    #[serde(default)]
    pub tracks: Vec<TrackRef>,
}

/// Response from `GET /get-file-info` or an element of `downloadInfos` from
/// `GET /get-file-info/batch`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadInfo {
    #[serde(default, deserialize_with = "opt_id_as_string")]
    pub track_id: Option<String>,
    pub codec: String,
    pub bitrate: u32,
    pub transport: String,
    /// AES-128 key in hex, used to decrypt the `encraw` transport stream.
    pub key: String,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GetFileInfoResponse {
    #[serde(rename = "downloadInfo")]
    pub download_info: DownloadInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GetFileInfoBatchResponse {
    #[serde(rename = "downloadInfos")]
    pub download_infos: Vec<DownloadInfo>,
}
