//! PK (packed bitmap font, gftopk/pktopx): `pk_pre` `0xF7` +
//! id `89` + `k`-byte comment + `ds`/`cs`/`hppp`/`vppp`;
//! char packets (`flag < 240`), specials `pk_xxx1-4`
//! (`240–243`), `pk_yyy` (`244`), `pk_post` (`245`), `pk_nop`
//! (`246`) padding.
//!
//! ```
//! let mut d = vec![0xF7, 89, 0]; // pre, empty comment
//! d.extend_from_slice(&[0, 0x0A, 0x00, 0x00]); // ds
//! d.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]); // cs
//! d.extend_from_slice(&[0, 0, 0x02, 0x8C]); // hppp
//! d.extend_from_slice(&[0, 0, 0x02, 0x8C]); // vppp
//! d.extend_from_slice(&[245, 246, 246]); // post + nop pad
//! let p = izanagi_kit::pk::parse(&d).unwrap();
//! assert_eq!(p.id, 89);
//! assert!(p.has_post);
//! assert_eq!(p.nop_pad, 2);
//! assert!(izanagi_kit::pk::detect(&d));
//! ```

/// Census of a PK file.
#[derive(Debug, Clone, PartialEq)]
pub struct Pk {
    /// Id byte (89).
    pub id: u8,
    /// Comment bytes after `pk_pre`.
    pub comment_len: u32,
    /// `ds` — design size fixword.
    pub design_size: u32,
    /// `cs` — checksum.
    pub checksum: u32,
    /// `hppp` — horizontal pixels per point.
    pub hppp: u32,
    /// `vppp` — vertical pixels per point.
    pub vppp: u32,
    /// `pk_post` (`245`) terminator found.
    pub has_post: bool,
    /// Trailing `pk_nop` (`246`) bytes before EOF.
    pub nop_pad: u32,
    /// Bytes between the 19-byte preamble tail and the
    /// postamble (char packets + specials).
    pub payload_len: u32,
}

fn be32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}

/// `true` on `0xF7 0x59` with a complete preamble.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    // pre(3) + k + ds cs hppp vppp (16).
    b.len() >= 19 && b[0] == 0xF7 && b[1] == 89 && b.len() >= 19 + b[2] as usize
}

/// Census; `None` without the `0xF7 89` preamble.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Pk> {
    if !detect(b) {
        return None;
    }
    let k = b[2] as usize;
    let base = 3 + k;
    let mut p = Pk {
        id: b[1],
        comment_len: k as u32,
        design_size: be32(b, base),
        checksum: be32(b, base + 4),
        hppp: be32(b, base + 8),
        vppp: be32(b, base + 12),
        has_post: false,
        nop_pad: 0,
        payload_len: 0,
    };
    // postamble: strip trailing pk_nop (246) then expect pk_post (245).
    let mut j = b.len();
    while j > 0 && b[j - 1] == 246 {
        j -= 1;
    }
    p.nop_pad = (b.len() - j) as u32;
    if j > base + 16 && b[j - 1] == 245 {
        p.has_post = true;
        p.payload_len = (j - 1 - (base + 16)) as u32;
    } else {
        p.payload_len = (b.len() - (base + 16)) as u32;
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0xF7, 89, 1, b'p']; // comment "p"
        d.extend_from_slice(&[0x00, 0x0A, 0x00, 0x00]); // ds
        d.extend_from_slice(&[0xDE, 0xAD, 0x00, 0x01]); // cs
        d.extend_from_slice(&[0x00, 0x00, 0x02, 0x8C]); // hppp
        d.extend_from_slice(&[0x00, 0x00, 0x02, 0x8C]); // vppp
        d.extend_from_slice(&[9, 9, 9]); // fake char packets
        d.extend_from_slice(&[245, 246, 246, 246]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"\xF7\x02"));
        assert!(!detect(b"pk file"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.id, 89);
        assert_eq!(p.comment_len, 1);
        assert_eq!(p.design_size, 0x000A_0000);
        assert_eq!(p.checksum, 0xDEAD_0001);
        assert_eq!(p.hppp, 0x0000_028C);
        assert!(p.has_post);
        assert_eq!(p.nop_pad, 3);
        assert_eq!(p.payload_len, 3);
    }

    #[test]
    fn no_postamble() {
        let mut d = vec![0xF7, 89, 0];
        d.extend_from_slice(&[0; 16]);
        let p = parse(&d).unwrap();
        assert!(!p.has_post);
        assert_eq!(p.payload_len, 0);
    }
}
