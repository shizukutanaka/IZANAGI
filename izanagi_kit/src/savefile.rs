//! Versioned binary save-file framing (N2 / N3).
//!
//! Provides a minimal container format for game-specific save data:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │ magic[4]  │ version[4LE] │ checksum[8LE] │ len[4LE] │ payload │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! - **magic** = `b"IZNG"` — identifies the file type.
//! - **version** = caller-defined `u32`; increment when the payload schema
//!   changes. `load_bytes` returns it so the caller can reject incompatible
//!   saves (N3 versioning).
//! - **checksum** = FNV-1a 64-bit over the payload bytes — integrity check
//!   against truncation or corruption.
//! - **len** = payload byte count as `u32 LE`.
//! - **payload** = arbitrary caller bytes; the game layer writes its
//!   serialized world state here.
//!
//! This module deliberately does **not** define how game state is serialised
//! into `payload` — that is the caller's responsibility. The framing layer
//! only adds magic, version, and a checksum.

const MAGIC: &[u8; 4] = b"IZNG";

/// Header extracted from a validated save file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveHeader {
    /// Caller-defined format version.
    pub version: u32,
}

impl SaveHeader {
    /// Construct a header with the given version number.
    pub const fn new(version: u32) -> Self {
        SaveHeader { version }
    }
}

/// Encode `payload` with `header` into a portable byte buffer.
///
/// The result begins with [`MAGIC`], followed by the version, a FNV-1a
/// checksum of `payload`, the payload length, and then the payload itself.
pub fn save_bytes(header: &SaveHeader, payload: &[u8]) -> Vec<u8> {
    let checksum = fnv1a(payload);
    let len = payload.len() as u32;
    let mut out = Vec::with_capacity(20 + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&header.version.to_le_bytes());
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Decode and validate a save file buffer.
///
/// Returns `(SaveHeader, payload_slice)` on success.
/// - `payload_slice` is a sub-slice of `data` — no copying.
/// - `header.version` lets the caller decide whether to accept or reject the
///   save (N3 versioning: compare against the current schema version).
pub fn load_bytes(data: &[u8]) -> Result<(SaveHeader, &[u8]), LoadError> {
    // magic(4) + version(4) + checksum(8) + len(4) = 20 bytes minimum
    if data.len() < 20 {
        return Err(LoadError::TooShort);
    }
    if &data[0..4] != MAGIC {
        return Err(LoadError::BadMagic);
    }
    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let checksum = u64::from_le_bytes([
        data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
    ]);
    let payload_len = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
    if data.len() < 20 + payload_len {
        return Err(LoadError::TooShort);
    }
    let payload = &data[20..20 + payload_len];
    if fnv1a(payload) != checksum {
        return Err(LoadError::ChecksumMismatch);
    }
    Ok((SaveHeader { version }, payload))
}

/// Like [`load_bytes`] but returns an owned `Vec<u8>` payload instead of a
/// borrowed slice. Useful when the caller cannot hold a reference to `data`
/// long enough, or when the payload needs to outlive `data`.
pub fn load_bytes_owned(data: &[u8]) -> Result<(SaveHeader, Vec<u8>), LoadError> {
    let (header, payload) = load_bytes(data)?;
    Ok((header, payload.to_vec()))
}

/// Check that `data` is a structurally valid save file without returning the
/// payload. Equivalent to `load_bytes(data).map(|_| ())`.
///
/// Useful for save-slot browser UI that needs to show a "valid / corrupt"
/// indicator without deserialising the full game state.
#[inline]
pub fn validate_integrity(data: &[u8]) -> Result<(), LoadError> {
    load_bytes(data).map(|_| ())
}

/// Errors returned by [`load_bytes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The buffer is shorter than the minimum header size or the declared
    /// payload length exceeds the remaining bytes.
    TooShort,
    /// The first four bytes do not match the expected magic `b"IZNG"`.
    BadMagic,
    /// The payload's FNV-1a hash does not match the stored checksum.
    ChecksumMismatch,
}

