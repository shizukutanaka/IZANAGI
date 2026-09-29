//! Harwell-Boeing (HB/Rutherford-Boeing) sparse-matrix file: an
//! 80-column card format — line 1 title, line 2 five `i5` counts
//! (total lines, ptr, ind, val, rhs lines), line 3 `mxn nnz` sizes,
//! line 4 Fortran formats, then pointer/indices/values blocks.
//!
//! ```
//! use izanagi_kit::hb::{detect, parse};
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(b"TITLE                                                                       \n");
//! d.extend_from_slice(b"    3           1           1           1           0\n");
//! d.extend_from_slice(b"psa             3           3           3           0\n");
//! d.extend_from_slice(b"(4i4)           (4i4)           (3e8)           \n");
//! d.extend_from_slice(b"   1   2   3   4\n   1   2   3\n 1.0 2.0 3.0\n");
//! assert!(detect(&d));
//! let h = parse(&d).unwrap();
//! assert_eq!(h.nrows, Some(3));
//! ```

/// Parsed Harwell-Boeing census.
#[derive(Debug, Clone, PartialEq)]
pub struct Hb {
    /// Title line (line 1, trimmed).
    pub title: Option<String>,
    /// `mxkey` type field from line 3 (`rsa`, `psa`, `rre`, …).
    pub mxkey: Option<String>,
    /// `nrow` from line 3.
    pub nrows: Option<u32>,
    /// `ncol` from line 3.
    pub ncols: Option<u32>,
    /// `nnzero` from line 3.
    pub nnz: Option<u32>,
    /// `neltvl` from line 3 (elemental entries).
    pub neltvl: Option<u32>,
    /// Line-2 card counts `[total, ptr, ind, val, rhs]`.
    pub card_counts: [u32; 5],
    /// Non-empty data lines after the format card.
    pub data_lines: u32,
    /// `rhs` block line count claimed by card 2 field 5 > 0.
    pub has_rhs: bool,
}

/// `true` when line 2 has five positive `i5` counts and line 3 a
/// 3-letter type + 3 ints.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut lines = s.lines();
    if lines.next().is_none() {
        return false;
    }
    let l2_ok = lines.next().is_some_and(|l| {
        l.split_whitespace()
            .filter(|t| t.parse::<u32>().is_ok())
            .count()
            >= 5
    });
    let l3_ok = lines.next().is_some_and(|l| {
        let mut it = l.split_whitespace();
        it.next()
            .is_some_and(|k| k.len() == 3 && k.chars().all(|c| c.is_ascii_alphabetic()))
            && it.filter(|t| t.parse::<u32>().is_ok()).count() >= 3
    });
    l2_ok && l3_ok
}

/// Census; `None` without the card skeleton.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Hb> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut lines = s.lines();
    let title = lines.next().map(|l| l.trim().to_string());
    let mut card_counts = [0u32; 5];
    if let Some(l2) = lines.next() {
        for (i, t) in l2.split_whitespace().take(5).enumerate() {
            card_counts[i] = t.parse().unwrap_or(0);
        }
    }
    let (mut mxkey, mut nrows, mut ncols, mut nnz, mut neltvl) = (None, None, None, None, None);
    if let Some(l3) = lines.next() {
        let mut it = l3.split_whitespace();
        mxkey = it.next().map(str::to_string);
        nrows = it.next().and_then(|v| v.parse().ok());
        ncols = it.next().and_then(|v| v.parse().ok());
        nnz = it.next().and_then(|v| v.parse().ok());
        neltvl = it.next().and_then(|v| v.parse().ok());
    }
    let mut data_lines = 0u32;
    for line in lines.skip(1) {
        if !line.trim().is_empty() {
            data_lines += 1;
        }
    }
    Some(Hb {
        title,
        mxkey,
        nrows,
        ncols,
        nnz,
        neltvl,
        card_counts,
        data_lines,
        has_rhs: card_counts[4] > 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(
            b"TITLE                                                                       \n",
        );
        d.extend_from_slice(b"    3           1           1           1           0\n");
        d.extend_from_slice(b"psa             3           3           3           0\n");
        d.extend_from_slice(b"(4i4)           (4i4)           (3e8)           \n");
        d.extend_from_slice(b"   1   2   3   4\n   1   2   3\n 1.0 2.0 3.0\n");
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"a\nb\n"));
    }

    #[test]
    fn parses() {
        let h = parse(&fixture()).unwrap();
        assert_eq!(h.title.as_deref(), Some("TITLE"));
        assert_eq!(h.mxkey.as_deref(), Some("psa"));
        assert_eq!(h.nrows, Some(3));
        assert_eq!(h.ncols, Some(3));
        assert_eq!(h.nnz, Some(3));
        assert_eq!(h.card_counts, [3, 1, 1, 1, 0]);
        assert_eq!(h.data_lines, 3);
        assert!(!h.has_rhs);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
