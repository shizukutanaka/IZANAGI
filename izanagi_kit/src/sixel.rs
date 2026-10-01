//! DEC Sixel graphics — a `DCS P…q` string carrying sixel data bytes
//! (`0x3F`–`0x7E`) plus `#` colour registers, `!` RLE repeats, `"` raster
//! attributes, `-` graphics newlines and `$` carriage returns, closed by
//! `ESC \` (ST).
//!
//! ```
//! let d = b"\x1bPq#0;2;0;0;0#1~~!40~\x1b\\";
//! let s = izanagi_kit::sixel::parse(d).unwrap();
//! assert_eq!(s.colors, 2);
//! assert_eq!(s.repeats, 1);
//! assert!(s.terminated);
//! assert!(izanagi_kit::sixel::detect(d));
//! ```

/// Census of a Sixel image embedded in a buffer.
#[derive(Debug, Clone)]
pub struct Sixel {
    /// Byte offset of the `ESC P` DCS introducer.
    pub offset: usize,
    /// `#n` colour-register commands.
    pub colors: usize,
    /// `!n` run-length repeats.
    pub repeats: usize,
    /// `"` raster-attribute commands.
    pub rasters: usize,
    /// `-` graphics newlines.
    pub lines: usize,
    /// `$` graphics carriage returns.
    pub returns: usize,
    /// Sixel data bytes (`0x3F`–`0x7E`, excluding command introducers).
    pub data_bytes: usize,
    /// Whether a terminating `ESC \` (ST) was found.
    pub terminated: bool,
}

fn scan(b: &[u8]) -> Option<Sixel> {
    let start = b.windows(2).position(|w| w == b"\x1bP")?;
    // parameters may follow before the final 'q'
    let mut i = start + 2;
    while i < b.len() && (0x20..=0x3f).contains(&b[i]) && b[i] != b'q' {
        i += 1;
    }
    if i >= b.len() || b[i] != b'q' {
        return None;
    }
    i += 1;
    let mut s = Sixel {
        offset: start,
        colors: 0,
        repeats: 0,
        rasters: 0,
        lines: 0,
        returns: 0,
        data_bytes: 0,
        terminated: false,
    };
    while i < b.len() {
        if b[i] == 0x1b && b.get(i + 1) == Some(&0x5c) {
            s.terminated = true;
            break;
        }
        match b[i] {
            b'#' => s.colors += 1,
            b'!' => s.repeats += 1,
            b'"' => s.rasters += 1,
            b'-' => s.lines += 1,
            b'$' => s.returns += 1,
            x if (0x3f..=0x7e).contains(&x) => s.data_bytes += 1,
            _ => {}
        }
        i += 1;
    }
    (s.data_bytes > 0).then_some(s)
}

/// Detects a Sixel stream: `ESC P` … `q` followed by sixel data.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    scan(b).is_some()
}

/// Parses a Sixel stream; `None` without a `DCS q` introducer and data.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Sixel> {
    scan(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"pre\x1bP1;2q#0;2;0;0;0#1?~~-!40~\x1b\\post";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.offset, 3);
        assert_eq!(s.colors, 2);
        assert_eq!(s.repeats, 1);
        assert_eq!(s.lines, 1);
        assert!(s.data_bytes > 0);
        assert!(s.terminated);
    }

    #[test]
    fn unterminated_ok() {
        let s = parse(b"\x1bPq#0~~~").unwrap();
        assert!(!s.terminated);
        assert_eq!(s.colors, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"\x1bPq").is_none()); // no data bytes
        assert!(parse(b"abc").is_none());
    }
}
