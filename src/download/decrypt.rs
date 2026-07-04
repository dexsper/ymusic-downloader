//! Decryption of the `encraw` transport stream.
//!
//! The file served at `downloadInfo.urls[0]` is encrypted with **AES-128-CTR**:
//! the key is 16 bytes from the `key` field (a 32-character hex string), and the
//! counter (IV) is 16 zero bytes. CTR is a symmetric streaming mode, so decryption
//! and encryption are the same XOR-with-keystream operation applied in place.

use aes::Aes128;
use ctr::cipher::{KeyIvInit, StreamCipher};

/// AES-128 in CTR mode with a 128-bit big-endian counter, matching the desktop client behaviour.
type Aes128Ctr = ctr::Ctr128BE<Aes128>;

/// Errors from decrypting an `encraw` stream.
#[derive(Debug, thiserror::Error)]
pub enum DecryptError {
    #[error("invalid hex decryption key: {0}")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("expected a 16-byte AES-128 key, got {0} bytes")]
    InvalidKeyLength(usize),
}

/// Decrypts `buffer` in place using `key_hex` (32 hex chars → 16 bytes).
///
/// # Errors
/// Returns an error if `key_hex` is not valid hex or does not decode to exactly 16 bytes.
pub fn decrypt_in_place(buffer: &mut [u8], key_hex: &str) -> Result<(), DecryptError> {
    let key = hex::decode(key_hex)?;
    if key.len() != 16 {
        return Err(DecryptError::InvalidKeyLength(key.len()));
    }
    let iv = [0u8; 16];
    let mut cipher =
        Aes128Ctr::new_from_slices(&key, &iv).expect("key and IV lengths are already validated");
    cipher.apply_keystream(buffer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctr_is_symmetric() {
        let key_hex = "000102030405060708090a0b0c0d0e0f";
        let plain = b"The Things You Say (Rework) - lossless FLAC payload".to_vec();

        let mut buffer = plain.clone();
        decrypt_in_place(&mut buffer, key_hex).unwrap();
        assert_ne!(buffer, plain, "CTR must alter the data");

        decrypt_in_place(&mut buffer, key_hex).unwrap();
        assert_eq!(buffer, plain);
    }

    #[test]
    fn rejects_bad_key_length() {
        let mut buffer = vec![0u8; 8];
        assert!(matches!(
            decrypt_in_place(&mut buffer, "00010203"),
            Err(DecryptError::InvalidKeyLength(4))
        ));
    }
}
