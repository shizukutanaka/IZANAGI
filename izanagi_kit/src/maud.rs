//! MAUD — Commodore Amiga IFF `MAUD` sampled-sound form.
//!
//! ```
//! let mut d = b"FORM\x00\x00\x00\x24MAUD".to_vec();
//! // MHDR: rate u32be, + 18 bytes
//! d.extend_from_slice(b"MHDR\x00\x00\x00\x16\x00\x00\x22\xB8");
//! d.extend_from_slice(&[0u8; 18]);
//! let m = izanagi_kit::maud::parse(&d).unwrap();
//! assert_eq!(m.sample_rate, Some(8888));
//! assert!(izanagi_kit::maud::detect(&d));
//! ```

/// Parsed MAUD form summary.
#[derive(Debug, Clone)]
pub struct Maud {
    /// Declared FORM payload size.
    pub form_size: u32,
    /// Chunk ids in order.
    pub chunks: Vec<String>,
    /// `MHDR` sample rate (u32be, first 4 bytes).
    pub sample_rate: Option<u32>,
    /// `ANNO`/`AUTH`/`(c) `/`NAME` text chunks collected.
    pub annotations: Vec<String>,
    /// `MDAT` data byte count.
    pub data_bytes: u32,
    /// Truncated trailing bytes.
    pub trailing: usize,
}

fn be32(b: &[u8], o: usize) -> u32 {
    ((b[o] as u32) << 24) | ((b[o + 1] as u32) << 16) | ((b[o + 2] as u32) << 8) | (b[o + 3] as u32)
}

/// Detects `FORM` + `MAUD`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 12 && b.starts_with(b"FORM") && &b[8..12] == b"MAUD"
}

/// Parses a MAUD container; `None` without the signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Maud> {
    if !detect(b) {
        return None;
    }
    let mut f = Maud {
        form_size: be32(b, 4),
        chunks: Vec::new(),
        sample_rate: None,
        annotations: Vec::new(),
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
        let body = i + 8;
        if body + size > b.len() {
            f.trailing = b.len() - i;
            break;
        }
        match id {
            b"MHDR" if size >= 4 => f.sample_rate = Some(be32(b, body)),
            b"ANNO" | b"AUTH" | b"(c) " | b"NAME" => {
                f.annotations
                    .push(String::from_utf8_lossy(&b[body..body + size]).into_owned());
            }
            b"MDAT" => f.data_bytes = size as u32,
            _ => {}
        }
        i = body + size + (size & 1);
    }
    if i < b.len() && f.trailing == 0 {
        f.trailing = b.len() - i;
    }
    if f.chunks.is_empty() {
        return None;
    }
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"FORM\x00\x00\x00\x3cMAUD".to_vec();
        d.extend_from_slice(b"MHDR\x00\x00\x00\x16\x00\x00\x22\xB8");
        d.extend_from_slice(&[0u8; 18]);
        d.extend_from_slice(b"ANNO\x00\x00\x00\x05hello\x00");
        d.extend_from_slice(b"MDAT\x00\x00\x00\x04\x01\x02\x03\x04");
        d
    }

    #[test]
    fn parses() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.chunks, vec!["MHDR", "ANNO", "MDAT"]);
        assert_eq!(f.sample_rate, Some(8888));
        assert_eq!(f.annotations, vec!["hello"]);
        assert_eq!(f.data_bytes, 4);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"FORM\x00\x00\x00\x04MAUD").is_none());
        assert!(parse(b"FORM\x00\x00\x00\x0c8SVX").is_none());
        assert!(!detect(&[0u8; 16]));
    }
}
