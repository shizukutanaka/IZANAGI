//! VF (Virtual Font, vftovp/vptovf): `0xF7` + id `202` +
//! `k`-byte comment + `cs`/`ds`, then packets — char packets
//! (`flag < 242` ⇒ `pl = flag`; `242` ⇒ `pl[4] cc[4]`),
//! `fnt_def1..4` (`243–246`) like DVI's, `post` (`248`)
//! terminator.
//!
//! ```
//! let mut d = vec![0xF7, 202, 0]; // pre, empty comment
//! d.extend_from_slice(&[0x00, 0x0A, 0x00, 0x00]); // cs
//! d.extend_from_slice(&[0x00, 0x0A, 0x00, 0x00]); // ds
//! d.extend_from_slice(&[1, 65, 0x8B]); // char: pl=1, cc=65, 1 dvi byte
//! d.push(248); // post
//! let v = izanagi_kit::vf::parse(&d).unwrap();
//! assert_eq!(v.chars, 1);
//! assert!(v.has_post);
//! assert!(izanagi_kit::vf::detect(&d));
//! ```

/// Census of a VF file.
#[derive(Debug, Clone, PartialEq)]
pub struct Vf {
    /// Id byte (202).
    pub id: u8,
    /// Comment bytes after `pre`.
    pub comment_len: u32,
    /// `cs` — checksum.
    pub checksum: u32,
    /// `ds` — design size fixword.
    pub design_size: u32,
    /// Char packets walked (`flag < 242` + `242` long form).
    pub chars: u32,
    /// `242` long-form char packets.
    pub long_chars: u32,
    /// `fnt_def` packets (243–246).
    pub font_defs: u32,
    /// `post` (`248`) terminator found.
    pub has_post: bool,
    /// Packet walk stopped early (truncated packet).
    pub truncated: bool,
}

fn be32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}

/// `true` on `0xF7 0xCA` with room for the preamble tail.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    // pre(3) + k + cs ds (8).
    b.len() >= 11 && b[0] == 0xF7 && b[1] == 202 && b.len() >= 11 + b[2] as usize
}

/// Census; `None` without the `0xF7 202` preamble.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Vf> {
    if !detect(b) {
        return None;
    }
    let k = b[2] as usize;
    let mut v = Vf {
        id: b[1],
        comment_len: k as u32,
        checksum: be32(b, 3 + k),
        design_size: be32(b, 7 + k),
        chars: 0,
        long_chars: 0,
        font_defs: 0,
        has_post: false,
        truncated: false,
    };
    let mut i = 11 + k;
    while i < b.len() {
        let op = b[i];
        match op {
            248 => {
                v.has_post = true;
                break;
            }
            242 => {
                // long_char: pl(4) cc(4) dvi[pl]
                if i + 9 > b.len() {
                    v.truncated = true;
                    break;
                }
                let pl = be32(b, i + 1) as usize;
                if i + 9 + pl > b.len() {
                    v.truncated = true;
                    break;
                }
                v.long_chars += 1;
                v.chars += 1;
                i += 9 + pl;
            }
            243..=246 => {
                // fnt_def: font_num[op-242] cs(4) s(4) d(4) a(1) l(1) name[a+l]
                let num_len = (op - 242) as usize;
                if i + 1 + num_len + 14 > b.len() {
                    v.truncated = true;
                    break;
                }
                let a = b[i + 1 + num_len + 12] as usize;
                let l = b[i + 1 + num_len + 13] as usize;
                if i + 1 + num_len + 14 + a + l > b.len() {
                    v.truncated = true;
                    break;
                }
                v.font_defs += 1;
                i += 1 + num_len + 14 + a + l;
            }
            _ => {
                // short char packet: pl=flag, cc(1), dvi[pl]
                let pl = op as usize;
                if i + 2 + pl > b.len() {
                    v.truncated = true;
                    break;
                }
                v.chars += 1;
                i += 2 + pl;
            }
        }
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0xF7, 202, 1, b'v']; // comment "v"
        d.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]); // cs
        d.extend_from_slice(&[0x00, 0x0A, 0x00, 0x00]); // ds
                                                        // fnt_def1: num(1) cs s d a l name
        d.push(243);
        d.push(1); // font_num
        d.extend_from_slice(&[0; 12]); // cs s d
        d.push(0); // a
        d.push(4); // l
        d.extend_from_slice(b"cmr1");
        // char packets
        d.extend_from_slice(&[2, 66, 0x8B, 0x8C]); // pl=2 cc=66 + 2 dvi
        d.push(242); // long_char
        d.extend_from_slice(&[0, 0, 0, 3]); // pl=3
        d.extend_from_slice(&[0, 0, 0, 67]); // cc=67
        d.extend_from_slice(&[0x8B, 0x8C, 0x8E]);
        d.push(248); // post
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"\xF7\x02"));
        assert!(!detect(b"vf"));
    }

    #[test]
    fn parses() {
        let v = parse(&fixture()).unwrap();
        assert_eq!(v.id, 202);
        assert_eq!(v.comment_len, 1);
        assert_eq!(v.checksum, 0x1234_5678);
        assert_eq!(v.design_size, 0x000A_0000);
        assert_eq!(v.chars, 2);
        assert_eq!(v.long_chars, 1);
        assert_eq!(v.font_defs, 1);
        assert!(v.has_post);
        assert!(!v.truncated);
    }

    #[test]
    fn truncated_packet() {
        let mut d = fixture();
        d.truncate(d.len() - 5); // cut inside long_char
        let v = parse(&d).unwrap();
        assert!(v.truncated);
        assert!(!v.has_post);
    }
}
