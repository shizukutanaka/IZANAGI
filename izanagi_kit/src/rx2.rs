//! Propellerhead REX / REX2 loop files — IFF `FORM` container with `REX `/`REX2` types.
//!
//! ```
//! let mut d = b"FORM\x00\x00\x00\x10REX2".to_vec();
//! d.extend_from_slice(b"HEAD\x00\x00\x00\x04\x01\x02\x03\x04");
//! let r = izanagi_kit::rx2::parse(&d).unwrap();
//! assert_eq!(r.version, 2);
//! assert_eq!(r.chunks, vec!["HEAD"]);
//! assert!(izanagi_kit::rx2::detect(&d));
//! ```

/// Parsed REX/REX2 container summary.
#[derive(Debug, Clone)]
pub struct Rx2 {
    /// 1 for `REX `, 2 for `REX2`.
    pub version: u8,
    /// Declared FORM payload size.
    pub form_size: u32,
    /// Chunk ids in order.
    pub chunks: Vec<String>,
    /// Total chunk data bytes.
    pub data_bytes: u32,
    /// Truncated trailing bytes.
    pub trailing: usize,
}

fn be32(b: &[u8], o: usize) -> u32 {
    ((b[o] as u32) << 24) | ((b[o + 1] as u32) << 16) | ((b[o + 2] as u32) << 8) | (b[o + 3] as u32)
}

/// Detects `FORM` + `REX ` or `REX2`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 12 && b.starts_with(b"FORM") && (&b[8..12] == b"REX " || &b[8..12] == b"REX2")
}

/// Parses a REX container; `None` without the signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Rx2> {
    if !detect(b) {
        return None;
    }
    let mut f = Rx2 {
        version: if b[11] == b'2' { 2 } else { 1 },
        form_size: be32(b, 4),
        chunks: Vec::new(),
        data_bytes: 0,
        trailing: 0,
    };
    let mut i = 12usize;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        if !id.iter().all(|&c| (0x20..0x7f).contains(&c)) {
            break;
        }
        f.chunks.push(String::from_utf8_lossy(id).into_owned());
        let size = be32(b, i + 4) as usize;
        if i + 8 + size > b.len() {
            f.trailing = b.len() - i;
            break;
        }
        f.data_bytes = f.data_bytes.saturating_add(size as u32);
        i += 8 + size + (size & 1);
    }
    if i < b.len() && f.trailing == 0 {
        f.trailing = b.len() - i;
    }
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"FORM\x00\x00\x00\x1cREX2".to_vec();
        d.extend_from_slice(b"HEAD\x00\x00\x00\x04\x01\x02\x03\x04");
        d.extend_from_slice(b"SLIC\x00\x00\x00\x03abc\x00");
        d
    }

    #[test]
    fn parses() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.version, 2);
        assert_eq!(f.chunks, vec!["HEAD", "SLIC"]);
        assert_eq!(f.data_bytes, 7);
        assert_eq!(f.trailing, 0);
    }

    #[test]
    fn v1() {
        let mut d = b"FORM\x00\x00\x00\x0cREX ".to_vec();
        d.extend_from_slice(b"HEAD\x00\x00\x00\x00");
        assert_eq!(parse(&d).unwrap().version, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"FORM\x00\x00\x00\x04WAVE").is_none());
        assert!(!detect(b"FORM\x00\x00\x00\x04REX"));
    }
}
