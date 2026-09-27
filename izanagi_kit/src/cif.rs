//! Crystallographic Information File (CIF / mmCIF) — `data_` blocks,
//! `_tag value` items, and `loop_` columnar sections.
//!
//! Scalars keep their raw text; `num()` additionally folds a numeric
//! prefix (dropping an `(su)` suffix) into ×10⁶ micro-units so the
//! parser itself stays float-free.
//!
//! ```
//! use izanagi_kit::cif::parse;
//!
//! let c = parse(b"data_x\n_cell_length_a 5.641(2)\nloop_\n_a\n_b\n1 2\n3 4\n").unwrap();
//! assert_eq!(c.blocks[0].items[0].1, "5.641(2)");
//! assert_eq!(c.blocks[0].loops[0].columns, vec!["a".to_string(), "b".to_string()]);
//! ```

use std::string::String;
use std::vec::Vec;

/// One `loop_` section: column tags and rows (rows × columns values).
#[derive(Clone, Debug)]
pub struct Loop {
    /// Column tags, without the leading `_`.
    pub columns: Vec<String>,
    /// Row-major values; `rows.len() % columns.len() == 0`.
    pub rows: Vec<Vec<String>>,
}

/// One `data_` block.
#[derive(Clone, Debug)]
pub struct Block {
    /// Block name (text after `data_`).
    pub name: String,
    /// Scalar items as `(tag, value)` with tag's `_` stripped.
    pub items: Vec<(String, String)>,
    /// `loop_` sections.
    pub loops: Vec<Loop>,
}

/// Whole file: every `data_` block.
#[derive(Clone, Debug)]
pub struct Cif {
    /// Data blocks in file order.
    pub blocks: Vec<Block>,
}

struct Tok {
    text: String,
    /// Line the token starts on (1-based).
    line: usize,
    /// True for `;`-delimited text fields (allowed on a later line).
    text_field: bool,
}

fn toks(d: &[u8]) -> Option<Vec<Tok>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut bol = true; // at start of a line
    while i < d.len() {
        let c = d[i];
        if c == b'\n' {
            line += 1;
            bol = true;
            i += 1;
            continue;
        }
        if c == b'#' {
            while i < d.len() && d[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            i += 1;
            continue;
        }
        if bol && c == b';' {
            // text field: until a line starting with ';'
            let start_line = line;
            i += 1;
            let start = i;
            loop {
                while i < d.len() && d[i] != b'\n' {
                    i += 1;
                }
                if i >= d.len() {
                    return None; // unterminated text field
                }
                line += 1;
                i += 1;
                if i < d.len() && d[i] == b';' {
                    let end = i.saturating_sub(1); // before the newline
                    while i < d.len() && d[i] != b'\n' {
                        i += 1;
                    }
                    out.push(Tok {
                        text: String::from_utf8_lossy(&d[start..end]).into_owned(),
                        line: start_line,
                        text_field: true,
                    });
                    bol = true;
                    break;
                }
            }
            continue;
        }
        if c == b'\'' || c == b'"' {
            let q = c;
            let start_line = line;
            i += 1;
            let start = i;
            while i < d.len() && d[i] != q {
                i += 1;
            }
            if i >= d.len() {
                return None;
            }
            let text = String::from_utf8_lossy(&d[start..i]).into_owned();
            i += 1;
            out.push(Tok {
                text,
                line: start_line,
                text_field: false,
            });
            bol = false;
            continue;
        }
        let start = i;
        let start_line = line;
        while i < d.len() && !matches!(d[i], b' ' | b'\t' | b'\r' | b'\n' | b'#') {
            i += 1;
        }
        out.push(Tok {
            text: String::from_utf8_lossy(&d[start..i]).into_owned(),
            line: start_line,
            text_field: false,
        });
        bol = false;
    }
    Some(out)
}

fn is_keyword(t: &str) -> bool {
    let tl = t.to_ascii_lowercase();
    tl.starts_with("data_")
        || tl.starts_with("loop_")
        || tl.starts_with("save_")
        || tl.starts_with("stop_")
        || tl.starts_with('_')
}

