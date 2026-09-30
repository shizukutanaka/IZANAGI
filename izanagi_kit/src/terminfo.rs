//! Compiled `terminfo` entries — the `tic` output format: magic
//! `0x011A` (legacy) or `0x021E` (extended-number) followed by six
//! little-endian u16 section sizes (names, booleans, numbers, strings,
//! string-table).
//!
//! ```
//! let mut d = vec![0x1a, 0x01];
//! for v in [10u16, 3, 2, 5, 10] {
//!     d.push((v & 0xff) as u8);
//!     d.push((v >> 8) as u8);
//! }
//! d.extend_from_slice(b"xterm|x\0\0\0\0");
//! let t = izanagi_kit::terminfo::parse(&d).unwrap();
//! assert!(!t.extended);
//! assert_eq!(t.names_size, 10);
//! assert_eq!(t.strs, 5);
//! assert!(izanagi_kit::terminfo::detect(&d));
//! ```

/// A compiled-terminfo header census.
#[derive(Debug, Clone)]
pub struct Terminfo {
    /// `true` for the extended `0x021E` format (32-bit numbers section).
    pub extended: bool,
    /// Size in bytes of the terminal-names section.
    pub names_size: u16,
    /// Boolean-capability count.
    pub bools: u16,
    /// Numeric-capability count.
    pub nums: u16,
    /// String-capability count.
    pub strs: u16,
    /// String-table size in bytes.
    pub table_size: u16,
}

fn u16le(b: &[u8], o: usize) -> u16 {
    (b[o] as u16) | ((b[o + 1] as u16) << 8)
}

fn scan(b: &[u8]) -> Option<Terminfo> {
    if b.len() < 14 {
        return None;
    }
    let extended = match (b[0], b[1]) {
        (0x1a, 0x01) => false,
        (0x1e, 0x02) => true,
        _ => return None,
    };
    let names = u16le(b, 2);
    let bools = u16le(b, 4);
    let nums = u16le(b, 6);
    let strs = u16le(b, 8);
    let table = u16le(b, 10);
    let rest = b.len().saturating_sub(12);
    if names == 0 || names > 512 || names as usize > rest || table as usize > rest {
        return None;
    }
    if (bools as usize) + (nums as usize) + (strs as usize) == 0 {
        return None;
    }
    Some(Terminfo {
        extended,
        names_size: names,
        bools,
        nums,
        strs,
        table_size: table,
    })
}

/// Detects a compiled terminfo entry by its two magic words and sane counts.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    scan(b).is_some()
}

/// Parses a terminfo header; `None` on bad magic or implausible sizes.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Terminfo> {
    scan(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(ext: bool) -> Vec<u8> {
        let mut d = if ext {
            vec![0x1e, 0x02]
        } else {
            vec![0x1a, 0x01]
        };
        for v in [9u16, 4, 3, 6, 10] {
            d.push((v & 0xff) as u8);
            d.push((v >> 8) as u8);
        }
        d.extend_from_slice(b"vt100|x\0\0\0\0\0\0");
        d
    }

    #[test]
    fn parses() {
        let t = parse(&fixture(false)).unwrap();
        assert!(!t.extended);
        assert_eq!(t.names_size, 9);
        assert_eq!(t.bools, 4);
        assert_eq!(t.nums, 3);
        assert_eq!(t.strs, 6);
        assert_eq!(t.table_size, 10);
    }

    #[test]
    fn extended_flag() {
        let t = parse(&fixture(true)).unwrap();
        assert!(t.extended);
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture(false)));
        assert!(!detect(b"hello"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0x1a, 0x01]).is_none()); // short header
        let mut bad = fixture(false);
        bad[2] = 0;
        bad[3] = 0; // names_size = 0
        assert!(parse(&bad).is_none());
    }
}
