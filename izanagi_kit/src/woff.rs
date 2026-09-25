//! WOFF (Web Open Font Format) wrapper: the `wOFF` header, the
//! 20-byte table directory, and decompression of individual tables
//! via `inflate` (entries whose `compressed_length` < `orig_length`
//! are raw DEFLATE streams). WOFF2 (`wOF2`) is *not* covered — its
//! brotli compression is outside this crate's reach.
//!
//! `ttf` parses the sfnt inside; `woff` is the delivery wrapper.
//!
//! ```
//! use izanagi_kit::woff;
//! let mut f = b"wOFF".to_vec();
//! f.extend_from_slice(&[0, 1, 0, 0]); // flavor: TrueType
//! f.extend_from_slice(&[0, 0, 0, 68]); // length = 44 + 20 + 4
//! f.extend_from_slice(&[0, 1]);          // numTables
//! f.extend_from_slice(&[0, 0]);          // reserved
//! f.extend_from_slice(&[0, 0, 0, 8]);    // totalSfntSize
//! f.extend_from_slice(&[0; 24]); // versions + meta/priv
//! // table record: tag, offset, compLen, origLen, checksum
//! f.extend_from_slice(b"head");
//! f.extend_from_slice(&[0, 0, 0, 64]); // offset
//! f.extend_from_slice(&[0, 0, 0, 4]);  // compLen == origLen → stored
//! f.extend_from_slice(&[0, 0, 0, 4]);
//! f.extend_from_slice(&[0, 0, 0, 0]);
//! f.extend_from_slice(b"HEAD"); // the table data at offset 64
//! let w = woff::parse(&f).unwrap();
//! assert_eq!(w.tables[0].name(), "head");
//! assert_eq!(woff::inflate_table(&f, &w, 0).unwrap(), b"HEAD");
//! ```

fn r16(d: &[u8], at: usize) -> Option<u32> {
    Some((*d.get(at)? as u32) << 8 | *d.get(at + 1)? as u32)
}
fn r32(d: &[u8], at: usize) -> Option<u32> {
    Some(r16(d, at)? << 16 | r16(d, at + 2)?)
}

/// One WOFF table-directory record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    /// 4-byte sfnt tag.
    pub tag: [u8; 4],
    /// File offset of the (possibly compressed) data.
    pub offset: usize,
    /// Compressed length (`== orig_length` means stored verbatim).
    pub compressed_length: usize,
    /// Original (decompressed) length.
    pub orig_length: usize,
    /// Checksum of the *original* table data.
    pub checksum: u32,
}

impl Tab {
    /// Tag as a display string.
    pub fn name(&self) -> String {
        self.tag
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '?' })
            .collect()
    }

    /// Whether the entry is deflate-compressed.
    pub fn compressed(&self) -> bool {
        self.compressed_length < self.orig_length
    }
}

/// A parsed WOFF container.
#[derive(Clone, Debug)]
pub struct Woff {
    /// The sfnt flavor this wraps (`0x00010000` TrueType, `OTTO` CFF).
    pub flavor: u32,
    /// Declared total file length.
    pub total_length: usize,
    /// Font version (major/minor).
    pub version: (u16, u16),
    /// Table directory.
    pub tables: Vec<Tab>,
}

/// Parse a `wOFF` header + directory.
pub fn parse(d: &[u8]) -> Option<Woff> {
    if d.get(0..4)? != b"wOFF" {
        return None;
    }
    let flavor = r32(d, 4)?;
    let total_length = r32(d, 8)? as usize;
    let num = r16(d, 12)? as usize;
    let version = (r16(d, 20)? as u16, r16(d, 22)? as u16);
    let mut tables = Vec::with_capacity(num.min(64));
    for i in 0..num {
        let at = 44usize.checked_add(i.checked_mul(20)?)?;
        if at.checked_add(20)? > d.len() {
            return None;
        }
        let tag = [d[at], d[at + 1], d[at + 2], d[at + 3]];
        let offset = r32(d, at + 4)? as usize;
        let compressed_length = r32(d, at + 8)? as usize;
        let orig_length = r32(d, at + 12)? as usize;
        let checksum = r32(d, at + 16)?;
        if compressed_length > orig_length {
            return None; // impossible per spec
        }
        tables.push(Tab {
            tag,
            offset,
            compressed_length,
            orig_length,
            checksum,
        });
    }
    Some(Woff {
        flavor,
        total_length,
        version,
        tables,
    })
}

