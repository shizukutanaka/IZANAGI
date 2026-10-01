//! Apple Metal Library (`.metallib`): `MTLB` magic followed by a
//! `u32le` version-ish word and tagged metadata — modern containers
//! carry `TARG` (target triple string), `TYPE`, `MDCL`/`HASH` records,
//! ending with `ENDT`; function lists appear as `NAME`/`FNMD` tags.
//! Older `.metallib` payloads are LLVM bitcode (`0xDEC0 17B1` inside).
//!
//! ```
//! let mut d = b"MTLB".to_vec();
//! d.extend_from_slice(&[1, 0, 0, 0]); // abi
//! d.extend_from_slice(b"TARG");
//! d.extend_from_slice(&[26, 0, 0, 0]); // tag size incl. header
//! d.extend_from_slice(b"air64-apple-macosx");
//! let p = izanagi_kit::metallib::parse(&d).unwrap();
//! assert!(p.tags >= 1);
//! assert!(izanagi_kit::metallib::detect(&d));
//! ```

/// Census of a Metal library container.
#[derive(Debug, Clone, PartialEq)]
pub struct Metallib {
    /// Header word after `MTLB` (platform/abi selector).
    pub header_word: u32,
    /// Tagged metadata records walked.
    pub tags: u32,
    /// `TARG` target-triple tags.
    pub target_tags: u32,
    /// `TYPE` tags.
    pub type_tags: u32,
    /// `NAME`/`FNMD` function-name tags.
    pub name_tags: u32,
    /// `HASH`/`MDCL` digest tags.
    pub hash_tags: u32,
    /// `ENDT` terminator seen.
    pub has_endt: bool,
    /// `BITC`/`DE C0 17 0B` bitcode tag seen.
    pub has_bitcode: bool,
    /// Payload bytes covered by walked tags.
    pub payload_len: u32,
    /// A tag length overran the buffer.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the `MTLB` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8 && b[..4] == *b"MTLB"
}

/// Census; `None` without `MTLB`. Tags are `FOURCC` + `u32le`
/// size (size includes the 8-byte tag header) walked from offset 8.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Metallib> {
    if !detect(b) {
        return None;
    }
    let mut m = Metallib {
        header_word: le32(b, 4),
        tags: 0,
        target_tags: 0,
        type_tags: 0,
        name_tags: 0,
        hash_tags: 0,
        has_endt: false,
        has_bitcode: b.windows(4).any(|w| w == [0xDE, 0xC0, 0x17, 0x0B]),
        payload_len: 0,
        truncated: false,
    };
    // Metadata tags live after the 20-byte fixed header on modern
    // libraries; legacy ones keep a u64 then the tag stream. Try 8.
    let mut i = 8usize;
    let mut steps = 0;
    while i + 8 <= b.len() && steps < 4096 {
        steps += 1;
        let tag = &b[i..i + 4];
        let size = le32(b, i + 4) as usize;
        if !tag
            .iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            break;
        }
        if size < 8 || i + size > b.len() {
            m.truncated = true;
            break;
        }
        match tag {
            b"TARG" => m.target_tags += 1,
            b"TYPE" => m.type_tags += 1,
            b"NAME" | b"FNMD" => m.name_tags += 1,
            b"HASH" | b"MDCL" => m.hash_tags += 1,
            b"ENDT" => m.has_endt = true,
            b"BITC" => m.has_bitcode = true,
            _ => {}
        }
        m.tags += 1;
        m.payload_len += size as u32;
        i += size;
        if m.has_endt {
            break;
        }
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(d: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) {
        d.extend_from_slice(fourcc);
        d.extend_from_slice(&((8 + payload.len()) as u32).to_le_bytes());
        d.extend_from_slice(payload);
    }

    fn fixture() -> Vec<u8> {
        let mut d = b"MTLB".to_vec();
        d.extend_from_slice(&[1, 0, 0, 0]);
        tag(&mut d, b"TARG", b"air64-apple-macosx14.0.0");
        tag(&mut d, b"NAME", b"vertexMain");
        tag(&mut d, b"ENDT", &[]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"MTLB"));
        assert!(!detect(b"MTLF0000"));
        assert!(!detect(b"MTLX123456789"));
    }

    #[test]
    fn parses_tags() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.header_word, 1);
        assert_eq!(p.tags, 3);
        assert_eq!(p.target_tags, 1);
        assert_eq!(p.name_tags, 1);
        assert!(p.has_endt);
        assert!(!p.truncated);
    }

    #[test]
    fn truncated_tag_flagged() {
        let mut d = b"MTLB".to_vec();
        d.extend_from_slice(&[0; 4]);
        d.extend_from_slice(b"TARG");
        d.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x7F]);
        let p = parse(&d).unwrap();
        assert!(p.truncated);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not a library").is_none());
    }
}
