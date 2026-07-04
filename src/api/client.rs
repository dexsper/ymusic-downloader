//! `reqwest::Client` wrapper that attaches headers identical to the Yandex Music desktop client
//! (`YandexMusicDesktopAppWindows/5.109.1`). Without these headers (particularly
//! `Origin: music-application://desktop` and `X-Yandex-Music-Client`) the API returns a
//! different, reduced response.

use std::sync::{Arc, RwLock};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::AuthState;

/// Base URL of the Yandex Music API.
pub const BASE_URL: &str = "https://api.music.yandex.net";

/// Desktop client version being impersonated (`app/package.json`, `buildInfo.VERSION`).
pub const CLIENT_VERSION: &str = "5.109.1";

/// OS platform string sent in `X-Yandex-Music-Device`. Always `win32` because signing keys
/// and headers correspond to the Windows desktop build (`YandexMusicDesktopAppWindows`).
pub const DEVICE_OS: &str = "win32";

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) YandexMusic/5.109.1 Chrome/140.0.7339.133 Electron/38.2.2 Safari/537.36";
const ORIGIN: &str = "music-application://desktop";

/// Errors that can occur when building or executing API requests.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to build HTTP client: {0}")]
    Build(reqwest::Error),
}

/// `reqwest::Client` wrapper that attaches desktop-client headers to every request,
/// including authorization and device identification.
#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    token: Arc<RwLock<Option<String>>>,
    device_uuid: String,
    device_id: String,
}

impl ApiClient {
    /// Creates a client, reusing (or generating) `uuid`/`device_id` from persisted settings
    /// so they remain stable across application restarts.
    ///
    /// # Errors
    /// Returns an error if the underlying `reqwest::Client` cannot be built.
    pub fn new(auth: &AuthState) -> Result<Self, ClientError> {
        let http = reqwest::Client::builder().build().map_err(ClientError::Build)?;
        let device_uuid = auth.device_uuid.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let device_id = auth.device_id.clone().unwrap_or_else(generate_device_id);
        Ok(Self {
            http,
            token: Arc::new(RwLock::new(auth.token.clone())),
            device_uuid,
            device_id,
        })
    }

    /// Random UUIDv4 used in the `X-Yandex-Music-Device` header (stable across restarts).
    #[must_use]
    pub fn device_uuid(&self) -> &str {
        &self.device_uuid
    }

    /// Pseudo-random `device_id` used in the `X-Yandex-Music-Device` header.
    #[must_use]
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Replaces the OAuth token used for subsequent requests.
    pub fn set_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.token.write() {
            *guard = token;
        }
    }

    /// Returns the current OAuth token, if set.
    #[must_use]
    pub fn token(&self) -> Option<String> {
        self.token.read().ok().and_then(|guard| guard.clone())
    }

    fn device_header_value(&self) -> String {
        format!(
            "manufacturer=; model=; uuid={}; os={DEVICE_OS}; os_version=; device_id={}; clid=0",
            self.device_uuid, self.device_id
        )
    }

    /// Begins building a `GET` request with the full set of desktop client headers.
    pub fn get(&self, path_and_query: &str) -> reqwest::RequestBuilder {
        self.request(reqwest::Method::GET, path_and_query)
    }

    /// Begins building a `POST` request with the full set of desktop client headers.
    pub fn post(&self, path_and_query: &str) -> reqwest::RequestBuilder {
        self.request(reqwest::Method::POST, path_and_query)
    }

    fn request(&self, method: reqwest::Method, path_and_query: &str) -> reqwest::RequestBuilder {
        let url = format!("{BASE_URL}{path_and_query}");
        let mut builder = self
            .http
            .request(method, url)
            .header("X-Yandex-Music-Client", format!("YandexMusicDesktopAppWindows/{CLIENT_VERSION}"))
            .header("X-Yandex-Music-Device", self.device_header_value())
            .header("X-Yandex-Music-Without-Invocation-Info", "1")
            .header("X-Request-Id", Uuid::new_v4().to_string())
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN)
            .header("Accept-Language", "ru");
        if let Some(token) = self.token() {
            builder = builder.header("Authorization", format!("OAuth {token}"));
        }
        builder
    }
}

/// Generates a `device_id` using the same algorithm as the desktop client (`app/preload.js`):
/// `sha256([hostname, platform, machine, totalmem, mac].join(","))` encoded as hex.
///
/// Values come from real hardware (as in the original client), so the `device_id` is stable
/// across restarts even without caching — but we cache it anyway to avoid sensitivity to
/// network interface / MAC address changes.
fn generate_device_id() -> String {
    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let platform = node_platform();
    let machine = node_machine();
    let totalmem = total_memory_bytes();
    let mac = first_global_mac().unwrap_or_default();

    let data = format!("{hostname},{platform},{machine},{totalmem},{mac}");
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

/// Returns the platform name as Node.js `os.platform()` would report it.
fn node_platform() -> &'static str {
    match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        other => other,
    }
}

/// Returns the machine architecture as Node.js `os.machine()` would report it.
fn node_machine() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "x86" => "i686",
        "aarch64" => "arm64",
        other => other,
    }
}

/// Returns total physical memory in bytes, equivalent to Node.js `os.totalmem()`.
fn total_memory_bytes() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory()
}

/// Returns the first globally-unique MAC address formatted as colon-separated lowercase hex,
/// applying the same filtering rules as the desktop client (`app/preload.js`):
/// skips all-zero MACs and only accepts universally-administered addresses.
fn first_global_mac() -> Option<String> {
    let addr = mac_address::get_mac_address().ok().flatten()?;
    let bytes = addr.bytes();
    if bytes.iter().all(|&b| b == 0) {
        return None;
    }
    let second_nibble = bytes[0] & 0x0f;
    const UNIVERSAL_DIGITS: [u8; 8] = [0x0, 0x1, 0x4, 0x5, 0x8, 0x9, 0xc, 0xd];
    if !UNIVERSAL_DIGITS.contains(&second_nibble) {
        return None;
    }
    Some(
        bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}
