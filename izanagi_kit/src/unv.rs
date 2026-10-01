//! I-DEAS Universal File (`.unv`): a sequence of datasets, each
//! bracketed by lines containing exactly `-1`, with a dataset id
//! (e.g. `2411` = nodes, `2412` = elements, `2414` = analysis data)
//! on the line after the opening `-1`.
//!
//! ```
//! let d = b"    -1\n  2411\n    1    1    1    1\n    -1\n";
//! let u = izanagi_kit::unv::parse(d).unwrap();
//! assert_eq!(u.datasets, vec![2411]);
//! ```

use std::vec::Vec;

/// A parsed Universal File: just the dataset id sequence.
#[derive(Clone, Debug)]
pub struct Unv {
    /// Dataset ids in file order (2411 nodes, 2412 elements,
    /// 55 header, 58 function, 82 truss, …).
    pub datasets: Vec<u32>,
}

/// Parse a `.unv`; `None` unless it consists of `-1`-delimited
/// datasets with numeric ids.
pub fn parse(d: &[u8]) -> Option<Unv> {
    let s = std::str::from_utf8(d).ok()?;
    let mut datasets = Vec::new();
    // An opening `-1` toggles on; the next non-empty line is the
    // dataset id; the closing `-1` toggles off.
    let mut in_ds = false;
    let mut need_id = false;
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        if t == "-1" {
            if in_ds {
                in_ds = false;
            } else {
                in_ds = true;
                need_id = true;
            }
            continue;
        }
        if in_ds && need_id {
            let id: u32 = t.parse().ok()?;
            if id == 0 || id >= 10000 {
                return None;
            }
            datasets.push(id);
            need_id = false;
        }
    }
    if datasets.is_empty() || in_ds {
        return None;
    }
    Some(Unv { datasets })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"    -1\n  55\nHeader Text\n    -1\n    -1\n  2411\n 1 1 1 1\n    -1\n";
        let u = parse(d).unwrap();
        assert_eq!(u.datasets, vec![55, 2411]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello\n").is_none());
        assert!(parse(b"-1\nabc\n-1\n").is_none()); // non-numeric id
        assert!(parse(b"-1\n55\n").is_none()); // unterminated dataset
    }
}