/// Table bytes — verbatim when stored, deflated via `inflate` when
/// compressed. `None` when the entry is out of range, truncated, or
/// inflates to a size that isn't `orig_length`.
pub fn inflate_table(d: &[u8], w: &Woff, index: usize) -> Option<Vec<u8>> {
    let t = w.tables.get(index)?;
    let end = t.offset.checked_add(t.compressed_length)?;
    if end > d.len() {
        return None;
    }
    let raw = &d[t.offset..end];
    if !t.compressed() {
        return Some(raw.to_vec());
    }
    let out = crate::inflate::inflate_zlib(raw)?;
    if out.len() != t.orig_length {
        return None; // deflate'd to a wrong size — corrupt
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn woff_one(stored: &[u8]) -> Vec<u8> {
        let mut f = b"wOFF".to_vec();
        f.extend_from_slice(&[0, 1, 0, 0]);
        let len = 44 + 20 + stored.len();
        f.extend_from_slice(&[
            (len >> 24) as u8,
            (len >> 16) as u8,
            (len >> 8) as u8,
            len as u8,
        ]);
        f.extend_from_slice(&[0, 1, 0, 0]);
        f.extend_from_slice(&[0; 28]); // totalSfntSize/version/meta/priv
        f.extend_from_slice(b"head");
        f.extend_from_slice(&[0, 0, 0, 64]);
        f.extend_from_slice(&[
            (stored.len() >> 24) as u8,
            (stored.len() >> 16) as u8,
            (stored.len() >> 8) as u8,
            stored.len() as u8,
        ]);
        f.extend_from_slice(&[
            (stored.len() >> 24) as u8,
            (stored.len() >> 16) as u8,
            (stored.len() >> 8) as u8,
            stored.len() as u8,
        ]);
        f.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        f.extend_from_slice(stored);
        f
    }

    #[test]
    fn header_and_stored_table() {
        let d = woff_one(b"ABCD");
        let w = parse(&d).unwrap();
        assert_eq!(w.flavor, 0x0001_0000);
        assert_eq!(w.version, (0, 0));
        assert_eq!(w.tables.len(), 1);
        assert!(!w.tables[0].compressed());
        assert_eq!(inflate_table(&d, &w, 0).unwrap(), b"ABCD");
    }

    #[test]
    fn compressed_table_roundtrip() {
        // deflate-compress "AAAA…" via the crate's own deflate
        let raw = b"WOFFWOFFWOFFWOFFWOFFWOFFWOFFWOFF";
        let comp = crate::deflate::deflate_zlib(raw);
        let mut f = b"wOFF".to_vec();
        f.extend_from_slice(&[0, 1, 0, 0]);
        let len = 44 + 20 + comp.len();
        f.extend_from_slice(&[
            (len >> 24) as u8,
            (len >> 16) as u8,
            (len >> 8) as u8,
            len as u8,
        ]);
        f.extend_from_slice(&[0, 1, 0, 0]);
        f.extend_from_slice(&[0; 28]);
        f.extend_from_slice(b"cmap");
        f.extend_from_slice(&[0, 0, 0, 64]);
        f.extend_from_slice(&[
            (comp.len() >> 24) as u8,
            (comp.len() >> 16) as u8,
            (comp.len() >> 8) as u8,
            comp.len() as u8,
        ]);
        f.extend_from_slice(&[
            (raw.len() >> 24) as u8,
            (raw.len() >> 16) as u8,
            (raw.len() >> 8) as u8,
            raw.len() as u8,
        ]);
        f.extend_from_slice(&[0, 0, 0, 0]);
        f.extend_from_slice(&comp);
        let w = parse(&f).unwrap();
        assert!(w.tables[0].compressed());
        assert_eq!(inflate_table(&f, &w, 0).unwrap(), raw.to_vec());
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"wOF2").is_none());
        let mut f = woff_one(b"ABCD");
        f[13] = 4; // claim 4 tables, 1 present
        assert!(parse(&f).is_none());
        // compLen > origLen is impossible
        let mut g = woff_one(b"ABCD");
        g[55] = 9; // compLen=9 while origLen=4
        assert!(parse(&g).is_none());
    }
}
