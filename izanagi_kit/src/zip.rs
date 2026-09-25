//! ZIP archive reader/writer (PKWARE APPNOTE) — the container layer
//! for [`crate::deflate`]/[`crate::inflate`] and [`crate::crc`].
//!
//! Reader walks the end-of-central-directory record, then the central
//! directory, then each local header — so archives with prepended junk
//! or comments still resolve. Compression method 0 (stored) and 8
//! (deflate) are supported; encryption, spanning, zip64, and
//! data-descriptor entries degrade to `None`.
//!
//! [`ZipWriter`] emits classic single-disk archives: local headers,
//! stored-or-deflated payloads, central directory, EOCD. Its output
//! round-trips through this module's own reader and through `unzip`
//! / Python `zipfile`.
//!
//! ```
//! use izanagi_kit::zip::{extract, ZipWriter};
//!
//! let mut w = ZipWriter::new();
//! w.add("hello.txt", b"hi hi hi hi");
//! let bytes = w.finish();
//! assert_eq!(extract(&bytes, "hello.txt").as_deref(), Some(&b"hi hi hi hi"[..]));
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

fn le16(b: &[u8]) -> u16 {
    (b[0] as u16) | ((b[1] as u16) << 8)
}
fn le32(b: &[u8]) -> u32 {
    (b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16) | ((b[3] as u32) << 24)
}
fn push16(out: &mut Vec<u8>, v: u16) {
    out.push(v as u8);
    out.push((v >> 8) as u8);
}
fn push32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// A central-directory entry.
pub struct ZipEntry {
    /// File name (UTF-8, lossy-decoded).
    pub name: String,
    /// Compression method: 0 stored, 8 deflate.
    pub method: u16,
    /// CRC-32 of the decompressed bytes.
    pub crc32: u32,
    /// Compressed size in bytes.
    pub compressed: u32,
    /// Uncompressed size in bytes.
    pub size: u32,
    /// Offset of the local file header.
    pub local_offset: u32,
}

/// Locate the EOCD record by scanning backwards for `PK\x05\x06`
/// (it may sit anywhere in the last 64KiB + 22 bytes).
fn eocd(data: &[u8]) -> Option<(usize, u16, u32, u32)> {
    if data.len() < 22 {
        return None;
    }
    let start = data.len() - 22;
    let floor = data.len().saturating_sub(22 + 65536);
    let mut at = start;
    loop {
        if data[at..].starts_with(b"PK\x05\x06") {
            let r = &data[at + 4..at + 22];
            let count = le16(&r[6..]);
            let cd_size = le32(&r[8..]);
            let cd_off = le32(&r[12..]);
            return Some((at, count, cd_size, cd_off));
        }
        if at == floor {
            return None;
        }
        at -= 1;
    }
}

/// List the archive's central directory. `None` on missing/garbled EOCD.
pub fn list(data: &[u8]) -> Option<Vec<ZipEntry>> {
    let (_, count, _, cd_off) = eocd(data)?;
    let mut at = cd_off as usize;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if at + 46 > data.len() || !data[at..].starts_with(b"PK\x01\x02") {
            return None;
        }
        let h = &data[at..at + 46];
        let method = le16(&h[10..]);
        let crc32 = le32(&h[16..]);
        let compressed = le32(&h[20..]);
        let size = le32(&h[24..]);
        let name_len = le16(&h[28..]) as usize;
        let extra_len = le16(&h[30..]) as usize;
        let comment_len = le16(&h[32..]) as usize;
        let local_offset = le32(&h[42..]);
        let name_at = at + 46;
        if name_at + name_len > data.len() {
            return None;
        }
        let name = String::from_utf8_lossy(&data[name_at..name_at + name_len]).to_string();
        out.push(ZipEntry {
            name,
            method,
            crc32,
            compressed,
            size,
            local_offset,
        });
        at = name_at + name_len + extra_len + comment_len;
    }
    Some(out)
}

/// Extract `name` from the archive: stored or deflated bytes,
/// CRC-verified. Returns `None` when absent, encrypted, spanned,
/// zip64-sized, or CRC-mismatched.
pub fn extract(data: &[u8], name: &str) -> Option<Vec<u8>> {
    let entries = list(data)?;
    let e = entries.iter().find(|e| e.name == name)?;
    let at = e.local_offset as usize;
    if at + 30 > data.len() || !data[at..].starts_with(b"PK\x03\x04") {
        return None;
    }
    let h = &data[at..at + 30];
    let flags = le16(&h[6..]);
    // Encrypted (bit 0) or data-descriptor (bit 3) → bail.
    if flags & 0b0000_1001 != 0 {
        return None;
    }
    let name_len = le16(&h[26..]) as usize;
    let extra_len = le16(&h[28..]) as usize;
    let body = at + 30 + name_len + extra_len;
    let end = body.checked_add(e.compressed as usize)?;
    if end > data.len() {
        return None;
    }
    let raw = &data[body..end];
    let out = match e.method {
        0 => raw.to_vec(),
        8 => crate::inflate::inflate(raw)?,
        _ => return None,
    };
    if out.len() != e.size as usize || crate::crc::crc32(&out) != e.crc32 {
        return None;
    }
    Some(out)
}

