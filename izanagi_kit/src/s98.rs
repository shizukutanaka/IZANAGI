//! PC-98 S98 sound log parser.
//!
//! `.s98` files capture register writes to the PC-98 sound boards
//! (OPN/OPM/OPNA…). The 32-byte header is `"S98" + version_char`
//! (`'0'`, `'1'`, `'2'`, `'3'`) followed by little-endian fields:
//! `timer_info1` (numerator, 0 → 10), `timer_info2` (denominator,
//! 0 → 1000), `compressing`, `tag_offset`, `dump_offset`,
//! `loop_offset`, `device_count` (v3+). After the fixed header an
//! optional `[S98INFO]`/`[TAG]` text block lists metadata.
//!
//! ```
//! let mut f = b"S983".to_vec();
//! f.extend_from_slice(&[0; 28]); // timers, offsets
//! let s = izanagi_kit::s98::parse(&f).unwrap();
//! assert_eq!(s.version, 3);
//! assert_eq!(s.timer_num, 10);   // 0 means the default 10
//! assert_eq!(s.timer_denom, 1000);
//! ```

/// Parsed S98 header.
#[derive(Debug, Clone, PartialEq)]
pub struct S98 {
    /// Format version digit (`0`–`3`).
    pub version: u8,
    /// Timer numerator (0 stored = default 10).
    pub timer_num: u32,
    /// Timer denominator (0 stored = default 1000).
    pub timer_denom: u32,
    /// Compression flag word.
    pub compressing: u32,
    /// Absolute offset of the tag section (0 = none).
    pub tag_offset: u32,
    /// Absolute offset of the dump data (0 = end of header region).
    pub dump_offset: u32,
    /// Absolute offset of the loop point in the dump (0 = none).
    pub loop_offset: u32,
    /// Device count (v3 and later; 0 for earlier).
    pub device_count: u32,
}

fn u32le(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

/// Parse an S98 file; `None` without `S98<digit>` or a truncated
/// 32-byte header.
pub fn parse(d: &[u8]) -> Option<S98> {
    if d.len() < 32 || &d[..3] != b"S98" || !d[3].is_ascii_digit() {
        return None;
    }
    let mut num = u32le(d, 4);
    let mut den = u32le(d, 8);
    if num == 0 {
        num = 10;
    }
    if den == 0 {
        den = 1000;
    }
    Some(S98 {
        version: d[3] - b'0',
        timer_num: num,
        timer_denom: den,
        compressing: u32le(d, 12),
        tag_offset: u32le(d, 16),
        dump_offset: u32le(d, 20),
        loop_offset: u32le(d, 24),
        device_count: u32le(d, 28),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut f = b"S981".to_vec();
        f.extend_from_slice(&[1, 0, 0, 0]); // timer_num = 1
        f.extend_from_slice(&[10, 0, 0, 0]); // timer_denom = 10
        f.resize(32, 0);
        let s = parse(&f).unwrap();
        assert_eq!(s.version, 1);
        assert_eq!(s.timer_num, 1);
        assert_eq!(s.timer_denom, 10);
        assert_eq!(s.dump_offset, 0);
    }

    #[test]
    fn defaults() {
        let mut f = b"S983".to_vec();
        f.resize(32, 0);
        let s = parse(&f).unwrap();
        assert_eq!(s.timer_num, 10);
        assert_eq!(s.timer_denom, 1000);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"S98").is_none());
        assert!(parse(b"S98X").is_none()); // non-digit version char
        assert!(parse(b"S99").is_none()); // wrong signature
        let mut bad = b"S980".to_vec();
        bad.resize(31, 0);
        assert!(parse(&bad).is_none());
    }
}
