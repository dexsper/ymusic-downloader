//! Parses user-supplied Yandex Music URLs (track/album/playlist) into structured values.

use url::Url;

/// A recognized Yandex Music resource link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceLink {
    /// A single track; `album_id` is set when the URL contains both path segments
    /// (`/album/{albumId}/track/{trackId}`).
    Track {
        track_id: String,
        album_id: Option<String>,
    },
    /// A full album including all discs and tracks.
    Album { album_id: String },
    /// A playlist identified by a public share UUID (including `lk.`-prefixed UUIDs).
    PlaylistByUuid { uuid: String },
    /// A specific user's playlist (`/users/{login}/playlists/{kind}`).
    UserPlaylist { login: String, kind: String },
}

/// Errors from parsing a user-supplied URL.
#[derive(Debug, thiserror::Error)]
pub enum UrlParseError {
    #[error("failed to parse URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("URL does not point to Yandex Music: {0}")]
    UnsupportedHost(String),
    #[error("unrecognized Yandex Music URL format")]
    UnrecognizedPath,
}

const SUPPORTED_HOSTS: &[&str] = &[
    "music.yandex.ru",
    "music.yandex.com",
    "music.yandex.by",
    "music.yandex.kz",
    "music.yandex.uz",
];

/// Parses a `music.yandex.ru/...` URL into a [`ResourceLink`].
/// The scheme (`https://`) may be omitted and will be prepended automatically.
///
/// # Errors
/// Returns an error if the string is not a valid URL, the host is not a Yandex Music domain,
/// or the path does not match any known URL format.
pub fn parse(input: &str) -> Result<ResourceLink, UrlParseError> {
    let trimmed = input.trim();
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };
    let url = Url::parse(&with_scheme)?;

    let host = url.host_str().unwrap_or_default();
    if !SUPPORTED_HOSTS
        .iter()
        .any(|supported| host.eq_ignore_ascii_case(supported))
    {
        return Err(UrlParseError::UnsupportedHost(host.to_owned()));
    }

    let segments: Vec<&str> = url
        .path_segments()
        .map(Iterator::collect)
        .unwrap_or_default();

    match segments.as_slice() {
        ["album", album_id, "track", track_id] => Ok(ResourceLink::Track {
            track_id: (*track_id).to_owned(),
            album_id: Some((*album_id).to_owned()),
        }),
        ["album", album_id] => Ok(ResourceLink::Album {
            album_id: (*album_id).to_owned(),
        }),
        ["track", track_id] => Ok(ResourceLink::Track {
            track_id: (*track_id).to_owned(),
            album_id: None,
        }),
        ["playlist", uuid] | ["playlists", uuid] => Ok(ResourceLink::PlaylistByUuid {
            uuid: (*uuid).to_owned(),
        }),
        ["users", login, "playlists", kind] => Ok(ResourceLink::UserPlaylist {
            login: (*login).to_owned(),
            kind: (*kind).to_owned(),
        }),
        _ => Err(UrlParseError::UnrecognizedPath),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_track_link() {
        assert_eq!(
            parse("https://music.yandex.ru/track/12345").unwrap(),
            ResourceLink::Track {
                track_id: "12345".to_owned(),
                album_id: None
            }
        );
    }

    #[test]
    fn parses_album_track_link() {
        assert_eq!(
            parse("music.yandex.ru/album/999/track/12345").unwrap(),
            ResourceLink::Track {
                track_id: "12345".to_owned(),
                album_id: Some("999".to_owned())
            }
        );
    }

    #[test]
    fn parses_album_link() {
        assert_eq!(
            parse("https://music.yandex.ru/album/999").unwrap(),
            ResourceLink::Album {
                album_id: "999".to_owned()
            }
        );
    }

    #[test]
    fn parses_playlist_share_link() {
        assert_eq!(
            parse("https://music.yandex.ru/playlist/lk.abc-123").unwrap(),
            ResourceLink::PlaylistByUuid {
                uuid: "lk.abc-123".to_owned()
            }
        );
    }

    #[test]
    fn parses_playlists_share_link_plural() {
        assert_eq!(
            parse("https://music.yandex.ru/playlists/lk.d0bdeacd-23fd-42a4-b6c0-28872088528d")
                .unwrap(),
            ResourceLink::PlaylistByUuid {
                uuid: "lk.d0bdeacd-23fd-42a4-b6c0-28872088528d".to_owned()
            }
        );
    }

    #[test]
    fn parses_user_playlist_link() {
        assert_eq!(
            parse("https://music.yandex.ru/users/ivan/playlists/3").unwrap(),
            ResourceLink::UserPlaylist {
                login: "ivan".to_owned(),
                kind: "3".to_owned()
            }
        );
    }

    #[test]
    fn rejects_unsupported_host() {
        assert!(matches!(
            parse("https://example.com/track/1"),
            Err(UrlParseError::UnsupportedHost(_))
        ));
    }
}