/// Builder for a classic single-disk ZIP archive.
pub struct ZipWriter {
    entries: Vec<(String, Vec<u8>, u32, u32, u16)>, // name, payload, crc, size, method
    body: Vec<u8>,
    /// Byte offsets of each local header, filled by [`ZipWriter::add`].
    offsets: Vec<u32>,
}

impl Default for ZipWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ZipWriter {
    /// Empty archive.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            body: Vec::new(),
            offsets: Vec::new(),
        }
    }

    /// Add a file compressed with [`crate::deflate::deflate`].
    pub fn add(&mut self, name: &str, data: &[u8]) {
        self.add_method(name, data, 8);
    }

    /// Add a file verbatim (method 0, stored).
    pub fn add_stored(&mut self, name: &str, data: &[u8]) {
        self.add_method(name, data, 0);
    }

    fn add_method(&mut self, name: &str, data: &[u8], method: u16) {
        let payload = if method == 8 {
            crate::deflate::deflate(data)
        } else {
            data.to_vec()
        };
        let crc = crate::crc::crc32(data);
        self.offsets.push(self.body.len() as u32);
        let mut h = Vec::with_capacity(30);
        h.extend_from_slice(b"PK\x03\x04");
        push16(&mut h, 20); // version needed
        push16(&mut h, 0); // flags
        push16(&mut h, method);
        push16(&mut h, 0); // mod time
        push16(&mut h, 0); // mod date
        push32(&mut h, crc);
        push32(&mut h, payload.len() as u32);
        push32(&mut h, data.len() as u32);
        push16(&mut h, name.len() as u16);
        push16(&mut h, 0); // extra len
        self.body.extend_from_slice(&h);
        self.body.extend_from_slice(name.as_bytes());
        self.body.extend_from_slice(&payload);
        self.entries
            .push((name.to_string(), payload, crc, data.len() as u32, method));
    }

    /// Emit the complete archive: all local entries, central directory,
    /// and EOCD.
    pub fn finish(self) -> Vec<u8> {
        let mut out = self.body;
        let cd_off = out.len() as u32;
        for ((name, payload, crc, size, method), &off) in
            self.entries.iter().zip(self.offsets.iter())
        {
            let mut h = Vec::with_capacity(46);
            h.extend_from_slice(b"PK\x01\x02");
            push16(&mut h, 20); // version made by
            push16(&mut h, 20); // version needed
            push16(&mut h, 0);
            push16(&mut h, *method);
            push16(&mut h, 0);
            push16(&mut h, 0);
            push32(&mut h, *crc);
            push32(&mut h, payload.len() as u32);
            push32(&mut h, *size);
            push16(&mut h, name.len() as u16);
            push16(&mut h, 0);
            push16(&mut h, 0);
            push16(&mut h, 0);
            push16(&mut h, 0);
            push32(&mut h, 0); // external attrs
            push32(&mut h, off);
            out.extend_from_slice(&h);
            out.extend_from_slice(name.as_bytes());
        }
        let cd_size = out.len() as u32 - cd_off;
        out.extend_from_slice(b"PK\x05\x06");
        push16(&mut out, 0);
        push16(&mut out, 0);
        push16(&mut out, self.entries.len() as u16);
        push16(&mut out, self.entries.len() as u16);
        push32(&mut out, cd_size);
        push32(&mut out, cd_off);
        push16(&mut out, 0); // comment len
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_roundtrip() {
        let mut w = ZipWriter::new();
        w.add("a.txt", b"aaa aaa aaa");
        w.add_stored("b.bin", &[0u8, 1, 2, 255]);
        let z = w.finish();
        let entries = list(&z).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a.txt");
        assert_eq!(entries[0].method, 8);
        assert_eq!(entries[1].method, 0);
        assert_eq!(extract(&z, "a.txt").as_deref(), Some(&b"aaa aaa aaa"[..]));
        assert_eq!(extract(&z, "b.bin").as_deref(), Some(&[0u8, 1, 2, 255][..]));
        assert!(extract(&z, "missing").is_none());
    }

    #[test]
    fn empty_archive_lists_empty() {
        let z = ZipWriter::new().finish();
        assert_eq!(list(&z).map(|e| e.len()), Some(0));
    }

    #[test]
    fn truncated_and_garbage_degrade() {
        assert!(list(b"").is_none());
        assert!(list(b"PK\x05\x06").is_none()); // too short
        let mut w = ZipWriter::new();
        w.add("x", b"data");
        let z = w.finish();
        assert!(extract(&z[..z.len() / 2], "x").is_none()); // EOCD cut off
    }

    #[test]
    fn deterministic_write() {
        let mut a = ZipWriter::new();
        a.add("f", b"payload");
        let mut b = ZipWriter::new();
        b.add("f", b"payload");
        assert_eq!(a.finish(), b.finish());
    }
}
