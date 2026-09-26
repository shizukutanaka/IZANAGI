//! FITS (Flexible Image Transport System) — astronomy's 1980s
//! standard that still rules: an *HDU* is a header of 80-byte
//! ASCII cards (padded to 2880) followed by a data unit
//! (`BITPIX × NAXISn` bytes, padded the same way). A file is a
//! sequence of HDUs: primary, then `XTENSION` extensions
//! (`IMAGE`, `BINTABLE`, `TABLE`, …).
//!
//! ```
//! use izanagi_kit::fits;
//! let mut d = Vec::new();
//! let card = |k: &str, v: &str| -> [u8; 80] {
//!     let mut c = [b' '; 80];
//!     c[..k.len()].copy_from_slice(k.as_bytes());
//!     let s = format!("{}= {}", k, v);
//!     c[..s.len()].copy_from_slice(s.as_bytes());
//!     c
//! };
//! let mut h = Vec::new();
//! h.extend_from_slice(&card("SIMPLE  ", "                  T"));
//! h.extend_from_slice(&card("BITPIX  ", "                   8"));
//! h.extend_from_slice(&card("NAXIS   ", "                   0"));
//! h.extend_from_slice(&card("END     ", "                    "));
//! while h.len() % 2880 != 0 {
//!     h.push(b' ');
//! }
//! d.extend_from_slice(&h);
//! let f = fits::parse(&d).unwrap();
//! assert_eq!(f.hdus.len(), 1);
//! assert_eq!(fits::get(&f.hdus[0], "BITPIX"), Some("8"));
//! ```

use std::vec::Vec;

/// Cards per header block.
pub const BLOCK: usize = 2880;
/// Card width.
pub const CARD: usize = 80;

/// One header card.
#[derive(Clone, Debug)]
pub struct Card {
    /// Keyword, left-justified in 8 bytes.
    pub key: [u8; 8],
    /// `Some(value)` when columns 9–10 are `= `; the value field
    /// (cols 11+, before any `/` comment) is kept verbatim-trimmed.
    /// `None` for commentary cards (`COMMENT`, `HISTORY`, `END`,
    /// blank).
    pub value: Option<std::string::String>,
}

/// One Header+Data Unit.
#[derive(Clone, Debug)]
pub struct Hdu {
    /// Byte offset of the header.
    pub header_at: usize,
    /// Byte offset of the data unit.
    pub data_at: usize,
    /// Data bytes (unpadded) per the header math.
    pub data_len: usize,
    /// The header cards in order.
    pub cards: Vec<Card>,
}

/// File surface.
#[derive(Clone, Debug)]
pub struct Fits {
    /// HDUs in order (`hdus[0]` is the primary array).
    pub hdus: Vec<Hdu>,
}

fn card(d: &[u8], at: usize) -> Option<Card> {
    let c = d.get(at..at + CARD)?;
    let mut key = [0u8; 8];
    key.copy_from_slice(&c[..8]);
    // every byte must be ASCII printable or space
    if c.iter().any(|&b| !(b == b' ' || b.is_ascii_graphic())) {
        return None;
    }
    let value = if &c[8..10] == b"= " {
        // value runs to col 70ish; `/` outside quotes starts comment
        let field = &c[10..];
        let mut out = std::string::String::new();
        let mut in_str = false;
        for &b in field {
            match b {
                b'\'' => {
                    in_str = !in_str;
                    out.push('\'');
                }
                b'/' if !in_str => break,
                _ => out.push(b as char),
            }
        }
        Some(out.trim().to_string())
    } else {
        None
    };
    Some(Card { key, value })
}

fn key_str(k: &[u8; 8]) -> &str {
    std::str::from_utf8(k).unwrap_or("").trim_end()
}

/// Card lookup by keyword (first match).
pub fn get<'a>(h: &'a Hdu, key: &str) -> Option<&'a str> {
    h.cards
        .iter()
        .find(|c| key_str(&c.key) == key)
        .and_then(|c| c.value.as_deref())
        .map(|s| s.trim())
}

/// Integer-valued card lookup (`BITPIX`, `NAXISn`, …).
pub fn get_int(h: &Hdu, key: &str) -> Option<i64> {
    get(h, key)?.parse().ok()
}

