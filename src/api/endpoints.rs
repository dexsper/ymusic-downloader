//! Requests to specific Yandex Music API endpoints: track/album/playlist metadata
//! and download file information (`get-file-info`).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::download::quality::Quality;

use super::client::{ApiClient, ClientError};
use super::models::{Album, DownloadInfo, GetFileInfoResponse, Playlist, Track};
use super::sign::sign_get_file_info;

/// Transport used by the desktop client: AES-128-CTR encrypted stream.
const TRANSPORT: &str = "encraw";

/// Errors from API endpoint calls.
#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP client error: {0}")]
    Client(#[from] ClientError),
    #[error("failed to read the current time")]
    Clock,
}

fn unix_timestamp() -> Result<i64, EndpointError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .map_err(|_| EndpointError::Clock)
}

/// Fetches full track metadata by composite identifiers (`id` or `id:albumId`),
/// using `POST /tracks` with `multipart/form-data`.
///
/// # Errors
/// Returns an error on network failure or a malformed API response.
pub async fn fetch_tracks(
    client: &ApiClient,
    track_full_ids: &[String],
) -> Result<Vec<Track>, EndpointError> {
    let form = reqwest::multipart::Form::new()
        .text("trackIds", track_full_ids.join(","))
        .text("removeDuplicates", "false");

    let response = client
        .post("/tracks")
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}

/// Fetches a complete album including all discs and tracks.
///
/// # Errors
/// Returns an error on network failure or a malformed API response.
pub async fn fetch_album_with_tracks(
    client: &ApiClient,
    album_id: &str,
) -> Result<Album, EndpointError> {
    let response = client
        .get(&format!("/albums/{album_id}/with-tracks"))
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

/// Fetches a playlist by its public share UUID (`music.yandex.ru/playlist/{uuid}`).
///
/// # Errors
/// Returns an error on network failure or a malformed API response.
pub async fn fetch_playlist_by_uuid(
    client: &ApiClient,
    uuid: &str,
) -> Result<Playlist, EndpointError> {
    let response = client
        .get(&format!(
            "/playlist/{uuid}?resumeStream=false&richTracks=false"
        ))
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

/// Fetches a specific user's playlist by kind number. `user` can be a login name or numeric UID.
///
/// # Errors
/// Returns an error on network failure or a malformed API response.
pub async fn fetch_user_playlist(
    client: &ApiClient,
    user: &str,
    kind: &str,
) -> Result<Playlist, EndpointError> {
    let response = client
        .get(&format!(
            "/users/{user}/playlists/{kind}?resumeStream=false&trackMetaType=music&richTracks=false"
        ))
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

/// Requests download file information for a single track at the selected quality.
///
/// # Errors
/// Returns an error on network failure, a malformed API response, or a clock read failure.
pub async fn get_file_info(
    client: &ApiClient,
    track_id: &str,
    quality: Quality,
) -> Result<DownloadInfo, EndpointError> {
    let ts = unix_timestamp()?;
    let codecs = quality.api_codecs();
    let quality_param = quality.api_quality();
    let sign = sign_get_file_info(ts, &[track_id], quality_param, codecs, TRANSPORT);
    let codecs_param = codecs.join(",");

    let response = client
        .get(&format!(
            "/get-file-info?ts={ts}&trackId={track_id}&quality={quality_param}&codecs={codecs_param}&transports={TRANSPORT}&sign={sign}"
        ))
        .send()
        .await?
        .error_for_status()?;
    let parsed: GetFileInfoResponse = response.json().await?;

    Ok(parsed.download_info)
}
