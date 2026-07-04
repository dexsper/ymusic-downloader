//! HMAC-SHA256 signing for `GET /get-file-info(/batch)` requests.
//!
//! Reverse-engineered from the desktop client (see `requests.txt`; independently confirmed at
//! <https://github.com/MarshalX/yandex-music-api/issues/656>):
//! `sign = base64(HMAC-SHA256(key, ts + trackIds.concat + quality + codecs.concat + transports))`,
//! trimmed by one character (the trailing `=` base64 padding is always present for a
//! 32-byte SHA-256 digest and is dropped by the original client).

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

const SIGN_KEY: &str = "kzqU4XhfCaY6B6JTHODeq5";

type HmacSha256 = Hmac<Sha256>;

/// Computes the `sign` parameter required by `GET /get-file-info` and `GET /get-file-info/batch`.
#[must_use]
pub fn sign_get_file_info(
    ts: i64,
    track_ids: &[&str],
    quality: &str,
    codecs: &[&str],
    transports: &str,
) -> String {
    let mut message = ts.to_string();
    for id in track_ids {
        message.push_str(id);
    }
    message.push_str(quality);
    for codec in codecs {
        message.push_str(codec);
    }
    message.push_str(transports);

    let mut mac = HmacSha256::new_from_slice(SIGN_KEY.as_bytes())
        .expect("HMAC-SHA256 accepts keys of any length");
    mac.update(message.as_bytes());
    let digest = mac.finalize().into_bytes();

    let mut encoded = base64::engine::general_purpose::STANDARD.encode(digest);
    encoded.pop();
    encoded.replace('+', "%2B").replace('/', "%2F")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values taken from a real captured desktop client request (`requests.txt`).
    #[test]
    fn matches_captured_desktop_request() {
        let sign = sign_get_file_info(
            1_783_169_766,
            &["151431535"],
            "nq",
            &["flac", "aac", "he-aac", "mp3"],
            "encraw",
        );
        assert_eq!(sign, "4YlQb1dVsu9NXCDw9smXbaBvp2iYD738ay702aDVTdY");
    }
}
