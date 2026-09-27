//! zlib — RFC 1950's two-byte prelude: CMF (low nibble = method, 8 =
//! deflate; high nibble = `CINFO`, window = `2^(CINFO+8)`, ≤ 7) and
//! FLG (bits 7–6 = FLEVEL, bit 5 = FDICT, low 5 bits = FCHECK chosen
//! so `(CMF*256 + FLG) % 31 == 0`). When FDICT is set a u32 dictionary
//! Adler32 follows; the stream ends with the data's Adler32.
//!
//! ```
//! use izanagi_kit::zlib::parse;
//! let mut d = vec![0x78, 0x9C];      // deflate, 32K window, FLEVEL 2
//! d.extend_from_slice(b"body");
//! d.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]); // adler32, big-endian
//! let z = parse(&d).unwrap();
//! assert_eq!(z.cinfo, 7);
//! assert_eq!(z.flevel, 2);
//! ```

/// A parsed zlib prelude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zlib {
    /// Compression method (8 = deflate).
    pub method: u8,
    /// `CINFO` — window = `2^(cinfo + 8)` bytes, spec-capped at 7.
    pub cinfo: u8,
    /// `FLEVEL` 0–3 (fastest … best).
    pub flevel: u8,
    /// `FDICT` — a preset dictionary follows.
    pub fdict: bool,
    /// Byte offset where the deflate body begins (8 with FDICT).
    pub data_at: usize,
}

/// Window size in bytes for a `CINFO` value.
pub fn window_size(cinfo: u8) -> u32 {
    1u32 << (u32::from(cinfo) + 8)
}

/// Parse the prelude; `None` on bad method, `cinfo > 7`, or a failed
/// `FCHECK` (`(CMF*256 + FLG) % 31 != 0`).
pub fn parse(d: &[u8]) -> Option<Zlib> {
    let cmf = *d.first()?;
    let flg = *d.get(1)?;
    let method = cmf & 0x0F;
    let cinfo = cmf >> 4;
    if method != 8 || cinfo > 7 {
        return None;
    }
    if (u32::from(cmf) * 256 + u32::from(flg)) % 31 != 0 {
        return None;
    }
    let fdict = flg & 0x20 != 0;
    let data_at = if fdict {
        d.get(2..6)?; // dictionary id must be present
        6
    } else {
        2
    };
    Some(Zlib {
        method,
        cinfo,
        flevel: flg >> 6,
        fdict,
        data_at,
    })
}

/// The preset-dictionary Adler32 when `FDICT` is set.
pub fn dict_id(d: &[u8]) -> Option<u32> {
    let r = d.get(2..6)?;
    Some(
        (u32::from(r[0]) << 24)
            | (u32::from(r[1]) << 16)
            | (u32::from(r[2]) << 8)
            | u32::from(r[3]),
    )
}

/// Byte offset of the trailing Adler32 — the stream's last 4 bytes.
pub fn adler_at(z: &Zlib, d: &[u8]) -> Option<usize> {
    d.len().checked_sub(4).filter(|&t| t >= z.data_at)
}

/// The trailing Adler32 value.
pub fn adler(z: &Zlib, d: &[u8]) -> Option<u32> {
    let at = adler_at(z, d)?;
    let r = d.get(at..at + 4)?;
    Some(
        (u32::from(r[0]) << 24)
            | (u32::from(r[1]) << 16)
            | (u32::from(r[2]) << 8)
            | u32::from(r[3]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0x78, 0x9C];
        d.extend_from_slice(b"BODY");
        d.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]); // adler32, big-endian
        d
    }

    #[test]
    fn parses_prelude() {
        let d = fixture();
        let z = parse(&d).unwrap();
        assert_eq!(z.method, 8);
        assert_eq!(z.cinfo, 7);
        assert_eq!(window_size(z.cinfo), 32768);
        assert_eq!(z.flevel, 2);
        assert!(!z.fdict);
        assert_eq!(z.data_at, 2);
        assert_eq!(&d[z.data_at..adler_at(&z, &d).unwrap()], b"BODY");
        assert_eq!(adler(&z, &d), Some(0x1122_3344));
    }

    #[test]
    fn fdict() {
        // find a valid FLG with FDICT for CMF 0x78
        let flg = (0..256u32)
            .find(|f| (0x78 * 256 + f) % 31 == 0 && f & 0x20 != 0)
            .unwrap() as u8;
        let mut d = vec![0x78, flg];
        d.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        d.extend_from_slice(b"x");
        let z = parse(&d).unwrap();
        assert!(z.fdict);
        assert_eq!(z.data_at, 6);
        assert_eq!(dict_id(&d), Some(0xDEAD_BEEF));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x78").is_none());
        assert!(parse(b"\x79\x9C").is_none()); // method 9
        assert!(parse(b"\x88\x9C").is_none()); // cinfo 8
        assert!(parse(b"\x78\x9D").is_none()); // bad FCHECK
        assert!(parse(b"\x78\x20").is_none()); // FDICT set but no dict bytes? 0x7820%31==0, fdict set, len<6
    }
}
