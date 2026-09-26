//! RPM package lead + header-structure envelope.
//!
//! A `.rpm` opens with a 96-byte *lead*: `ED AB EE DB`, version,
//! type (0 binary / 1 source), arch and os numbers, a 66-byte
//! name field and a signature type. It is followed by *header*
//! structures — `8E A8 E8 01` + 7 bytes + `nindex`/`hlen` (u32
//! BE) — first the signature header, then the main header (8-byte
//! aligned). `parse` reports the lead and locates both headers.
//!
//! ```
//! use izanagi_kit::rpm::{parse, LEAD, LEAD_MAGIC, HEADER_MAGIC};
//!
//! let mut d = vec![0u8; LEAD + 32];
//! d[..4].copy_from_slice(&LEAD_MAGIC);
//! d[4] = 3; d[5] = 0;           // version 3.0
//! d[7] = 1;                     // binary package
//! d[9] = 1;                     // i386
//! d[10..15].copy_from_slice(b"pkg-1");
//! d[77] = 1;                    // linux
//! d[79] = 5;                    // RPMSIGTYPE
//! // signature header at 96
//! d[96..99].copy_from_slice(&HEADER_MAGIC);
//! d[107] = 2; // nindex = 2 (u32 BE at 104)
//! d[111] = 16; // hlen = 16 (u32 BE at 108)
//! // signature body: 2*16 index + 16 store = 48 bytes at 112..160
//! d.resize(160, 0);
//! let r = parse(&d).unwrap();
//! assert_eq!(r.package_type, 1);
//! assert_eq!(r.name, "pkg-1");
//! assert_eq!(r.sig_count, 2);
//! ```

/// Lead size.
pub const LEAD: usize = 96;
/// Lead magic.
pub const LEAD_MAGIC: [u8; 4] = [0xed, 0xab, 0xee, 0xdb];
/// Header-structure magic (first 3 bytes of an 8-byte prologue).
pub const HEADER_MAGIC: [u8; 3] = [0x8e, 0xad, 0xe8];

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

/// A header structure (signature or main).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// File offset of the `8E A8 E8 01` prologue.
    pub at: usize,
    /// Number of 16-byte index entries.
    pub index_count: u32,
    /// Byte length of the data store following the index.
    pub store_len: u32,
}

impl Header {
    /// File offset just past the header (index + store).
    pub fn end(&self) -> usize {
        self.at + 16 + self.index_count as usize * 16 + self.store_len as usize
    }
}

/// Parse one header structure at `at`.
fn header(d: &[u8], at: usize) -> Option<Header> {
    if d.get(at..at + 3)? != HEADER_MAGIC.as_slice() {
        return None;
    }
    let index_count = be32(d, at + 8)?;
    let store_len = be32(d, at + 12)?;
    let h = Header {
        at,
        index_count,
        store_len,
    };
    if h.end() > d.len() {
        return None;
    }
    Some(h)
}

/// A parsed RPM envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rpm {
    /// RPM lead major/minor version.
    pub version: (u8, u8),
    /// Package type (0 binary, 1 source).
    pub package_type: u16,
    /// Architecture number.
    pub arch: u16,
    /// Lead name field (NUL-trimmed).
    pub name: String,
    /// OS number.
    pub os: u16,
    /// Signature type.
    pub sig_type: u16,
    /// Signature header.
    pub signature: Header,
    /// Number of index entries in the signature header.
    pub sig_count: u32,
    /// Main header, when present and reachable (8-byte aligned
    /// after the signature).
    pub main: Option<Header>,
}

/// Parse the lead + signature + main headers. Returns `None` on a
/// bad magic or truncated structures.
pub fn parse(d: &[u8]) -> Option<Rpm> {
    if d.get(..4)? != LEAD_MAGIC.as_slice() {
        return None;
    }
    if d.len() < LEAD {
        return None;
    }
    let name_raw = d.get(10..76)?;
    let end = name_raw
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_raw.len());
    let signature = header(d, LEAD)?;
    // main header follows the signature, padded to 8 bytes
    let sig_end = signature.end();
    let main_at = sig_end.div_ceil(8) * 8;
    let main = header(d, main_at);
    Some(Rpm {
        version: (d.get(4).copied()?, d.get(5).copied()?),
        package_type: be16(d, 6)?,
        arch: be16(d, 8)?,
        name: core::str::from_utf8(&name_raw[..end])
            .unwrap_or("")
            .to_string(),
        os: be16(d, 76)?,
        sig_type: be16(d, 78)?,
        signature,
        sig_count: signature.index_count,
        main,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(nindex: u32, hlen: u32, with_main: bool) -> Vec<u8> {
        let sig_end = LEAD + 16 + nindex as usize * 16 + hlen as usize;
        let main_at = sig_end.div_ceil(8) * 8;
        let total = if with_main {
            main_at + 16 + 2 * 16 + 8 // main header: 2 entries + 8B store
        } else {
            sig_end
        };
        let mut d = vec![0u8; total];
        d[..4].copy_from_slice(&LEAD_MAGIC);
        d[4] = 4;
        d[5] = 11;
        d[7] = 1;
        d[9] = 1;
        d[10..18].copy_from_slice(b"test-pkg");
        d[77] = 1;
        d[79] = 5;
        d[96..99].copy_from_slice(&HEADER_MAGIC);
        let w = |d: &mut [u8], o: usize, v: u32| {
            d[o] = (v >> 24) as u8;
            d[o + 1] = (v >> 16) as u8;
            d[o + 2] = (v >> 8) as u8;
            d[o + 3] = v as u8;
        };
        w(&mut d, 96 + 8, nindex);
        w(&mut d, 96 + 12, hlen);
        if with_main {
            d[main_at..main_at + 3].copy_from_slice(&HEADER_MAGIC);
            w(&mut d, main_at + 8, 2);
            w(&mut d, main_at + 12, 8);
        }
        d
    }

    #[test]
    fn lead_and_headers() {
        let d = fixture(3, 48, true);
        let r = parse(&d).unwrap();
        assert_eq!(r.version, (4, 11));
        assert_eq!(r.package_type, 1);
        assert_eq!(r.arch, 1);
        assert_eq!(r.name, "test-pkg");
        assert_eq!(r.os, 1);
        assert_eq!(r.sig_type, 5);
        assert_eq!(r.sig_count, 3);
        assert_eq!(r.signature.store_len, 48);
        let m = r.main.unwrap();
        assert_eq!(m.index_count, 2);
        assert_eq!(m.store_len, 8);
    }

    #[test]
    fn no_main() {
        let d = fixture(1, 8, false);
        let r = parse(&d).unwrap();
        assert!(r.main.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture(0, 0, false);
        d[0] = 0;
        assert!(parse(&d).is_none());
        let mut d2 = fixture(2, 4096, false); // signature body truncated
        d2.truncate(LEAD + 20);
        assert!(parse(&d2).is_none());
    }
}
