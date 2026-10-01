//! Trivial Graph Format (`.tgf`).
//!
//! TGF files are plain text: node lines `id label`, then a single `#`
//! separator, then edge lines `from to [label]`.
//!
//! ```
//! let b = concat!(
//!     "1 Alpha\n",
//!     "2 Beta\n",
//!     "#\n",
//!     "1 2 edge-label\n"
//! ).as_bytes();
//! assert!(izanagi_kit::tgf::detect(b));
//! let c = izanagi_kit::tgf::Tgf::parse(b).unwrap();
//! assert_eq!(c.nodes, 2);
//! assert_eq!(c.edges, 1);
//! ```

/// Parsed TGF summary.
#[derive(Debug, Clone)]
pub struct Tgf {
    /// Node lines before `#`.
    pub nodes: usize,
    /// Edge lines after `#`.
    pub edges: usize,
    /// `#` separator count (0 or 1).
    pub separators: usize,
    /// Edge lines carrying a third (label) token.
    pub labels: usize,
}

/// Whether the buffer looks like TGF (numeric node ids + `#` + numeric edge pairs).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut saw_sep = false;
    let mut node_lines = 0usize;
    let mut edge_lines = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr == "#" {
            saw_sep = true;
            continue;
        }
        if tr.is_empty() {
            continue;
        }
        if saw_sep {
            let mut it = tr.split_whitespace();
            let ok = it
                .next()
                .is_some_and(|t| t.chars().all(|c| c.is_ascii_digit()))
                && it
                    .next()
                    .is_some_and(|t| t.chars().all(|c| c.is_ascii_digit()));
            if ok {
                edge_lines += 1;
            }
        } else {
            let first = tr.split_whitespace().next().unwrap_or("");
            if !first.is_empty() && first.chars().all(|c| c.is_ascii_digit()) {
                node_lines += 1;
            }
        }
    }
    saw_sep && node_lines > 0 && edge_lines > 0
}

impl Tgf {
    /// Parses a TGF summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            nodes: 0,
            edges: 0,
            separators: 0,
            labels: 0,
        };
        let mut sep = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr == "#" {
                c.separators += 1;
                sep = true;
                continue;
            }
            if tr.is_empty() {
                continue;
            }
            if sep {
                c.edges += 1;
                if tr.split_whitespace().count() > 2 {
                    c.labels += 1;
                }
            } else {
                c.nodes += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tgf() {
        let b = concat!(
            "1 Alpha\n",
            "2 Beta\n",
            "3 Gamma\n",
            "#\n",
            "1 2 edge-label\n",
            "2 3\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Tgf::parse(b).unwrap();
        assert_eq!(c.nodes, 3);
        assert_eq!(c.edges, 2);
        assert_eq!(c.separators, 1);
        assert_eq!(c.labels, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"1 a\n2 b\n"));
        assert!(Tgf::parse(b"x").is_none());
    }
}
