//! EPWING `CATALOGS` — the book catalog of an EB/EBXA/EPWING
//! electronic-book disc: a 16-byte header (`\x00` + disc code +
//! big-endian book count) followed by fixed 80-byte book entries
//! carrying a title field.
//!
//! ```
//! use izanagi_kit::epwing::{detect, parse};
//!
//! let mut d = vec![0u8; 16];
//! d[0] = 0; d[1] = 5;               // EPWING disc code
//! d[2] = 0; d[3] = 1;               // one book
//! let mut e = vec![0u8; 80];
//! e[0] = 0x30; e[1] = 0x00;
//! e[2..6].copy_from_slice(b"DICT");
//! d.extend_from_slice(&e);
//! assert!(detect(&d));
//! let c = parse(&d).unwrap();
//! assert_eq!(c.books, 1);
//! ```

/// Parsed EPWING catalog census.
#[derive(Debug, Clone, PartialEq)]
pub struct Epwing {
    /// Disc code byte (`1` EBXA, `3` EBXA-C, `4` S-EBXA, `5` EPWING).
    pub disc_code: u8,
    /// Declared book count from the catalog header.
    pub books: u16,
    /// Entry records actually walked (truncated files report fewer).
    pub entries: u32,
    /// Non-empty entry titles (lossy, trimmed at the first NUL).
    pub titles: Vec<String>,
}

fn be16(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from(*b.get(off)?) << 8 | u16::from(*b.get(off + 1)?))
}

/// `true` on `00 {01|03|04|05}` + a plausible book count.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 16 || b[0] != 0 {
        return false;
    }
    if !matches!(b[1], 1 | 3 | 4 | 5) {
        return false;
    }
    let n = be16(b, 2).unwrap_or(0);
    (1..=255).contains(&n)
}

/// Census; `None` without a catalog header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Epwing> {
    if !detect(b) {
        return None;
    }
    let books = be16(b, 2)?;
    let mut c = Epwing {
        disc_code: b[1],
        books,
        entries: 0,
        titles: Vec::new(),
    };
    let mut off = 16usize;
    while c.entries < u32::from(books) && off + 80 <= b.len() {
        let e = &b[off..off + 80];
        c.entries += 1;
        let name = &e[2..e.len().min(66)];
        let end = name.iter().position(|&x| x == 0).unwrap_or(name.len());
        let t = String::from_utf8_lossy(&name[..end]).trim().to_string();
        if !t.is_empty() {
            c.titles.push(t);
        }
        off += 80;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(code: u8, titles: &[&str]) -> Vec<u8> {
        let mut d = vec![0u8; 16];
        d[1] = code;
        let n = u16::try_from(titles.len()).unwrap();
        d[2] = (n >> 8) as u8;
        d[3] = n as u8;
        for t in titles {
            let mut e = vec![0u8; 80];
            e[0] = 0x30;
            e[2..2 + t.len()].copy_from_slice(t.as_bytes());
            d.extend_from_slice(&e);
        }
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&build(5, &["A"])));
        assert!(detect(&build(4, &["A", "B"])));
        assert!(!detect(&build(2, &["A"])));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn parses() {
        let c = parse(&build(5, &["GENJI", "KOJIEN"])).unwrap();
        assert_eq!(c.disc_code, 5);
        assert_eq!(c.books, 2);
        assert_eq!(c.entries, 2);
        assert_eq!(c.titles, ["GENJI", "KOJIEN"]);
    }

    #[test]
    fn truncated_still_parses() {
        let mut d = build(5, &["A"]);
        d[3] = 3; // declare 3, provide 1
        let c = parse(&d).unwrap();
        assert_eq!(c.entries, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