/// Parse a `.cif` / `.mmcif` file.
pub fn parse(d: &[u8]) -> Option<Cif> {
    let t = toks(d)?;
    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0usize;
    while i < t.len() {
        if !t[i].text.to_ascii_lowercase().starts_with("data_") {
            i += 1;
            continue;
        }
        let mut b = Block {
            name: t[i].text[5..].to_string(),
            items: Vec::new(),
            loops: Vec::new(),
        };
        i += 1;
        while i < t.len() && !t[i].text.to_ascii_lowercase().starts_with("data_") {
            if t[i].text.to_ascii_lowercase().starts_with("loop_") {
                i += 1;
                let mut l = Loop {
                    columns: Vec::new(),
                    rows: Vec::new(),
                };
                while i < t.len() && t[i].text.starts_with('_') {
                    l.columns.push(t[i].text[1..].to_string());
                    i += 1;
                }
                if l.columns.is_empty() {
                    return None;
                }
                let mut vals = Vec::new();
                while i < t.len() && !is_keyword(&t[i].text) && !t[i].text_field {
                    vals.push(t[i].text.clone());
                    i += 1;
                }
                if vals.is_empty() || vals.len() % l.columns.len() != 0 {
                    return None;
                }
                let mut r = Vec::new();
                for chunk in vals.chunks(l.columns.len()) {
                    r.push(chunk.to_vec());
                }
                l.rows = r;
                b.loops.push(l);
                continue;
            }
            if t[i].text.starts_with('_') {
                let tag = t[i].text[1..].to_string();
                let line = t[i].line;
                i += 1;
                if i < t.len() && !is_keyword(&t[i].text) && (t[i].line == line || t[i].text_field)
                {
                    b.items.push((tag, t[i].text.clone()));
                    i += 1;
                } else {
                    b.items.push((tag, String::new()));
                }
                continue;
            }
            i += 1; // save_/stop_/stray tokens are skipped
        }
        blocks.push(b);
    }
    if blocks.is_empty() {
        return None;
    }
    Some(Cif { blocks })
}

/// Numeric value of a scalar item, ×10⁶ micro-units, ignoring a
/// standard-uncertainty `(n)` suffix. `None` when not numeric.
pub fn num(s: &str) -> Option<i64> {
    let s = s.split('(').next()?.trim();
    if s == "?" || s == "." {
        return None;
    }
    micro(s)
}

fn micro(s: &str) -> Option<i64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if int.is_empty() && frac.is_empty() || frac.len() > 6 {
        return None;
    }
    if !int.bytes().all(|c| c.is_ascii_digit()) || !frac.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let ip = int.parse::<i64>().unwrap_or(0);
    let mut fp = 0i64;
    let mut sc = 1i64;
    for c in frac.bytes() {
        fp = fp.checked_mul(10)?.checked_add((c - b'0') as i64)?;
        sc *= 10;
    }
    for _ in frac.len()..6 {
        fp *= 10;
    }
    let v = ip.checked_mul(1_000_000)?.checked_add(fp).unwrap_or(0);
    let _ = sc;
    Some(if neg { -v } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_and_loops() {
        let c =
            parse(b"data_test\n_cell_length_a 5.641(2)\n_atom_sites\nloop_\n_a\n_b\n1 2\n3 4\n")
                .unwrap();
        assert_eq!(c.blocks.len(), 1);
        assert_eq!(c.blocks[0].items[0].0, "cell_length_a");
        assert_eq!(num(&c.blocks[0].items[0].1), Some(5_641_000));
        assert_eq!(c.blocks[0].loops[0].rows.len(), 2);
    }

    #[test]
    fn text_fields_and_quotes() {
        let c = parse(b"data_x\n_t 'a b'\n_c\n;hello\nworld\n;\n").unwrap();
        assert_eq!(c.blocks[0].items[0].1, "a b");
        assert_eq!(c.blocks[0].items[1].1, "hello\nworld");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"just text").is_none());
        assert!(parse(b"data_x\nloop_\n_a\n_b\n1 2 3\n").is_none()); // row not multiple of cols
    }

    #[test]
    fn non_numeric() {
        assert_eq!(num("?"), None);
        assert_eq!(num("."), None);
        assert_eq!(num("-1.5"), Some(-1_500_000));
    }
}
