//! Qualcomm QCP audio — RIFF container with `QLCM` form type.
//!
//! ```
//! let mut d = b"RIFF\x34\x00\x00\x00QLCM".to_vec();
//! d.extend_from_slice(b"fmt \x06\x00\x00\x00\x02\x01\x10\x00\x01\x00");
//! d.extend_from_slice(b"data\x04\x00\x00\x00\x00\x00\x00\x00");
//! let q = izanagi_kit::qcp::parse(&d).unwrap();
//! assert_eq!(q.codec, 2);
//! assert!(izanagi_kit::qcp::detect(&d));
//! ```

/// Parsed QCP (QLCM RIFF) summary.
#[derive(Debug, Clone)]
pub struct Qcp {
    /// Codec byte from `fmt ` (0=QCELP-13K full rate, 1=EVRC, 2=SMV…).
    pub codec: u8,
    /// Rate field from `fmt `.
    pub rate: u8,
    /// Chunk ids seen after `QLCM`, in order.
    pub chunks: Vec<String>,
    /// Declared size of the `data` chunk.
    pub data_len: u32,
    /// RIFF-declared total size.
    pub riff_size: u32,
    /// Truncated trailing chunk bytes.
    pub trailing: usize,
}

/// Detects `RIFF … QLCM`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == b"QLCM"
}

fn le32(b: &[u8], o: usize) -> u32 {
    (b[o] as u32) | ((b[o + 1] as u32) << 8) | ((b[o + 2] as u32) << 16) | ((b[o + 3] as u32) << 24)
}

/// Parses a QCP container; `None` when the RIFF/QLCM signature is absent.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Qcp> {
    if !detect(b) {
        return None;
    }
    let mut f = Qcp {
        codec: 0,
        rate: 0,
        chunks: Vec::new(),
        data_len: 0,
        riff_size: le32(b, 4),
        trailing: 0,
    };
    let mut i = 12usize;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        if !id.iter().all(|&c| (0x20..0x7f).contains(&c)) {
            break;
        }
        f.chunks.push(String::from_utf8_lossy(id).into_owned());
        let size = le32(b, i + 4) as usize;
        let body = i + 8;
        if body + size > b.len() {
            f.trailing = b.len() - i;
            break;
        }
        match id {
            b"fmt " => {
                f.codec = b[body];
                f.rate = b.get(body + 1).copied().unwrap_or(0);
            }
            b"data" => f.data_len = size as u32,
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
        let mut d = b"RIFF\x34\x00\x00\x00QLCM".to_vec();
        d.extend_from_slice(b"fmt \x06\x00\x00\x00\x02\x01\x10\x00\x01\x00");
        d.extend_from_slice(b"vndr\x04\x00\x00\x00QELP");
        d.extend_from_slice(b"data\x04\x00\x00\x00\x00\x00\x00\x00");
        d
    }

    #[test]
    fn parses() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.chunks, vec!["fmt ", "vndr", "data"]);
        assert_eq!(f.codec, 2);
        assert_eq!(f.data_len, 4);
        assert_eq!(f.riff_size, 0x34);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RIFF\x00\x00\x00\x00WAVE").is_none());
        assert!(parse(b"RIFF\x04\x00\x00\x00QLCM").is_none());
        assert!(!detect(&[0u8; 8]));
    }

    #[test]
    fn odd_size_pads() {
        let mut d = b"RIFF\x1f\x00\x00\x00QLCM".to_vec();
        d.extend_from_slice(b"labl\x03\x00\x00\x00ab\x00\x00");
        d.extend_from_slice(b"data\x01\x00\x00\x00x\x00");
        let f = parse(&d).unwrap();
        assert_eq!(f.chunks, vec!["labl", "data"]);
    }
}
