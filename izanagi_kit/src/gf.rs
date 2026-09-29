//! GF (Generic Font, Metafont): `pre` opcode `0xF7` + id
//! `131` + `k`-byte comment; char packets; `post` (`0xF8`)
//! carries `p`, `ds`, `cs`, `hppp`, `vppp`, bounding box
//! `min_m..max_n`; `post_post` (`0xF9`) + `q` + id + `0xDF`
//! padding, exactly like DVI's tail.
//!
//! ```
//! let mut d = vec![0xF7, 131, 0]; // pre, empty comment
//! d.extend_from_slice(&[0xF8]);
//! d.extend_from_slice(&[0; 36]); // p ds cs hppp vppp mm Mm mn Mn
//! d.extend_from_slice(&[0xF9]);
//! d.extend_from_slice(&[0, 0, 0, 3]); // q -> post
//! d.extend_from_slice(&[131, 0xDF, 0xDF, 0xDF, 0xDF]);
//! let g = izanagi_kit::gf::parse(&d).unwrap();
//! assert!(g.has_post);
//! assert!(g.has_post_post);
//! assert_eq!(g.id, 131);
//! assert!(izanagi_kit::gf::detect(&d));
//! ```

/// Census of a GF bitmap font.
#[derive(Debug, Clone, PartialEq)]
pub struct Gf {
    /// Id byte (131).
    pub id: u8,
    /// Comment bytes after `pre`.
    pub comment_len: u32,
    /// `post.ds` — design size fixword.
    pub design_size: u32,
    /// `post.cs` — checksum.
    pub checksum: u32,
    /// `post.hppp` — horizontal pixels per point.
    pub hppp: u32,
    /// `post.vppp` — vertical pixels per point.
    pub vppp: u32,
    /// Bounding box min/max m (columns).
    pub min_m: u32,
    /// `max_m`.
    pub max_m: u32,
    /// `min_n`/`max_n` (rows).
    pub min_n: u32,
    /// `max_n`.
    pub max_n: u32,
    /// `post` record found at `post_post.q`.
    pub has_post: bool,
    /// `post_post` trailer found and id-matched.
    pub has_post_post: bool,
    /// Bytes between preamble and `post` (char packets).
    pub payload_len: u32,
}

fn be32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}

/// `true` on `0xF7 0x83` with room for the comment length.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 3 && b[0] == 0xF7 && b[1] == 131 && b.len() >= 3 + b[2] as usize
}

/// Census; `None` without the `0xF7 131` preamble.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Gf> {
    if !detect(b) {
        return None;
    }
    let k = b[2] as usize;
    let mut g = Gf {
        id: b[1],
        comment_len: k as u32,
        design_size: 0,
        checksum: 0,
        hppp: 0,
        vppp: 0,
        min_m: 0,
        max_m: 0,
        min_n: 0,
        max_n: 0,
        has_post: false,
        has_post_post: false,
        payload_len: 0,
    };
    // post_post: 0xF9, q[4], i[1], 0xDF padding to EOF.
    let mut j = b.len();
    while j > 0 && b[j - 1] == 0xDF {
        j -= 1;
    }
    if j >= 6 && b[j - 6] == 0xF9 && b[j - 1] == 131 {
        g.has_post_post = true;
        let q = be32(b, j - 5) as usize;
        // post: 0xF8 + p(4) + ds cs hppp vppp mm Mm mn Mn (8×4) = 37 B.
        if q + 37 <= b.len() && b[q] == 0xF8 {
            g.has_post = true;
            g.design_size = be32(b, q + 5);
            g.checksum = be32(b, q + 9);
            g.hppp = be32(b, q + 13);
            g.vppp = be32(b, q + 17);
            g.min_m = be32(b, q + 21);
            g.max_m = be32(b, q + 25);
            g.min_n = be32(b, q + 29);
            g.max_n = be32(b, q + 33);
            g.payload_len = (q.saturating_sub(3 + k)) as u32;
        }
    }
    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0xF7, 131, 2, b'g', b'f']; // comment "gf"
        d.extend_from_slice(&[1, 2, 3]); // fake char packets
        let q = d.len();
        d.push(0xF8);
        d.extend_from_slice(&[0, 0, 0, 0]); // p
        d.extend_from_slice(&[0x00, 0x0A, 0x00, 0x00]); // ds
        d.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // cs
        d.extend_from_slice(&[0x00, 0x02, 0x8C, 0x00]); // hppp
        d.extend_from_slice(&[0x00, 0x02, 0x8C, 0x00]); // vppp
        d.extend_from_slice(&[0, 0, 0, 0]); // min_m
        d.extend_from_slice(&[0, 0, 0, 7]); // max_m
        d.extend_from_slice(&[0, 0, 0, 0]); // min_n
        d.extend_from_slice(&[0, 0, 0, 9]); // max_n
        d.push(0xF9);
        d.extend_from_slice(&(q as u32).to_be_bytes());
        d.push(131);
        d.extend_from_slice(&[0xDF, 0xDF, 0xDF, 0xDF]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"\xF7\x02"));
        assert!(!detect(b"gf"));
    }

    #[test]
    fn parses() {
        let g = parse(&fixture()).unwrap();
        assert_eq!(g.id, 131);
        assert_eq!(g.comment_len, 2);
        assert_eq!(g.design_size, 0x000A_0000);
        assert_eq!(g.checksum, 0xCAFE_BABE);
        assert_eq!(g.hppp, 0x0002_8C00);
        assert_eq!(g.max_m, 7);
        assert_eq!(g.max_n, 9);
        assert!(g.has_post);
        assert!(g.has_post_post);
        assert_eq!(g.payload_len, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"\xF7\x84\x00").is_none());
        assert!(parse(b"").is_none());
    }
}
