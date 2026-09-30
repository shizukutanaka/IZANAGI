//! Aard2 `.slob` — sorted-list-of-blobs dictionary container:
//! magic `21-2\x02\x02SLOB\x1F`, then `u16be` encoding names
//! (each `u8` length + bytes), `u16be` tags (`u8`-len key +
//! `u8`-len value), `u16be` content types, `u32be` blob count
//! and `u64be` store offset.
//!
//! ```
//! use izanagi_kit::slob::{detect, parse};
//!
//! let mut d = b"!-2\x02\x02SLOB\x1F".to_vec();
//! d.extend_from_slice(&[0, 1, 5, b'U', b'T', b'F', b'-', b'8']); // 1 encoding
//! d.extend_from_slice(&[0, 1, 4, b'l', b'a', b'b', b'l']);       // 1 tag key
//! // value
//! let mut d2 = d.clone();
//! d2.extend_from_slice(&[5, b'l', b'a', b'b', b'l', b'e']);
//! d2.extend_from_slice(&[0, 0]);                                 // 0 content types
//! d2.extend_from_slice(&[0, 0, 0, 7]);                           // blob count
//! d2.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 40]);              // store offset
//! assert!(detect(&d2));
//! let s = parse(&d2).unwrap();
//! assert_eq!(s.encodings, 1);
//! assert_eq!(s.blob_count, 7);
//! ```

/// Parsed Aard2 `.slob` header census.
#[derive(Debug, Clone, PartialEq)]
pub struct Slob {
    /// Text encodings declared (first entry is the primary).
    pub encodings: u32,
    /// First declared encoding name (usually `UTF-8`).
    pub primary_encoding: Option<String>,
    /// `u8`-len key/`u8`-len value tag pairs.
    pub tags: u32,
    /// MIME-ish content-type variants declared.
    pub content_types: u32,
    /// `u32be` stored blob count.
    pub blob_count: u32,
    /// `u64be` byte offset of the blob store.
    pub store_offset: u64,
}

const MAGIC: &[u8] = b"!-2\x02\x02SLOB\x1F";

fn be16(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from(*b.get(off)?) << 8 | u16::from(*b.get(off + 1)?))
}

fn u8str<'a>(b: &'a [u8], off: &mut usize) -> Option<&'a str> {
    let n = usize::from(*b.get(*off)?);
    *off += 1;
    let s = b.get(*off..*off + n)?;
    *off += n;
    core::str::from_utf8(s).ok()
}

fn walk(b: &[u8]) -> Option<(u32, Option<String>, u32, u32, u32, u64)> {
    let mut off = MAGIC.len();
    let n_enc = u32::from(be16(b, off)?);
    off += 2;
    let mut primary = None;
    for i in 0..n_enc {
        let s = u8str(b, &mut off)?;
        if i == 0 {
            primary = Some(s.to_string());
        }
    }
    let n_tags = u32::from(be16(b, off)?);
    off += 2;
    for _ in 0..n_tags {
        u8str(b, &mut off)?;
        u8str(b, &mut off)?;
    }
    let n_ct = u32::from(be16(b, off)?);
    off += 2;
    for _ in 0..n_ct {
        u8str(b, &mut off)?;
    }
    let blob = u32::from(*b.get(off)?) << 24
        | u32::from(*b.get(off + 1)?) << 16
        | u32::from(*b.get(off + 2)?) << 8
        | u32::from(*b.get(off + 3)?);
    off += 4;
    let mut store = 0u64;
    for i in 0..8 {
        store |= u64::from(*b.get(off + i)?) << (56 - i * 8);
    }
    Some((n_enc, primary, n_tags, n_ct, blob, store))
}

/// `true` on the `SLOB` magic prefix.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(MAGIC)
}

/// Census; `None` on bad magic or a truncated header walk.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Slob> {
    if !detect(b) {
        return None;
    }
    let (encodings, primary, tags, cts, blob, store) = walk(b)?;
    Some(Slob {
        encodings,
        primary_encoding: primary,
        tags,
        content_types: cts,
        blob_count: blob,
        store_offset: store,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&[0, 1, 5, b'U', b'T', b'F', b'-', b'8']);
        d.extend_from_slice(&[0, 1, 5, b'l', b'a', b'b', b'e', b'l']);
        d.extend_from_slice(&[7, b'c', b'o', b'n', b't', b'e', b'n', b't']);
        d.extend_from_slice(&[
            0, 1, 9, b't', b'e', b'x', b't', b'/', b'h', b't', b'm', b'l',
        ]);
        d.extend_from_slice(&[0, 0, 0, 7]);
        d.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 40]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&build()));
        assert!(!detect(b"SLOB"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn parses() {
        let s = parse(&build()).unwrap();
        assert_eq!(s.encodings, 1);
        assert_eq!(s.primary_encoding.as_deref(), Some("UTF-8"));
        assert_eq!(s.tags, 1);
        assert_eq!(s.content_types, 1);
        assert_eq!(s.blob_count, 7);
        assert_eq!(s.store_offset, 40);
    }

    #[test]
    fn truncated_rejects() {
        let mut d = build();
        d.truncate(d.len() - 4);
        assert!(parse(&d).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
