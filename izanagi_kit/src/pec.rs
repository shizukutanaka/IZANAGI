//! Brother PEC stitch block (`#PEC0001` + `LA:` label, name padded
//! to 16 bytes, `\r` separator, color-change count, `\xFF\x00`
//! boundary marker, then the stitch byte stream). This is the block
//! embedded at the tail of `.pes` files and also used standalone.
//!
//! ```
//! let mut d = b"#PEC0001LA:my design       \r".to_vec();
//! d.extend_from_slice(&[0xFF, 0x00, 0, 4]); // separator + bounds
//! d.extend_from_slice(&[0x20]);             // palette count byte
//! let p = izanagi_kit::pec::parse(&d).unwrap();
//! assert_eq!(p.label, "my design");
//! ```

use std::string::String;

/// A parsed PEC block.
#[derive(Clone, Debug)]
pub struct Pec {
    /// `LA:` design label (space-padded in the file, trimmed here).
    pub label: String,
    /// Byte offset of the `\xFF\x00` stitch-section marker.
    pub stitch_marker: usize,
}

/// Parse a PEC block; `None` without `#PEC0001`.
pub fn parse(d: &[u8]) -> Option<Pec> {
    if !d.starts_with(b"#PEC0001") {
        return None;
    }
    let la = d.get(8..11)?;
    if la != b"LA:" {
        return None;
    }
    let name_region = d.get(11..27).unwrap_or(&[]);
    let label = String::from_utf8_lossy(name_region).trim_end().to_string();
    // `\xFF\x00` marks the start of the stitch data region.
    let mut stitch_marker = d.len();
    for (i, w) in d.windows(2).enumerate().skip(27) {
        if w == [0xFF, 0x00] {
            stitch_marker = i;
            break;
        }
    }
    Some(Pec {
        label,
        stitch_marker,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut d = b"#PEC0001LA:sampler         \r".to_vec();
        d.extend_from_slice(&[0xFF, 0x00, 1, 2, 3]);
        let p = parse(&d).unwrap();
        assert_eq!(p.label, "sampler");
        assert!(p.stitch_marker < d.len());
        assert_eq!(&d[p.stitch_marker..p.stitch_marker + 2], &[0xFF, 0x00]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#PEC0000").is_none());
        assert!(parse(b"#PEC0001XX:").is_none()); // no LA:
    }
}