impl LoadError {
    /// Returns `true` when the error indicates a wrong-file condition rather
    /// than data corruption — i.e. the save slot should be treated as empty
    /// rather than damaged. Only [`BadMagic`](Self::BadMagic) qualifies (the
    /// buffer simply is not a save file). [`TooShort`](Self::TooShort) and
    /// [`ChecksumMismatch`](Self::ChecksumMismatch) indicate truncation or
    /// payload corruption and are not recoverable.
    #[inline]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, LoadError::BadMagic)
    }
}

impl core::fmt::Display for LoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadError::TooShort => write!(
                f,
                "save file too short or declared payload length exceeds buffer"
            ),
            LoadError::BadMagic => {
                write!(f, "save file magic mismatch (expected b\"IZNG\")")
            }
            LoadError::ChecksumMismatch => {
                write!(f, "save file checksum mismatch: payload may be corrupted")
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// FNV-1a 64-bit hash over raw bytes. Not exposed; callers use the checksums
/// embedded in save files rather than hashing payloads directly.
fn fnv1a(data: &[u8]) -> u64 {
    const BASIS: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    let mut h = BASIS;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_empty_payload() {
        let header = SaveHeader { version: 1 };
        let data = save_bytes(&header, &[]);
        let (loaded_header, payload) = load_bytes(&data).unwrap();
        assert_eq!(loaded_header.version, 1);
        assert!(payload.is_empty());
    }

    #[test]
    fn test_roundtrip_with_payload() {
        let payload = b"hello world save data";
        let header = SaveHeader { version: 42 };
        let data = save_bytes(&header, payload);
        let (h, p) = load_bytes(&data).unwrap();
        assert_eq!(h.version, 42);
        assert_eq!(p, payload);
    }

    #[test]
    fn test_version_is_preserved() {
        for v in [0u32, 1, 255, u32::MAX] {
            let data = save_bytes(&SaveHeader { version: v }, b"x");
            let (h, _) = load_bytes(&data).unwrap();
            assert_eq!(h.version, v);
        }
    }

    #[test]
    fn test_too_short_returns_error() {
        assert_eq!(load_bytes(&[]), Err(LoadError::TooShort));
        assert_eq!(load_bytes(&[0u8; 19]), Err(LoadError::TooShort));
    }

    #[test]
    fn test_bad_magic_returns_error() {
        let mut data = save_bytes(&SaveHeader { version: 1 }, b"test");
        data[0] = 0xFF; // corrupt magic
        assert_eq!(load_bytes(&data), Err(LoadError::BadMagic));
    }

    #[test]
    fn test_checksum_mismatch_on_payload_corruption() {
        let mut data = save_bytes(&SaveHeader { version: 1 }, b"abcdef");
        // Flip a bit in the payload section (after the 20-byte header).
        *data.last_mut().unwrap() ^= 0xFF;
        assert_eq!(load_bytes(&data), Err(LoadError::ChecksumMismatch));
    }

    #[test]
    fn test_checksum_mismatch_on_header_corruption() {
        let mut data = save_bytes(&SaveHeader { version: 1 }, b"data");
        // Corrupt the stored checksum bytes (bytes 8-15).
        data[8] ^= 0x01;
        assert_eq!(load_bytes(&data), Err(LoadError::ChecksumMismatch));
    }

    #[test]
    fn test_declared_len_beyond_buffer_is_too_short() {
        let mut data = save_bytes(&SaveHeader { version: 1 }, b"payload");
        // Set the declared length to something larger than actual.
        let large_len: u32 = 999;
        data[16..20].copy_from_slice(&large_len.to_le_bytes());
        assert_eq!(load_bytes(&data), Err(LoadError::TooShort));
    }

    #[test]
    fn test_output_starts_with_magic() {
        let data = save_bytes(&SaveHeader { version: 0 }, &[]);
        assert_eq!(&data[0..4], b"IZNG");
    }

    #[test]
    fn test_same_payload_same_bytes() {
        let p = b"deterministic";
        let a = save_bytes(&SaveHeader { version: 7 }, p);
        let b = save_bytes(&SaveHeader { version: 7 }, p);
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_versions_different_bytes() {
        let p = b"same payload";
        let a = save_bytes(&SaveHeader { version: 1 }, p);
        let b = save_bytes(&SaveHeader { version: 2 }, p);
        assert_ne!(a, b);
        // But payloads are still extractable.
        let (ha, _) = load_bytes(&a).unwrap();
        let (hb, _) = load_bytes(&b).unwrap();
        assert_eq!(ha.version, 1);
        assert_eq!(hb.version, 2);
    }

    #[test]
    fn test_large_payload_roundtrip() {
        let payload: Vec<u8> = (0u8..=255).cycle().take(4096).collect();
        let data = save_bytes(&SaveHeader { version: 100 }, &payload);
        let (_, p) = load_bytes(&data).unwrap();
        assert_eq!(p, payload.as_slice());
    }

    #[test]
    fn test_load_error_display_non_empty() {
        assert!(!LoadError::TooShort.to_string().is_empty());
        assert!(!LoadError::BadMagic.to_string().is_empty());
        assert!(!LoadError::ChecksumMismatch.to_string().is_empty());
    }

    #[test]
    fn test_load_error_is_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&LoadError::TooShort);
    }

    #[test]
    fn test_zero_version_is_valid() {
        let data = save_bytes(&SaveHeader { version: 0 }, b"v0 data");
        let (h, _) = load_bytes(&data).unwrap();
        assert_eq!(h.version, 0);
    }

    #[test]
    fn test_new_constructor_sets_version() {
        let h = SaveHeader::new(7);
        assert_eq!(h.version, 7);
        assert_eq!(h, SaveHeader { version: 7 });
    }

    #[test]
    fn test_new_constructor_is_const() {
        const H: SaveHeader = SaveHeader::new(42);
        assert_eq!(H.version, 42);
    }

    #[test]
    fn test_load_bytes_owned_roundtrip() {
        let payload = b"owned payload";
        let data = save_bytes(&SaveHeader::new(3), payload);
        let (h, owned) = load_bytes_owned(&data).unwrap();
        assert_eq!(h.version, 3);
        assert_eq!(owned.as_slice(), payload);
    }

    #[test]
    fn test_load_bytes_owned_error_propagates() {
        assert_eq!(load_bytes_owned(&[]), Err(LoadError::TooShort));
    }

    #[test]
    fn test_load_bytes_owned_payload_is_independent() {
        let payload = b"independent";
        let data = save_bytes(&SaveHeader::new(1), payload);
        let (_h, owned) = load_bytes_owned(&data).unwrap();
        // Drop data — owned should still be valid.
        drop(data);
        assert_eq!(owned.as_slice(), payload);
    }

    #[test]
    fn test_validate_integrity_ok_on_valid_save() {
        let data = save_bytes(&SaveHeader::new(1), b"hello");
        assert!(validate_integrity(&data).is_ok());
    }

    #[test]
    fn test_validate_integrity_rejects_corrupt_payload() {
        let mut data = save_bytes(&SaveHeader::new(1), b"hello");
        *data.last_mut().unwrap() ^= 0xFF;
        assert_eq!(validate_integrity(&data), Err(LoadError::ChecksumMismatch));
    }

    #[test]
    fn test_validate_integrity_rejects_too_short() {
        assert_eq!(validate_integrity(&[0u8; 5]), Err(LoadError::TooShort));
    }

    #[test]
    fn test_bad_magic_is_recoverable() {
        assert!(LoadError::BadMagic.is_recoverable());
    }

    #[test]
    fn test_too_short_is_not_recoverable() {
        assert!(!LoadError::TooShort.is_recoverable());
    }

    #[test]
    fn test_checksum_mismatch_is_not_recoverable() {
        assert!(!LoadError::ChecksumMismatch.is_recoverable());
    }
}
