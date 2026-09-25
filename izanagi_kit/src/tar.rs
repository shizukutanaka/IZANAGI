//! POSIX ustar tar archives — the sibling container of
//! [`crate::zip`] that composes with [`crate::deflate::deflate_gzip`]
//! to produce real `.tar.gz`.
//!
//! Format: 512-byte header blocks (name, mode, uid/gid, octal size and
//! mtime, checksum, typeflag, `ustar\0` magic, prefix), file payloads
//! padded to 512, two zero blocks at end. Header checksums are
//! verified on decode (the `chksum` field reads as eight spaces while
//! summing). Long names over the 100/155-byte ustar fields degrade to
//! `None` — no GNU longname entries.
//!
//! [`TarWriter::finish`] emits a deterministic archive: mode `644`,
//! uid/gid/mtime `0`, checksum computed, end blocks appended.
//!
//! ```
//! use izanagi_kit::tar::{extract, TarWriter};
//!
//! let mut w = TarWriter::new();
//! w.add("a.txt", b"hello");
//! let t = w.finish();
//! assert_eq!(extract(&t, "a.txt").as_deref(), Some(&b"hello"[..]));
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

const BLOCK: usize = 512;

/// A parsed tar header entry.
pub struct TarEntry {
    /// Path (prefix + name joined with `/`).
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// Permission bits (octal field decoded).
    pub mode: u32,
    /// Typeflag byte: `0`/`'\0'` file, `5` directory, others opaque.
    pub typeflag: u8,
    /// Byte offset of this entry's payload.
    pub offset: usize,
}

fn octal(b: &[u8]) -> Option<u64> {
    let mut v = 0u64;
    for &c in b {
        match c {
            b'0'..=b'7' => {
                v = (v << 3) | (c - b'0') as u64;
                if v > u64::MAX >> 4 {
                    return None;
                }
            }
            0 | b' ' => {}
            _ => return None,
        }
    }
    Some(v)
}

fn field_str(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).to_string()
}

fn parse_header(h: &[u8]) -> Option<TarEntry> {
    if h.iter().all(|&b| b == 0) {
        return None;
    }
    if &h[257..262] != b"ustar" {
        return None;
    }
    // Checksum: field reads as spaces during the sum.
    let stored = octal(&h[148..156])?;
    let mut sum = 0u64;
    for (i, &b) in h.iter().enumerate() {
        sum += if (148..156).contains(&i) {
            b' ' as u64
        } else {
            b as u64
        };
    }
    if sum != stored {
        return None;
    }
    let name = field_str(&h[0..100]);
    let prefix = field_str(&h[345..500]);
    let full = if prefix.is_empty() {
        name
    } else {
        std::format!("{prefix}/{name}")
    };
    Some(TarEntry {
        name: full,
        size: octal(&h[124..136])?,
        mode: octal(&h[100..108])? as u32,
        typeflag: h[156],
        offset: 0,
    })
}

/// List every member. `None` on a bad checksum, non-ustar magic, or a
/// header block that ends mid-file.
pub fn list(data: &[u8]) -> Option<Vec<TarEntry>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    loop {
        if at + BLOCK > data.len() {
            return None;
        }
        let h = &data[at..at + BLOCK];
        if h.iter().all(|&b| b == 0) {
            return Some(out);
        }
        let mut e = parse_header(h)?;
        e.offset = at + BLOCK;
        // Payload is padded up to a whole block.
        let blocks = (e.size as usize).div_ceil(BLOCK);
        if e.offset + blocks * BLOCK > data.len() {
            return None;
        }
        out.push(e);
        at += BLOCK + blocks * BLOCK;
    }
}

/// Extract `name`'s payload bytes. `None` when absent or the archive
/// is malformed.
pub fn extract(data: &[u8], name: &str) -> Option<Vec<u8>> {
    let entries = list(data)?;
    let e = entries.iter().find(|e| e.name == name)?;
    Some(data[e.offset..e.offset + e.size as usize].to_vec())
}

fn put_octal(h: &mut [u8], start: usize, width: usize, v: u64) {
    let s = std::format!("{:o}", v);
    let digits = s.as_bytes();
    // Right-aligned octal, NUL-terminated.
    let field = &mut h[start..start + width];
    for b in field.iter_mut() {
        *b = 0;
    }
    let pad = width - 1 - digits.len();
    field[pad..pad + digits.len()].copy_from_slice(digits);
}