fn data_bytes(h: &Hdu) -> Option<u64> {
    let bitpix = get_int(h, "BITPIX")?;
    if !matches!(bitpix, 8 | 16 | 32 | 64 | -32 | -64) {
        return None;
    }
    let naxis = get_int(h, "NAXIS")?;
    if !(0..=999).contains(&naxis) {
        return None;
    }
    let mut total: u64 = (bitpix.unsigned_abs() / 8).max(1);
    if naxis == 0 {
        total = 0;
    }
    for i in 1..=(naxis as usize) {
        let n = get_int(h, &(std::string::String::from("NAXIS") + &i.to_string()))?;
        if !(0..=1_000_000_000).contains(&n) {
            return None;
        }
        total = total.checked_mul(n as u64)?;
    }
    // random-groups/table convention: GCOUNT groups × (PCOUNT + array)
    let gcount = get_int(h, "GCOUNT").unwrap_or(1);
    let pcount = get_int(h, "PCOUNT").unwrap_or(0);
    if gcount > 0 {
        total = total.checked_mul(gcount as u64)?;
        total = total.checked_add(pcount as u64 * (bitpix.unsigned_abs() / 8))?;
    }
    Some(total)
}

/// Parses a FITS file into its HDU sequence; stops cleanly at
/// the first non-card boundary (FITS files may end with padding
/// or nothing).
pub fn parse(d: &[u8]) -> Option<Fits> {
    let mut hdus = Vec::new();
    let mut at = 0usize;
    while at + CARD <= d.len() {
        // an HDU header starts with an 8-char keyword card
        let first = card(d, at)?;
        let k = key_str(&first.key);
        if !(k == "SIMPLE" || k == "XTENSION") {
            break; // trailing padding or EOF — done
        }
        let header_at = at;
        let mut cards = vec![first];
        at += CARD;
        let mut ended = false;
        while !ended {
            let c = card(d, at)?;
            ended = key_str(&c.key) == "END";
            cards.push(c);
            at += CARD;
        }
        // header padded to 2880
        while at % BLOCK != 0 {
            if *d.get(at)? != b' ' {
                return None;
            }
            at += 1;
        }
        let data_at = at;
        let h = Hdu {
            header_at,
            data_at,
            data_len: 0,
            cards,
        };
        let len = data_bytes(&h)? as usize;
        if at.checked_add(len)? > d.len() {
            return None;
        }
        at += len;
        // data padded to 2880
        let pad = (BLOCK - (len % BLOCK)) % BLOCK;
        if at + pad <= d.len() {
            at += pad;
        }
        hdus.push(Hdu { data_len: len, ..h });
    }
    if hdus.is_empty() {
        return None;
    }
    Some(Fits { hdus })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card_bytes(k: &str, v: &str) -> [u8; 80] {
        let mut c = [b' '; 80];
        let s = if v.is_empty() {
            std::string::String::from(k)
        } else {
            k.to_string() + "= " + v
        };
        c[..s.len()].copy_from_slice(s.as_bytes());
        c
    }

    fn hdu(cards: &[(&str, &str)], data: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        for (k, val) in cards {
            v.extend_from_slice(&card_bytes(k, val));
        }
        v.extend_from_slice(&card_bytes("END     ", ""));
        while v.len() % 2880 != 0 {
            v.push(b' ');
        }
        v.extend_from_slice(data);
        while v.len() % 2880 != 0 {
            v.push(b' ');
        }
        v
    }

    #[test]
    fn parses_primary_and_image() {
        let mut d = hdu(
            &[
                ("SIMPLE  ", "                  T"),
                ("BITPIX  ", "                  -32"),
                ("NAXIS   ", "                   2"),
                ("NAXIS1  ", "                   2"),
                ("NAXIS2  ", "                   1"),
            ],
            &[0u8; 8],
        );
        d.extend_from_slice(&hdu(
            &[
                ("XTENSION", "'IMAGE   '"),
                ("BITPIX  ", "                   8"),
                ("NAXIS   ", "                   0"),
            ],
            &[],
        ));
        let f = parse(&d).unwrap();
        assert_eq!(f.hdus.len(), 2);
        assert_eq!(get(&f.hdus[0], "BITPIX"), Some("-32"));
        assert_eq!(get_int(&f.hdus[0], "BITPIX"), Some(-32));
        assert_eq!(f.hdus[0].data_len, 8);
        assert_eq!(f.hdus[1].data_len, 0);
    }

    #[test]
    fn card_comment_and_string_value() {
        let d = hdu(
            &[
                ("SIMPLE  ", "                  T / primary"),
                ("BITPIX  ", "                   8"),
                ("NAXIS   ", "                   0"),
                ("OBJECT  ", "'M42     ' / comment with /slash"),
            ],
            &[],
        );
        let f = parse(&d).unwrap();
        assert_eq!(get(&f.hdus[0], "OBJECT"), Some("'M42     '"));
        assert_eq!(get(&f.hdus[0], "SIMPLE"), Some("T"));
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[b' '; 80]).is_none()); // no SIMPLE/XTENSION
                                               // header where BITPIX is bogus
        let d = hdu(
            &[
                ("SIMPLE  ", "                  T"),
                ("BITPIX  ", "                 128"),
                ("NAXIS   ", "                   0"),
            ],
            &[],
        );
        assert!(parse(&d).is_none());
    }
}
