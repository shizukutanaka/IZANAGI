//! XAR (eXtensible ARchive) header — macOS `.pkg` / `.xar`.
//!
//! The file opens with the `xar!` magic (u32 BE), a u16 header
//! size (28), a u16 version, the compressed and uncompressed TOC
//! lengths (u64 BE) and a u32 checksum-type id (0 none, 1 SHA-1,
//! 2 MD5, 3 SHA-256, 4 SHA-512). The XML TOC follows, zlib- or
//! none-compressed. `parse` reports the header fields; the TOC
//! body is not inflated here (the kit stays dependency-free).
//!
//! ```
//! use izanagi_kit::xar::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 20];
//! d[0..4].copy_from_slice(b"xar!");
//! d[5] = 28;                    // header size u16 BE
//! d[6] = 0; d[7] = 1;           // version 1
//! let w64 = |d: &mut [u8], o: usize, v: u64| {
//!     for i in 0..8 { d[o + i] = (v >> ((7 - i) * 8)) as u8; }
//! };
//! w64(&mut d, 8, 20);           // compressed TOC (fits in buffer)
//! w64(&mut d, 16, 45);          // uncompressed TOC
//! d[27] = 1;                    // sha1 checksum
//! let x = parse(&d).unwrap();
//! assert_eq!(x.checksum, izanagi_kit::xar::Checksum::Sha1);
//! ```

/// Header size in bytes.
pub const HEADER: usize = 28;
/// Magic `xar!` as u32 BE.
pub const MAGIC: u32 = 0x7861_7221;

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}
fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}
fn be64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(be32(d, at)?) << 32 | u64::from(be32(d, at + 4)?))
}

/// TOC checksum flavour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checksum {
    /// No checksum.
    None,
    /// SHA-1.
    Sha1,
    /// MD5.
    Md5,
    /// SHA-256.
    Sha256,
    /// SHA-512 or unknown.
    Other(u32),
}

/// A parsed XAR header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Xar {
    /// Declared header size (usually 28; extra fields may extend it).
    pub header_size: u16,
    /// Format version.
    pub version: u16,
    /// Compressed TOC length in bytes.
    pub toc_compressed: u64,
    /// Uncompressed TOC length.
    pub toc_uncompressed: u64,
    /// TOC checksum id.
    pub checksum: Checksum,
}

impl Xar {
    /// File offset and length of the compressed TOC.
    pub fn toc(&self) -> Option<(usize, usize)> {
        Some((
            usize::from(self.header_size),
            usize::try_from(self.toc_compressed).ok()?,
        ))
    }
}

/// Parse an XAR header. Returns `None` on bad magic, a header size
/// below 28, or a TOC range that lies past the input.
pub fn parse(d: &[u8]) -> Option<Xar> {
    if be32(d, 0)? != MAGIC {
        return None;
    }
    let header_size = be16(d, 4)?;
    if usize::from(header_size) < HEADER || usize::from(header_size) > d.len() {
        return None;
    }
    let x = Xar {
        header_size,
        version: be16(d, 6)?,
        toc_compressed: be64(d, 8)?,
        toc_uncompressed: be64(d, 16)?,
        checksum: match be32(d, 24)? {
            0 => Checksum::None,
            1 => Checksum::Sha1,
            2 => Checksum::Md5,
            3 => Checksum::Sha256,
            c => Checksum::Other(c),
        },
    };
    let (at, len) = x.toc()?;
    if at.checked_add(len)? > d.len() {
        return None;
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(comp: u64, uncomp: u64, cksum: u32, total: usize) -> Vec<u8> {
        let mut d = vec![0u8; total];
        d[..4].copy_from_slice(b"xar!");
        d[4] = 0;
        d[5] = 28;
        d[6] = 0;
        d[7] = 1;
        for (i, v) in [comp, uncomp].iter().enumerate() {
            for j in 0..8 {
                d[8 + i * 8 + j] = (v >> ((7 - j) * 8)) as u8;
            }
        }
        for j in 0..4 {
            d[24 + j] = (cksum >> ((3 - j) * 8)) as u8;
        }
        d
    }

    #[test]
    fn fields() {
        let d = fixture(300, 900, 1, 400);
        let x = parse(&d).unwrap();
        assert_eq!(x.header_size, 28);
        assert_eq!(x.version, 1);
        assert_eq!(x.toc_compressed, 300);
        assert_eq!(x.toc_uncompressed, 900);
        assert_eq!(x.checksum, Checksum::Sha1);
        assert_eq!(x.toc(), Some((28, 300)));
    }

    #[test]
    fn checksum_kinds() {
        for (v, want) in [
            (0, Checksum::None),
            (2, Checksum::Md5),
            (3, Checksum::Sha256),
            (7, Checksum::Other(7)),
        ] {
            let d = fixture(10, 10, v, 64);
            assert_eq!(parse(&d).unwrap().checksum, want);
        }
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = fixture(10, 10, 0, 64);
        d[0] = b'X';
        assert!(parse(&d).is_none());
        // TOC past EOF
        assert!(parse(&fixture(300, 10, 0, 100)).is_none());
        // header_size < 28
        let mut d2 = fixture(10, 10, 0, 64);
        d2[5] = 8;
        assert!(parse(&d2).is_none());
    }
}
