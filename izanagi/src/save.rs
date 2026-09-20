//! Save and load.
//!
//! A tiny versioned binary format for game saves. Four-byte magic,
//! two-byte version, four-byte length, then bytes. No serde dependency —
//! users serialize their own structs into the byte buffer however they
//! like (bincode, postcard, hand-rolled).
//!
//! ```no_run
//! use izanagi::save::Save;
//!
//! let data: Vec<u8> = b"level=3;coins=42".to_vec();
//! Save::write("save.dat", 1, &data).unwrap();
//! let (version, bytes) = Save::read("save.dat").unwrap();
//! ```

use crate::error::{Error, Result};
use std::fs;
use std::path::Path;

const MAGIC: [u8; 4] = *b"IZAN";

/// Simple versioned save file.
pub struct Save;

impl Save {
    /// Write a save at `path` with an explicit schema `version`.
    pub fn write(path: impl AsRef<Path>, version: u16, data: &[u8]) -> Result<()> {
        // Delegates to `encode` so the wire layout exists in exactly one
        // place — the byte-pinned test covers files and memory identically.
        fs::write(path, Self::encode(version, data))?;
        Ok(())
    }

    /// Read a save. Returns `(version, data)`.
    pub fn read(path: impl AsRef<Path>) -> Result<(u16, Vec<u8>)> {
        let bytes = fs::read(path)?;
        Self::parse(&bytes)
    }

    /// Parse an in-memory save buffer (useful for tests and embedded assets).
    pub fn parse(bytes: &[u8]) -> Result<(u16, Vec<u8>)> {
        if bytes.len() < 10 {
            return Err(Error::Config(format!("save too short: {} bytes", bytes.len())));
        }
        if bytes[0..4] != MAGIC {
            return Err(Error::Config("not an IZANAGI save (bad magic)".into()));
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        // Subtraction, not `10 + len`: on 32-bit targets a hostile header
        // declaring `u32::MAX` bytes wraps the addition below the guard and
        // panics on the slice — same fix as izanagi_kit's savefile parser.
        if len > bytes.len() - 10 {
            return Err(Error::Config(format!(
                "save truncated: header claims {len} bytes, have {}",
                bytes.len() - 10
            )));
        }
        Ok((version, bytes[10..10 + len].to_vec()))
    }

    /// Encode to an in-memory buffer (no I/O).
    pub fn encode(version: u16, data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(10 + data.len());
        buf.extend_from_slice(&MAGIC);
        buf.extend_from_slice(&version.to_le_bytes());
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(data);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_in_memory() {
        let payload = b"hello world";
        let enc = Save::encode(7, payload);
        let (v, d) = Save::parse(&enc).unwrap();
        assert_eq!(v, 7);
        assert_eq!(d, payload);
    }

    #[test]
    fn rejects_bad_magic() {
        let bytes = b"XXXX\x00\x00\x00\x00\x00\x00";
        assert!(Save::parse(bytes).is_err());
    }

    #[test]
    fn rejects_hostile_declared_len() {
        // A header declaring u32::MAX payload bytes: on 32-bit `usize` the
        // old `10 + len` comparison overflowed and let the slice panic.
        // The subtraction form cannot wrap.
        let mut enc = Save::encode(1, b"x");
        enc[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Save::parse(&enc).is_err());
    }

    #[test]
    fn rejects_truncated() {
        let mut enc = Save::encode(1, b"abcd");
        enc.pop();
        assert!(Save::parse(&enc).is_err());
    }

    #[test]
    fn empty_payload_roundtrip() {
        let enc = Save::encode(0, &[]);
        let (v, d) = Save::parse(&enc).unwrap();
        assert_eq!(v, 0);
        assert!(d.is_empty());
    }

    #[test]
    fn format_layout_is_byte_exact() {
        // Roundtrip tests cannot see a self-consistent format change — a
        // u64 length or a BE version keeps its own tests green while every
        // save file in the wild goes unreadable. The wire contract is
        // pinned: MAGIC(4) + version(u16 LE) + len(u32 LE) + payload.
        let enc = Save::encode(0x0102, b"AB");
        assert_eq!(
            enc,
            b"IZAN\x02\x01\x02\x00\x00\x00AB".to_vec(),
            "save layout drifted — old saves can no longer be read"
        );
        let (v, d) = Save::parse(&enc).unwrap();
        assert_eq!(v, 0x0102);
        assert_eq!(d, b"AB");
    }

    #[test]
    fn write_and_encode_emit_the_same_bytes() {
        // `write` delegates to `encode`; this guards the delegation itself —
        // a second layout in `write` would break files while the pinned
        // in-memory bytes stayed green.
        let dir = std::env::temp_dir().join("izanagi_save_layout_probe.dat");
        Save::write(&dir, 0xBEEF, b"xy").unwrap();
        let on_disk = std::fs::read(&dir).unwrap();
        let _ = std::fs::remove_file(&dir);
        assert_eq!(on_disk, Save::encode(0xBEEF, b"xy"));
    }
}
