//! DVI (DeVice Independent, TeX82) / XDV (XeTeX extended):
//! `pre` opcode `0xF7` + version id (2 = DVI, 3 = pTeX DVI,
//! 7 = XDV), `num`/`den`/`mag` conversion triple, `k`-byte
//! comment, then page records, `post` (`0xF8`) and
//! `post_post` (`0xF9`) + `0xDF` padding tail.
//!
//! ```
//! let mut d = vec![0xF7, 2];
//! d.extend_from_slice(&[0x01, 0x83, 0x92, 0xC0]); // num
//! d.extend_from_slice(&[0x1C, 0x3B, 0x00, 0x00]); // den
//! d.extend_from_slice(&[0x00, 0x00, 0x03, 0xE8]); // mag
//! d.push(0);
//! let v = izanagi_kit::dvi::parse(&d).unwrap();
//! assert_eq!(v.version, 2);
//! assert!(!v.xdv);
//! assert_eq!(v.mag, 1000);
//! assert!(izanagi_kit::dvi::detect(&d));
//! ```

/// Census of a DVI/XDV stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Dvi {
    /// Version id byte (2 = DVI, 3 = pTeX, 7 = XDV).
    pub version: u8,
    /// `num` numerator of the conversion ratio.
    pub num: u32,
    /// `den` denominator of the conversion ratio.
    pub den: u32,
    /// Magnification ×1000.
    pub mag: u32,
    /// Comment bytes after `mag`.
    pub comment_len: u32,
    /// `post.s` — pages actually present.
    pub pages: u32,
    /// `post.t` — `bop` commands seen.
    pub bops: u32,
    /// `post.l` — tallest page height+depth (scaled points).
    pub max_page_dim: u32,
    /// `post.u` — maximum stack depth reached.
    pub max_stack: u32,
    /// `post_post` found and consistent with `pre`.
    pub has_postamble: bool,
    /// Version 7 → XDV (XeTeX).
    pub xdv: bool,
}

fn be32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}

fn be16(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 8) | b[i + 1] as u32
}

fn version_ok(id: u8) -> bool {
    matches!(id, 2 | 3 | 7)
}

/// `true` on a `0xF7` + known version id preamble.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 15 && b[0] == 0xF7 && version_ok(b[1]) && b.len() >= 15 + b[14] as usize
}

/// Census; `None` without the preamble.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dvi> {
    if !detect(b) {
        return None;
    }
    let num = be32(b, 2);
    let den = be32(b, 6);
    let mag = be32(b, 10);
    let k = b[14] as usize;
    let mut v = Dvi {
        version: b[1],
        num,
        den,
        mag,
        comment_len: k as u32,
        pages: 0,
        bops: 0,
        max_page_dim: 0,
        max_stack: 0,
        has_postamble: false,
        xdv: b[1] == 7,
    };
    // post_post: 0xF9, q[4], i[1], then >=4 bytes of 0xDF.
    let mut j = b.len();
    while j > 0 && b[j - 1] == 0xDF {
        j -= 1;
    }
    if j >= 6 && b[j - 6] == 0xF9 {
        let q = be32(b, j - 5) as usize;
        let id = b[j - 1];
        if id == b[1] && q + 27 <= b.len() && b[q] == 0xF8 {
            let (pn, pd, pm) = (be32(b, q + 5), be32(b, q + 9), be32(b, q + 13));
            if pn == num && pd == den && pm == mag {
                v.has_postamble = true;
                v.max_page_dim = be32(b, q + 17);
                v.max_stack = be32(b, q + 21);
                v.pages = be16(b, q + 25);
                v.bops = be16(b, q + 27 - 2 + 2);
            }
        }
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0xF7, 2];
        d.extend_from_slice(&[0x01, 0x83, 0x92, 0xC0]); // num
        d.extend_from_slice(&[0x1C, 0x3B, 0x00, 0x00]); // den
        d.extend_from_slice(&[0x00, 0x00, 0x03, 0xE8]); // mag = 1000
        d.push(3);
        d.extend_from_slice(b"one"); // comment
                                     // fake body byte + post at offset q
        let q = d.len();
        d.push(0xF8);
        d.extend_from_slice(&[0x00, 0x00, 0x00, 0x0F]); // p
        d.extend_from_slice(&[0x01, 0x83, 0x92, 0xC0]); // num
        d.extend_from_slice(&[0x1C, 0x3B, 0x00, 0x00]); // den
        d.extend_from_slice(&[0x00, 0x00, 0x03, 0xE8]); // mag
        d.extend_from_slice(&[0x00, 0x12, 0xD6, 0x7C]); // l
        d.extend_from_slice(&[0x00, 0x00, 0x00, 0x05]); // u
        d.extend_from_slice(&[0x00, 0x02]); // s = 2 pages
        d.extend_from_slice(&[0x00, 0x02]); // t = 2 bops
        d.push(0xF9);
        d.extend_from_slice(&(q as u32).to_be_bytes());
        d.push(2);
        d.extend_from_slice(&[0xDF, 0xDF, 0xDF, 0xDF]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"plain text"));
        assert!(!detect(&[0xF7, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn parses() {
        let v = parse(&fixture()).unwrap();
        assert_eq!(v.version, 2);
        assert_eq!(v.num, 0x0183_92C0);
        assert_eq!(v.den, 0x1C3B_0000);
        assert_eq!(v.mag, 1000);
        assert_eq!(v.comment_len, 3);
        assert_eq!(v.pages, 2);
        assert_eq!(v.bops, 2);
        assert_eq!(v.max_page_dim, 0x0012_D67C);
        assert_eq!(v.max_stack, 5);
        assert!(v.has_postamble);
        assert!(!v.xdv);
    }

    #[test]
    fn header_only() {
        let mut d = vec![0xF7, 7, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 3, 0xE8, 0];
        let v = parse(&d).unwrap();
        assert!(v.xdv);
        assert!(!v.has_postamble);
        d[1] = 4;
        assert!(parse(&d).is_none());
    }
}
