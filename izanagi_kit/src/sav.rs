//! SPSS `.sav` — the `$FL2` system file.
//!
//! Header: `"$FL2"` + 60-byte product string, then the fixed record:
//! `layout i32 | nominal_case_size i32 | compression i32 |
//! weight_index i32 | ncases i32 | bias f64 | creation 8+3`.
//!
//! ```
//! let mut d = b"$FL2".to_vec();
//! d.extend_from_slice(b"@(#) SPSS DATA FILE");
//! d.resize(64, b' ');
//! d.extend_from_slice(&[2, 0, 0, 0]);   // layout = 2
//! d.extend_from_slice(&[4, 0, 0, 0]);   // nominal case size
//! d.extend_from_slice(&[1, 0, 0, 0]);   // compression on
//! d.extend_from_slice(&[0, 0, 0, 0]);   // weight index
//! d.extend_from_slice(&[42, 0, 0, 0]);  // ncases
//! d.extend_from_slice(&[0u8; 8]);       // bias f64
//! let s = izanagi_kit::sav::parse(&d).unwrap();
//! assert_eq!(s.ncases, 42);
//! assert_eq!(s.product, "@(#) SPSS DATA FILE");
//! ```

/// Parsed SPSS system-file header.
#[derive(Debug, Clone, PartialEq)]
pub struct Sav {
    /// Product/eye-catcher string (60 bytes, space-trimmed).
    pub product: String,
    /// Layout code (2 or 3).
    pub layout: i32,
    /// Bytes per case per the nominal layout.
    pub nominal_case_size: i32,
    /// Compression flag (1 = integer-compressed values).
    pub compression: i32,
    /// Case count (`-1` when unset).
    pub ncases: i32,
    /// Compression bias — raw IEEE-754 bits (kept integral).
    pub bias_bits: u64,
}

fn le32(d: &[u8], o: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn le64(d: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(o..o + 8)?.try_into().ok()?))
}

/// Parse a `.sav` header; `None` without `$FL2` or a sane layout code.
pub fn parse(d: &[u8]) -> Option<Sav> {
    if !d.starts_with(b"$FL2") {
        return None;
    }
    let product = std::str::from_utf8(d.get(4..64)?)
        .ok()?
        .trim_end()
        .to_string();
    let layout = le32(d, 64)?;
    if !(2..=3).contains(&layout) {
        return None;
    }
    Some(Sav {
        product,
        layout,
        nominal_case_size: le32(d, 68)?,
        compression: le32(d, 72)?,
        ncases: le32(d, 80)?,
        bias_bits: le64(d, 84)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(layout: i32, ncases: i32) -> Vec<u8> {
        let mut d = b"$FL2".to_vec();
        d.extend_from_slice(b"@(#) PASW STATISTICS");
        d.resize(64, b' ');
        d.extend_from_slice(&layout.to_le_bytes());
        d.extend_from_slice(&8i32.to_le_bytes());
        d.extend_from_slice(&1i32.to_le_bytes());
        d.extend_from_slice(&0i32.to_le_bytes());
        d.extend_from_slice(&ncases.to_le_bytes());
        d.extend_from_slice(&[0u8; 8]); // bias f64
        d
    }

    #[test]
    fn basic() {
        let s = parse(&header(2, 100)).unwrap();
        assert_eq!(s.layout, 2);
        assert_eq!(s.ncases, 100);
        assert_eq!(s.product, "@(#) PASW STATISTICS");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"$FL3").is_none());
        assert!(parse(&header(9, 0)).is_none()); // bad layout
    }
}
