//! XQF 象棋棋譜 — `XQ` マジック、バージョン/結果/プレイヤー名、
//! 12 バイト指し手レコード歩行(暗号化フラグ付き)。
//!
//! ```
//! let mut d = vec![b'X', b'Q', 2, 0];
//! d.extend_from_slice(&[0; 40]);
//! // title (NUL-terminated, ≤48B)
//! d.extend_from_slice(b"demo\x00");
//! d.resize(48, 0);
//! // one 12-byte move record: from(4,12) to(4,10) captured=0 res=0
//! d.extend_from_slice(&[4, 12, 4, 10, 0, 0, 0, 0, 0, 0, 0, 0]);
//! let x = izanagi_kit::xqf::parse(&d).unwrap();
//! assert_eq!(x.moves, 1);
//! assert_eq!(x.version, 2);
//! assert!(izanagi_kit::xqf::detect(&d));
//! ```

/// A parsed XQF file census.
#[derive(Debug, Clone)]
pub struct Xqf {
    /// Version byte at offset 2 (usually 0/1/2).
    pub version: u8,
    /// Game-result code in the 44-byte header (0=unknown,1=red,2=black,3=draw).
    pub result: u8,
    /// NUL-terminated title text inside the header region.
    pub title: String,
    /// Number of complete 12-byte move records after the 44-byte header.
    pub moves: usize,
    /// Moves with a non-zero captured piece byte.
    pub captures: usize,
    /// Moves flagged with a non-zero comment/variation byte.
    pub annotated: usize,
    /// Trailing bytes that don't fill a record.
    pub residual: usize,
    /// Encryption flag byte (offset 3) non-zero → obfuscated records.
    pub encrypted: bool,
}

/// Detects an XQF file: `XQ` magic followed by a small version byte.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 48 && b[0] == b'X' && b[1] == b'Q' && b[2] <= 9
}

/// Parses an XQF file; `None` below 48 bytes or without `XQ`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xqf> {
    if !detect(b) {
        return None;
    }
    let header_len = 44 + 4; // magic(4) + game header(40) → move records follow
    let title_start = 4;
    let title_end = b[title_start..header_len.min(b.len())]
        .iter()
        .position(|&c| c == 0)
        .map(|i| title_start + i)
        .unwrap_or(title_start);
    let title = String::from_utf8_lossy(&b[title_start..title_end]).to_string();
    let body = &b[header_len.min(b.len())..];
    let mut moves = 0usize;
    let mut captures = 0usize;
    let mut annotated = 0usize;
    for rec in body.chunks(12) {
        if rec.len() < 12 {
            break;
        }
        moves += 1;
        if rec[6] != 0 || rec[7] != 0 {
            captures += 1;
        }
        if rec[11] != 0 {
            annotated += 1;
        }
    }
    Some(Xqf {
        version: b[2],
        result: b[3].min(3),
        title,
        moves,
        captures,
        annotated,
        residual: body.len() % 12,
        encrypted: b[2] > 8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![b'X', b'Q', 2, 1];
        d.extend_from_slice(&[0u8; 40]);
        d.extend_from_slice(b"demo game\x00");
        d.resize(48, 0);
        for i in 0..3u8 {
            d.extend_from_slice(&[4, 12 + i, 4, 10, 0, 0, 0, 0, 0, 0, 0, 0]);
        }
        d.extend_from_slice(&[6, 2, 6, 5, 0, 0, 9, 0, 0, 0, 0, 1]);
        d
    }

    #[test]
    fn parses() {
        let d = fixture();
        let x = parse(&d).unwrap();
        assert_eq!(x.version, 2);
        assert_eq!(x.moves, 4);
        assert_eq!(x.captures, 1);
        assert_eq!(x.annotated, 1);
        assert_eq!(x.residual, 0);
        assert_eq!(x.title, "");
    }

    #[test]
    fn detect_works() {
        let d = fixture();
        assert!(detect(&d));
        assert!(!detect(b"XQ"));
        assert!(!detect(b"AB\x02\x00"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain text").is_none());
    }
}
