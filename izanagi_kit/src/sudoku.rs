//! 数独テキスト形式 — 81 セル(数字 `1`-`9`、空は `0`/`.`/`_`/`x`)。
//! 単一行・9 行・`+---+---+` / `|` 区切り付きのいずれも受理する。
//!
//! ```
//! let d = b"53\x2e\x2e7\x2e\x2e\x2e\x2e\n6\x2e\x2e195\x2e\x2e\x2e\n\x2e98\x2e\x2e\x2e\x2e6\x2e\n8\x2e\x2e\x2e6\x2e\x2e\x2e3\n4\x2e\x2e8\x2e3\x2e\x2e1\n7\x2e\x2e\x2e2\x2e\x2e\x2e6\n\x2e6\x2e\x2e\x2e\x2e28\x2e\n\x2e\x2e\x2e419\x2e\x2e5\n\x2e\x2e\x2e\x2e8\x2e\x2e79\n";
//! let s = izanagi_kit::sudoku::parse(d).unwrap();
//! assert_eq!(s.givens, 30);
//! assert_eq!(s.empties, 51);
//! assert!(!s.has_conflicts);
//! assert!(izanagi_kit::sudoku::detect(d));
//! ```

/// A parsed sudoku census.
#[derive(Debug, Clone)]
pub struct Sudoku {
    /// Filled cells (`1`-`9`).
    pub givens: usize,
    /// Empty cells (`0`/`.`/`_`/`x`/`*`).
    pub empties: usize,
    /// Grid rows (≤9 when row-grouped input).
    pub rows: usize,
    /// Any digit repeated in a row, column or 3x3 box → true.
    pub has_conflicts: bool,
    /// Cells `1`-`9` whose value collides (counted once).
    pub conflict_cells: usize,
    /// `+---+`/`|`/`-` separator lines tolerated in the input.
    pub separator_lines: usize,
    /// Single-line (81-run) input form.
    pub single_line: bool,
}

const EMPTY: &[u8] = b"0._xX* ";

fn cells(b: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(81);
    for &c in b {
        if c.is_ascii_digit() {
            v.push(c - b'0');
        } else if EMPTY.contains(&c) {
            v.push(0);
        }
    }
    v
}

/// Detects sudoku text: ≥81 cell-ish symbols with the digit/empty alphabet only.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let v = cells(b);
    if v.len() < 81 {
        return false;
    }
    let grid_letters = t
        .bytes()
        .filter(|c| {
            !(c.is_ascii_digit()
                || EMPTY.contains(c)
                || c.is_ascii_whitespace()
                || matches!(c, b'+' | b'-' | b'|'))
        })
        .count();
    grid_letters == 0
}

/// Parses a sudoku grid; `None` unless exactly ≥81 cells parse cleanly.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Sudoku> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let v = cells(b);
    if v.len() < 81 {
        return None;
    }
    let g = &v[..81];
    let mut givens = 0usize;
    for &c in g {
        if c != 0 {
            givens += 1;
        }
    }
    let mut conflicts = 0usize;
    let mut seen = [[0u16; 9]; 9];
    for (idx, &val) in g.iter().enumerate() {
        let (r, c) = (idx / 9, idx % 9);
        if val == 0 {
            continue;
        }
        let bit = 1u16 << (val - 1);
        let mut hit = false;
        for (cc, &v) in seen[r].iter().enumerate() {
            if cc != c && v & bit != 0 {
                hit = true;
            }
        }
        for (rr, row) in seen.iter().enumerate() {
            if rr != r && row[c] & bit != 0 {
                hit = true;
            }
        }
        let (br, bc) = (r / 3 * 3, c / 3 * 3);
        for (dr, row) in seen[br..br + 3].iter().enumerate() {
            for (dc, &v) in row[bc..bc + 3].iter().enumerate() {
                if (br + dr != r || bc + dc != c) && v & bit != 0 {
                    hit = true;
                }
            }
        }
        if hit {
            conflicts += 1;
        }
        seen[r][c] |= bit;
    }
    let rows = t
        .lines()
        .filter(|l| {
            let l = l.trim();
            !l.is_empty() && !l.starts_with('+') && l.bytes().any(|c| c.is_ascii_digit())
        })
        .count();
    Some(Sudoku {
        givens,
        empties: 81 - givens,
        rows,
        has_conflicts: conflicts > 0,
        conflict_cells: conflicts,
        separator_lines: t
            .lines()
            .filter(|l| l.trim_start().starts_with('+') || l.contains('|'))
            .count(),
        single_line: rows <= 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"53\x2e\x2e7\x2e\x2e\x2e\x2e\n6\x2e\x2e195\x2e\x2e\x2e\n\x2e98\x2e\x2e\x2e\x2e6\x2e\n8\x2e\x2e\x2e6\x2e\x2e\x2e3\n4\x2e\x2e8\x2e3\x2e\x2e1\n7\x2e\x2e\x2e2\x2e\x2e\x2e6\n\x2e6\x2e\x2e\x2e\x2e28\x2e\n\x2e\x2e\x2e419\x2e\x2e5\n\x2e\x2e\x2e\x2e8\x2e\x2e79\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.givens, 30);
        assert_eq!(s.empties, 51);
        assert!(!s.has_conflicts);
        assert_eq!(s.rows, 9);
    }

    #[test]
    fn conflicts_flagged() {
        let mut d = [b'5'; 81];
        d[40] = b'0';
        let s = parse(&d).unwrap();
        assert!(s.has_conflicts);
    }

    #[test]
    fn single_line() {
        let mut d = Vec::new();
        d.extend_from_slice(b"53\x2e\x2e7\x2e\x2e\x2e\x2e");
        d.extend_from_slice(b"6\x2e\x2e195\x2e\x2e\x2e");
        d.extend_from_slice(b"\x2e98\x2e\x2e\x2e\x2e6\x2e");
        d.extend_from_slice(b"8\x2e\x2e\x2e6\x2e\x2e\x2e3");
        d.extend_from_slice(b"4\x2e\x2e8\x2e3\x2e\x2e1");
        d.extend_from_slice(b"7\x2e\x2e\x2e2\x2e\x2e\x2e6");
        d.extend_from_slice(b"\x2e6\x2e\x2e\x2e\x2e28\x2e");
        d.extend_from_slice(b"\x2e\x2e\x2e419\x2e\x2e5");
        d.extend_from_slice(b"\x2e\x2e\x2e\x2e8\x2e\x2e79");
        assert_eq!(d.len(), 81);
        let s = parse(&d).unwrap();
        assert!(s.single_line);
        assert_eq!(s.givens, 30);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"53\x2e\x2e7"));
        assert!(!detect(b"81 letters are not sudoku aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}
