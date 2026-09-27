//! WOFF 2.0 font header (W3C WOFF2 spec).
//!
//! The 48-byte big-endian header carries the `wOF2` signature, the
//! sfnt flavor, total sizes, numTables, and offsets of the optional
//! metadata and private blocks. The compressed Brotli payload and
//! transform tables are not decoded here.
//!
//! ```
//! use izanagi_kit::woff2::parse;
//! let mut d = vec![0u8; 48];
//! d[0..4].copy_from_slice(b"wOF2");
//! d[4..8].copy_from_slice(&[0, 1, 0, 0]);   // flavor: TTF 1.0
//! d[8..12].copy_from_slice(&[0, 0, 0, 48]); // length = 48 (whole buffer)
//! d[12..14].copy_from_slice(&[0, 13]);      // 13 tables
//! d[16..20].copy_from_slice(&[0, 0, 4, 0]); // totalSfntSize
//! d[20..24].copy_from_slice(&[0, 0, 2, 0]); // totalCompressedSize
//! let w = parse(&d).unwrap();
//! assert_eq!(w.num_tables, 13);
//! assert!(w.is_valid_flavor());
//! ```

fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some((*d.get(o)? as u16) << 8 | *d.get(o + 1)? as u16)
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32) << 24
            | (*d.get(o + 1)? as u32) << 16
            | (*d.get(o + 2)? as u32) << 8
            | *d.get(o + 3)? as u32,
    )
}

fn four(d: &[u8], o: usize) -> Option<[u8; 4]> {
    let mut v = [0u8; 4];
    v.copy_from_slice(d.get(o..o + 4)?);
    Some(v)
}

/// Header size in bytes.
pub const HEADER: usize = 48;
/// `wOF2` signature.
pub const SIG: [u8; 4] = *b"wOF2";

/// Parsed WOFF2 header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Woff2 {
    /// sfnt flavor (`0x00010000` TTF, `OTTO` CFF, `true`/`typ1`, `ttcf` collection).
    pub flavor: [u8; 4],
    /// Total WOFF2 file size in bytes.
    pub length: u32,
    /// Number of font tables.
    pub num_tables: u16,
    /// Uncompressed size of the reconstructed sfnt.
    pub total_sfnt_size: u32,
    /// Size of the Brotli-compressed block.
    pub total_compressed_size: u32,
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
    /// Optional metadata block `(offset, length, orig_length)`.
    pub meta: Option<(u32, u32, u32)>,
    /// Optional private block `(offset, length)`.
    pub priv_: Option<(u32, u32)>,
}

impl Woff2 {
    /// `true` when `flavor` is one of the defined values
    /// (`0x00010000`, `true`, `typ1`, `OTTO`, `ttcf`).
    pub fn is_valid_flavor(&self) -> bool {
        self.flavor == [0, 1, 0, 0]
            || self.flavor == *b"true"
            || self.flavor == *b"typ1"
            || self.flavor == *b"OTTO"
            || self.flavor == *b"ttcf"
    }
    /// `true` when this file carries a `ttcf` collection flavor.
    pub fn is_collection(&self) -> bool {
        self.flavor == *b"ttcf"
    }
}

/// Parse the 48-byte `wOF2` header. `None` on bad magic, size below
/// 48, reserved field non-zero, or a `length` exceeding the buffer.
pub fn parse(d: &[u8]) -> Option<Woff2> {
    if four(d, 0)? != SIG {
        return None;
    }
    let flavor = four(d, 4)?;
    let length = be32(d, 8)?;
    let num_tables = be16(d, 12)?;
    if be16(d, 14)? != 0 {
        return None; // reserved
    }
    let total_sfnt_size = be32(d, 16)?;
    let total_compressed_size = be32(d, 20)?;
    let major = be16(d, 24)?;
    let minor = be16(d, 26)?;
    let meta = (be32(d, 28)?, be32(d, 32)?, be32(d, 36)?);
    let priv_ = (be32(d, 40)?, be32(d, 44)?);
    if length < HEADER as u32 || length as usize > d.len() {
        return None;
    }
    // offset-only consistency: nonzero length requires nonzero offset.
    if (meta.1 != 0 && meta.0 == 0) || (priv_.1 != 0 && priv_.0 == 0) {
        return None;
    }
    Some(Woff2 {
        flavor,
        length,
        num_tables,
        total_sfnt_size,
        total_compressed_size,
        major,
        minor,
        meta: if meta.0 != 0 { Some(meta) } else { None },
        priv_: if priv_.0 != 0 { Some(priv_) } else { None },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 256];
        let w = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> ((3 - i) * 8)) as u8;
            }
        };
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            d[o] = (v >> 8) as u8;
            d[o + 1] = v as u8;
        };
        d[0..4].copy_from_slice(b"wOF2");
        w(&mut d, 4, 0x0001_0000);
        w(&mut d, 8, 256);
        w16(&mut d, 12, 13);
        w(&mut d, 16, 1024);
        w(&mut d, 20, 512);
        w16(&mut d, 24, 2);
        w16(&mut d, 26, 1);
        w(&mut d, 28, 200); // meta offset
        w(&mut d, 32, 40); // meta length
        w(&mut d, 36, 128); // meta orig
        w(&mut d, 40, 248); // priv offset
        w(&mut d, 44, 8); // priv len
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let w = parse(&d).unwrap();
        assert_eq!(w.flavor, [0, 1, 0, 0]);
        assert_eq!(w.length, 256);
        assert_eq!(w.num_tables, 13);
        assert_eq!(w.total_sfnt_size, 1024);
        assert_eq!(w.total_compressed_size, 512);
        assert_eq!((w.major, w.minor), (2, 1));
        assert_eq!(w.meta, Some((200, 40, 128)));
        assert_eq!(w.priv_, Some((248, 8)));
        assert!(w.is_valid_flavor());
        assert!(!w.is_collection());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"wOF1").is_none());
        let mut d = fixture();
        d[15] = 1; // reserved nonzero
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[11] = 0xFF; // length huge
        assert!(parse(&d2).is_none());
        let mut d3 = fixture();
        d3[28..32].copy_from_slice(&[0, 0, 0, 0]); // meta offset 0 with len 40
        assert!(parse(&d3).is_none());
        let mut d4 = fixture();
        d4[4..8].copy_from_slice(b"ttcf");
        assert!(parse(&d4).unwrap().is_collection());
    }
}
