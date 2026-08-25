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

    /// Returns `true` when this header's version matches `current_version`.
    ///
    /// The canonical "can I load this save?" guard — compare the loaded header
    /// against the application's current schema version constant before
    /// deserialising the payload. `false` means the format changed and a
    /// migration or "incompatible save" error message is needed.
    #[inline]
    pub fn is_compatible(&self, current_version: u32) -> bool {
        self.version == current_version
    }
}

/// Encode `payload` with `header` into a portable byte buffer.
///
/// The result begins with a `MAGIC` marker, followed by the version, a FNV-1a
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
    // `data.len() >= 20` is guaranteed above, so the subtraction cannot
    // underflow. Comparing this way — rather than `data.len() < 20 + payload_len`
    // — avoids overflowing the addition when a hostile/corrupt header declares a
    // `payload_len` near `usize::MAX` on 32-bit targets, which would wrap, slip
    // past the bounds check, and then panic on the inverted slice range below.
    if payload_len > data.len() - 20 {
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
    /// A [`Migrator`] could not convert the save file from its stored version.
    MigrationFailed,
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

    /// A short human-readable description of the error, suitable for logging.
    /// Does not allocate; returns a `'static str`.
    #[inline]
    pub fn message(&self) -> &'static str {
        match self {
            LoadError::TooShort => "save file too short",
            LoadError::BadMagic => "not a save file (bad magic)",
            LoadError::ChecksumMismatch => "save file corrupted (checksum mismatch)",
            LoadError::MigrationFailed => "save file version migration failed",
        }
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
            LoadError::MigrationFailed => {
                write!(f, "save file migration failed: incompatible version")
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// Compute the total byte size of a save file that [`save_bytes`] would produce
/// for a payload of `payload_len` bytes. The header is always 20 bytes
/// (magic 4 + version 4 + checksum 8 + len 4), so the result is
/// `20 + payload_len`. Use this to pre-allocate or budget storage before
/// calling `save_bytes`.
#[inline]
pub fn estimate_save_size(payload_len: usize) -> usize {
    20 + payload_len
}

// ── Schema migration (W4) ────────────────────────────────────────────────────

/// Version migration strategy for save files (W4 in
/// `STRENGTHS_WEAKNESSES.md`).
///
/// Implement this for your game's save format. When [`load_bytes_migrated`]
/// finds a save file whose version differs from [`current_version`], it calls
/// [`migrate`] with the old version and raw payload; return `Ok(new_bytes)` to
/// transform the data, or `Err(LoadError::MigrationFailed)` to signal an
/// unresolvable incompatibility.
///
/// [`current_version`]: Migrator::current_version
/// [`migrate`]: Migrator::migrate
///
/// # Example — single-step v0 → v1 migration
/// ```
/// use izanagi_kit::savefile::{save_bytes, load_bytes_migrated, Migrator, LoadError, SaveHeader};
///
/// struct V1Migrator;
///
/// impl Migrator for V1Migrator {
///     fn current_version(&self) -> u32 { 1 }
///     fn migrate(&self, old: u32, payload: &[u8]) -> Result<Vec<u8>, LoadError> {
///         match old {
///             0 => {
///                 // v0 payload had no length prefix; v1 prepends a u32 LE count.
///                 let mut out = (payload.len() as u32).to_le_bytes().to_vec();
///                 out.extend_from_slice(payload);
///                 Ok(out)
///             }
///             _ => Err(LoadError::MigrationFailed),
///         }
///     }
/// }
///
/// let v0_data = save_bytes(&SaveHeader::new(0), b"hello");
/// let (header, payload) = load_bytes_migrated(&v0_data, &V1Migrator).unwrap();
/// assert_eq!(header.version, 1);
/// // First 4 bytes of payload are now the length of "hello" (5u32 LE).
/// assert_eq!(&payload[4..], b"hello");
/// ```
pub trait Migrator {
    /// The application's current (target) schema version.
    fn current_version(&self) -> u32;

    /// Attempt to transform `payload` from `old_version` to the current format.
    ///
    /// Should be a pure function (no side effects); the caller may call it
    /// multiple times for a chain of version hops. Return
    /// [`LoadError::MigrationFailed`] for versions that cannot be migrated.
    fn migrate(&self, old_version: u32, payload: &[u8]) -> Result<Vec<u8>, LoadError>;
}

/// Load a save file and apply version migration if the stored version differs
/// from `migrator.current_version()`.
///
/// 1. Parses and checksum-validates the raw bytes with [`load_bytes`].
/// 2. If the stored version matches `current_version`, the payload is returned
///    unchanged.
/// 3. Otherwise, [`Migrator::migrate`] is called. On success, the returned
///    header carries `current_version` and the migrated payload is returned.
///
/// All [`LoadError`] variants from `load_bytes` propagate unchanged.
pub fn load_bytes_migrated<M: Migrator>(
    data: &[u8],
    migrator: &M,
) -> Result<(SaveHeader, Vec<u8>), LoadError> {
    let (header, payload) = load_bytes(data)?;
    let current = migrator.current_version();
    if header.version == current {
        return Ok((header, payload.to_vec()));
    }
    let migrated = migrator.migrate(header.version, payload)?;
    Ok((SaveHeader::new(current), migrated))
}

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
    fn test_declared_len_u32_max_is_too_short_not_panic() {
        // A hostile/corrupt header declaring the maximum payload length must
        // fail cleanly with TooShort, never panic. On 32-bit targets the old
        // `20 + payload_len` check overflowed usize, wrapped past the guard, and
        // panicked on the inverted slice range; the subtraction-based check is
        // overflow-safe on every target.
        let mut data = save_bytes(&SaveHeader { version: 1 }, b"payload");
        data[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(load_bytes(&data), Err(LoadError::TooShort));
    }

    #[test]
    fn test_declared_len_exactly_fits_buffer_is_ok() {
        // Boundary: declared len == available bytes must decode (not TooShort).
        let payload = b"exact";
        let data = save_bytes(&SaveHeader { version: 1 }, payload);
        assert_eq!(load_bytes(&data).unwrap().1, payload);
    }

    #[test]
    fn test_declared_len_one_past_buffer_is_too_short() {
        // Boundary: one byte more than present must be rejected.
        let mut data = save_bytes(&SaveHeader { version: 1 }, b"hello");
        let over = (b"hello".len() as u32) + 1;
        data[16..20].copy_from_slice(&over.to_le_bytes());
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

    #[test]
    fn test_estimate_save_size_empty_payload() {
        assert_eq!(estimate_save_size(0), 20);
    }

    #[test]
    fn test_estimate_save_size_matches_actual() {
        let payload = b"hello save";
        let actual = save_bytes(&SaveHeader::new(1), payload).len();
        assert_eq!(estimate_save_size(payload.len()), actual);
    }

    #[test]
    fn test_estimate_save_size_is_additive() {
        for n in [0, 1, 100, 4096] {
            assert_eq!(estimate_save_size(n), 20 + n);
        }
    }

    #[test]
    fn test_load_error_message_is_static() {
        assert!(!LoadError::TooShort.message().is_empty());
        assert!(!LoadError::BadMagic.message().is_empty());
        assert!(!LoadError::ChecksumMismatch.message().is_empty());
    }

    #[test]
    fn test_load_error_message_bad_magic_mentions_file() {
        assert!(LoadError::BadMagic.message().contains("file"));
    }

    #[test]
    fn test_load_error_message_checksum_mentions_corrupt() {
        let msg = LoadError::ChecksumMismatch.message();
        assert!(msg.contains("corrupt") || msg.contains("checksum"));
    }

    #[test]
    fn test_is_compatible_matching_version() {
        let h = SaveHeader::new(3);
        assert!(h.is_compatible(3));
    }

    #[test]
    fn test_is_compatible_mismatched_version() {
        let h = SaveHeader::new(2);
        assert!(!h.is_compatible(3));
    }

    #[test]
    fn test_is_compatible_zero_version() {
        let h = SaveHeader::new(0);
        assert!(h.is_compatible(0));
        assert!(!h.is_compatible(1));
    }

    // --- Migrator / load_bytes_migrated (W4) ---

    /// Trivial migrator: appends "_v1" to the payload for v0 → v1.
    struct AppendMigrator;
    impl Migrator for AppendMigrator {
        fn current_version(&self) -> u32 {
            1
        }
        fn migrate(&self, old: u32, payload: &[u8]) -> Result<Vec<u8>, LoadError> {
            if old == 0 {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"_v1");
                Ok(out)
            } else {
                Err(LoadError::MigrationFailed)
            }
        }
    }

    #[test]
    fn test_migrated_same_version_returns_payload_unchanged() {
        let data = save_bytes(&SaveHeader::new(1), b"hello");
        let (hdr, payload) = load_bytes_migrated(&data, &AppendMigrator).unwrap();
        assert_eq!(hdr.version, 1);
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn test_migrated_old_version_applies_migration() {
        let data = save_bytes(&SaveHeader::new(0), b"hello");
        let (hdr, payload) = load_bytes_migrated(&data, &AppendMigrator).unwrap();
        assert_eq!(hdr.version, 1, "header version bumped to current");
        assert_eq!(payload, b"hello_v1", "payload transformed by migrator");
    }

    #[test]
    fn test_migrated_unknown_version_returns_migration_failed() {
        let data = save_bytes(&SaveHeader::new(99), b"data");
        let err = load_bytes_migrated(&data, &AppendMigrator).unwrap_err();
        assert_eq!(err, LoadError::MigrationFailed);
    }

    #[test]
    fn test_migrated_bad_magic_propagates_error() {
        let err = load_bytes_migrated(&[0u8; 20], &AppendMigrator).unwrap_err();
        assert_eq!(err, LoadError::BadMagic);
    }

    #[test]
    fn test_migrated_checksum_mismatch_propagates_error() {
        let mut data = save_bytes(&SaveHeader::new(0), b"hello");
        *data.last_mut().unwrap() ^= 0xFF; // corrupt last byte
        let err = load_bytes_migrated(&data, &AppendMigrator).unwrap_err();
        assert_eq!(err, LoadError::ChecksumMismatch);
    }

    // --- Backward-compatible binary framing (golden on-disk bytes) ---
    //
    // Every other test in this module is a same-build round-trip: it writes with
    // the current `save_bytes` and reads with the current `load_bytes`, so a
    // change to the wire format (field order, endianness, magic, header size,
    // checksum algorithm) would leave them all green while silently making every
    // existing player save unreadable. These tests pin the *exact on-disk bytes*
    // and decode a *hardcoded* buffer (standing in for a prior build's output) so
    // a format break must be deliberate.

    /// A fixed (version, payload) whose exact encoding is pinned in `GOLDEN_SAVE`.
    const GOLDEN_VERSION: u32 = 2;
    const GOLDEN_PAYLOAD: &[u8] = b"save-data-v2";

    /// The exact bytes a build wrote for `(GOLDEN_VERSION, GOLDEN_PAYLOAD)`:
    /// magic `IZNG` | version 2 LE | FNV-1a checksum LE | len 12 LE | payload.
    /// A diff here means the on-disk save format changed — regenerate **only**
    /// alongside a deliberate format-version bump and a `CHANGELOG` note, never
    /// to "make the test pass". Regenerate via `print_golden_save` (ignored).
    const GOLDEN_SAVE: &[u8] = &[
        0x49, 0x5a, 0x4e, 0x47, 0x02, 0x00, 0x00, 0x00, 0x1e, 0xc2, 0x8c, 0x05, //
        0x46, 0x90, 0x6f, 0xb1, 0x0c, 0x00, 0x00, 0x00, 0x73, 0x61, 0x76, 0x65, //
        0x2d, 0x64, 0x61, 0x74, 0x61, 0x2d, 0x76, 0x32,
    ];

    #[test]
    fn test_save_bytes_matches_golden_encoding() {
        // Pins the *encoder*: any framing change flips these bytes.
        let data = save_bytes(&SaveHeader::new(GOLDEN_VERSION), GOLDEN_PAYLOAD);
        assert_eq!(
            data.as_slice(),
            GOLDEN_SAVE,
            "save framing changed — existing saves would break. Regenerate \
             GOLDEN_SAVE only with a deliberate format-version bump."
        );
    }

    #[test]
    fn test_load_bytes_decodes_golden_from_prior_build() {
        // Pins *backward compatibility*: a build must read the EXACT bytes a
        // prior build wrote, not merely bytes it just produced itself.
        let (h, p) = load_bytes(GOLDEN_SAVE).expect("golden save must still load");
        assert_eq!(h.version, GOLDEN_VERSION);
        assert_eq!(p, GOLDEN_PAYLOAD);
    }

    #[test]
    fn test_golden_layout_offsets() {
        // Locks the documented field layout: magic[4] ver[4LE] cksum[8LE] len[4LE].
        assert_eq!(&GOLDEN_SAVE[0..4], b"IZNG", "magic offset/value");
        assert_eq!(
            u32::from_le_bytes(GOLDEN_SAVE[4..8].try_into().unwrap()),
            GOLDEN_VERSION,
            "version offset/endianness"
        );
        assert_eq!(
            u32::from_le_bytes(GOLDEN_SAVE[16..20].try_into().unwrap()) as usize,
            GOLDEN_PAYLOAD.len(),
            "length offset/endianness"
        );
        assert_eq!(&GOLDEN_SAVE[20..], GOLDEN_PAYLOAD, "payload offset");
        assert_eq!(
            GOLDEN_SAVE.len(),
            20 + GOLDEN_PAYLOAD.len(),
            "header is 20 bytes"
        );
    }

    /// Prints the current encoding of the golden fixture for pasting into
    /// `GOLDEN_SAVE`. Ignored by default; run only when intentionally bumping
    /// the on-disk format version.
    #[test]
    #[ignore]
    fn print_golden_save() {
        let data = save_bytes(&SaveHeader::new(GOLDEN_VERSION), GOLDEN_PAYLOAD);
        print!("const GOLDEN_SAVE: &[u8] = &[");
        for (i, b) in data.iter().enumerate() {
            if i % 12 == 0 {
                print!("\n    ");
            }
            print!("0x{b:02x}, ");
        }
        println!("\n];");
    }
}