/// Builder for a deterministic ustar archive.
pub struct TarWriter {
    out: Vec<u8>,
}

impl Default for TarWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl TarWriter {
    /// Empty archive.
    pub fn new() -> Self {
        Self { out: Vec::new() }
    }

    /// Add a regular file. Fails silently into a header-less no-op on
    /// names that cannot fit the ustar name/prefix split — the caller
    /// sees them absent from `list`.
    pub fn add(&mut self, path: &str, data: &[u8]) {
        // Split into name ≤100 and prefix ≤155 on a `/` boundary.
        let b = path.as_bytes();
        if b.len() > 255 || b.is_empty() {
            return;
        }
        let (name, prefix) = if b.len() <= 100 {
            (path, "")
        } else {
            match path.rfind('/') {
                Some(i) if b.len() - i - 1 <= 100 && i <= 155 => (&path[i + 1..], &path[..i]),
                _ => return,
            }
        };
        let mut h = [0u8; BLOCK];
        h[..name.len()].copy_from_slice(name.as_bytes());
        put_octal(&mut h, 100, 8, 0o644); // mode
        put_octal(&mut h, 108, 8, 0); // uid
        put_octal(&mut h, 116, 8, 0); // gid
        put_octal(&mut h, 124, 12, data.len() as u64);
        put_octal(&mut h, 136, 12, 0); // mtime 0 — deterministic
                                       // checksum written after everything else.
        h[156] = b'0';
        h[257..262].copy_from_slice(b"ustar");
        h[262] = 0;
        h[263..265].copy_from_slice(b"00");
        h[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
        let mut sum = 0u64;
        for (i, &b) in h.iter().enumerate() {
            sum += if (148..156).contains(&i) {
                b' ' as u64
            } else {
                b as u64
            };
        }
        let s = std::format!("{:06o}", sum);
        h[148..154].copy_from_slice(s.as_bytes());
        h[154] = 0;
        h[155] = b' ';
        self.out.extend_from_slice(&h);
        self.out.extend_from_slice(data);
        let pad = (BLOCK - data.len() % BLOCK) % BLOCK;
        self.out.extend(std::iter::repeat(0u8).take(pad));
    }

    /// Finish with the two end-of-archive zero blocks.
    pub fn finish(self) -> Vec<u8> {
        let mut out = self.out;
        out.extend(std::iter::repeat(0u8).take(2 * BLOCK));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_roundtrip() {
        let mut w = TarWriter::new();
        w.add("a.txt", b"hello");
        w.add("dir/deep/file.bin", &[0u8; 600]);
        let t = w.finish();
        let es = list(&t).unwrap();
        assert_eq!(es.len(), 2);
        assert_eq!(es[0].name, "a.txt");
        assert_eq!(es[0].size, 5);
        assert_eq!(es[0].mode, 0o644);
        assert_eq!(es[0].typeflag, b'0');
        assert_eq!(extract(&t, "dir/deep/file.bin"), Some(vec![0u8; 600]));
        assert!(extract(&t, "nope").is_none());
    }

    #[test]
    fn long_name_splits_prefix() {
        let mut w = TarWriter::new();
        let p = std::format!("{}/leaf", "d".repeat(120));
        w.add(&p, b"x");
        let t = w.finish();
        assert_eq!(extract(&t, &p).as_deref(), Some(&b"x"[..]));
    }

    #[test]
    fn malformed_degrades() {
        assert!(list(b"").is_none());
        assert_eq!(list(&[0u8; 512]).map(|e| e.len()), Some(0)); // bare end
        let mut w = TarWriter::new();
        w.add("f", b"data");
        let mut t = w.finish();
        t[148] = b'9'; // corrupt checksum
        assert!(list(&t).is_none());
    }

    #[test]
    fn deterministic_write() {
        let mut a = TarWriter::new();
        a.add("f", b"payload");
        let mut b = TarWriter::new();
        b.add("f", b"payload");
        assert_eq!(a.finish(), b.finish());
    }
}
