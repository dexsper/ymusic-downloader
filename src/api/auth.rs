//! OAuth Device Code Flow, manual token entry, and account verification via `/account/about`.
//!
//! `client_id`/`client_secret` are the public credentials of the official Yandex Music
//! Android app, used throughout the `yandex-music-api` ecosystem.

use std::time::Duration;

use serde::Deserialize;

use super::client::ApiClient;

const CLIENT_ID: &str = "23cabbbdc6cd418abb4b39c32c41195d";
const CLIENT_SECRET: &str = "53bc75238f0c4d08a118e51fe9203300";
const DEVICE_NAME: &str = "Yandex Music Downloader";
const OAUTH_BASE_URL: &str = "https://oauth.yandex.ru";

/// Authorization errors: network failures, response parsing, and non-trivial OAuth outcomes.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("authorization confirmation timed out")]
    Expired,
    #[error("authorization was denied by the user")]
    Denied,
    #[error("token is invalid or has expired")]
    InvalidToken,
    #[error("authorization server returned an unexpected error: {0}")]
    Unexpected(String),
}

/// Data returned when initiating the Device Code Flow — must be presented to the user.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: String,
}

/// User account information returned by `GET /account/about`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AccountInfo {
    pub uid: u64,
    pub login: Option<String>,
    pub public_name: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub has_plus: bool,
    #[serde(default)]
    pub has_music_subscription: bool,
    #[serde(default)]
    pub service_available: bool,
    #[serde(default)]
    pub avatar_id: Option<String>,
}

impl AccountInfo {
    /// Returns `true` if the account has an active paid Yandex Music subscription.
    #[must_use]
    pub const fn has_active_subscription(&self) -> bool {
        self.has_plus || self.has_music_subscription
    }

    /// Returns the avatar URL for the given size string (e.g. `islands-75`, `islands-200`).
    #[must_use]
    pub fn get_avatar_url(&self, size: &str) -> Option<String> {
        self.avatar_id
            .as_ref()
            .map(|id| format!("https://avatars.yandex.net/get-yapic/{id}/{size}"))
    }
}

/// Initiates Device Code Flow by requesting a user confirmation code from Yandex.
///
/// # Errors
/// Returns an error on network failure or a malformed server response.
pub async fn request_device_code(http: &reqwest::Client) -> Result<DeviceCodeResponse, AuthError> {
    let device_id = uuid::Uuid::new_v4().to_string();
    let response = http
        .post(format!("{OAUTH_BASE_URL}/device/code"))
        .form(&[
            ("client_id", CLIENT_ID),
            ("device_id", device_id.as_str()),
            ("device_name", DEVICE_NAME),
        ])
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}

/// Polls the authorization server once. Returns `Ok(None)` while the user has not yet
/// confirmed (`authorization_pending` / `slow_down`); the caller controls the retry interval
/// (see [`wait_for_device_token`]).
///
/// # Errors
/// Returns an error on network failure, code expiry, or user denial.
pub async fn poll_device_token(
    http: &reqwest::Client,
    device_code: &str,
) -> Result<Option<String>, AuthError> {
    let response = http
        .post(format!("{OAUTH_BASE_URL}/token"))
        .form(&[
            ("grant_type", "device_code"),
            ("code", device_code),
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
        ])
        .send()
        .await?;

    if response.status().is_success() {
        let token: TokenResponse = response.json().await?;
        return Ok(Some(token.access_token));
    }

    let Ok(body) = response.json::<OAuthErrorResponse>().await else {
        return Ok(None);
    };
    match body.error.as_str() {
        "authorization_pending" | "slow_down" => Ok(None),
        "expired_token" => Err(AuthError::Expired),
        "access_denied" => Err(AuthError::Denied),
        other => Err(AuthError::Unexpected(other.to_owned())),
    }
}

/// Blocks the current async task until the user confirms login, polling at the recommended
/// interval until `expires_in` elapses.
///
/// # Errors
/// Returns an error if the wait times out, the user denies access, or a network/protocol
/// error occurs.
pub async fn wait_for_device_token(
    http: &reqwest::Client,
    device: &DeviceCodeResponse,
) -> Result<String, AuthError> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(device.expires_in);
    let mut interval = tokio::time::interval(Duration::from_secs(device.interval.max(1)));
    interval.tick().await;

    loop {
        interval.tick().await;
        if tokio::time::Instant::now() >= deadline {
            return Err(AuthError::Expired);
        }
        if let Some(token) = poll_device_token(http, &device.device_code).await? {
            return Ok(token);
        }
    }
}

/// Validates the token and retrieves basic account information.
///
/// # Errors
/// Returns [`AuthError::InvalidToken`] if the token is missing or invalid, or a network/parse
/// error otherwise.
pub async fn fetch_account_info(client: &ApiClient) -> Result<AccountInfo, AuthError> {
    let response = client.get("/account/about").send().await?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(AuthError::InvalidToken);
    }
    let response = response.error_for_status()?;
    Ok(response.json().await?)
}
