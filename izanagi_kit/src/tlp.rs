//! Tulip `.tlp` — `(tlp "2.x"` Lisp-style graph file with `(nb_nodes …)`,
//! `(nodes …)`, `(edge id s t)`, `(property …)` and `(cluster …)` forms.
//!
//! ```
//! let d = b"(tlp \"2\x2e0\"\n(date \"26-09-2026\")\n(nb_nodes 3)\n(nodes 0 1 2)\n(nb_edges 2)\n(edge 1 0 1)\n(edge 2 1 2)\n(property 0 int \"viewMetric\"\n(default \"0\" \"0\"))\n)\n";
//! let g = izanagi_kit::tlp::parse(d).unwrap();
//! assert_eq!(g.nb_nodes, 3);
//! assert_eq!(g.edges, 2);
//! assert_eq!(g.version, "2.0");
//! assert!(izanagi_kit::tlp::detect(d));
//! ```

/// A parsed Tulip `.tlp` census.
#[derive(Debug, Clone)]
pub struct Tlp {
    /// `tlp` header version string.
    pub version: String,
    /// `(nb_nodes N)` declared count.
    pub nb_nodes: usize,
    /// `(nb_edges N)` declared count.
    pub nb_edges: usize,
    /// `(nodes …)` statements.
    pub node_stmts: usize,
    /// Node ids listed across `(nodes …)` forms.
    pub node_ids: usize,
    /// `(edge …)` statements.
    pub edges: usize,
    /// `(property …)` statements.
    pub properties: usize,
    /// `(cluster …)` statements.
    pub clusters: usize,
    /// `(comments …)`/other parenthesised forms.
    pub other_forms: usize,
    /// Deepest `(` nesting level.
    pub max_depth: usize,
    /// `;;` comments.
    pub comments: usize,
}

/// Detects a Tulip file: begins with `(tlp`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.trim_start().starts_with("(tlp")
}

/// Parses a `.tlp` file into a census of forms and counts.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Tlp> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut g = Tlp {
        version: String::new(),
        nb_nodes: 0,
        nb_edges: 0,
        node_stmts: 0,
        node_ids: 0,
        edges: 0,
        properties: 0,
        clusters: 0,
        other_forms: 0,
        max_depth: 0,
        comments: 0,
    };
    let mut depth = 0usize;
    let mut i = 0usize;
    let tb = t.as_bytes();
    let mut in_str = false;
    while i < tb.len() {
        match tb[i] {
            b';' if !in_str => {
                g.comments += 1;
                while i < tb.len() && tb[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'"' => in_str = !in_str,
            b'(' if !in_str => {
                depth += 1;
                g.max_depth = g.max_depth.max(depth);
                // Form name = next word after `(`.
                let mut j = i + 1;
                while j < tb.len() && tb[j].is_ascii_whitespace() {
                    j += 1;
                }
                let s = j;
                while j < tb.len() && !tb[j].is_ascii_whitespace() && tb[j] != b'(' && tb[j] != b')'
                {
                    j += 1;
                }
                let name = &t[s..j];
                // First argument (string or integer) follows.
                let mut k = j;
                while k < tb.len() && tb[k].is_ascii_whitespace() {
                    k += 1;
                }
                let mut arg_num = 0usize;
                let arg_str;
                if k < tb.len() && tb[k] == b'"' {
                    let q = k + 1;
                    let mut e = q;
                    while e < tb.len() && tb[e] != b'"' {
                        e += 1;
                    }
                    arg_str = &t[q..e.min(tb.len())];
                } else {
                    let s2 = k;
                    while k < tb.len() && tb[k].is_ascii_digit() {
                        k += 1;
                    }
                    arg_str = &t[s2..k];
                    arg_num = arg_str.parse().unwrap_or(0);
                }
                match name {
                    "tlp" => g.version = arg_str.to_string(),
                    "nb_nodes" => g.nb_nodes = arg_num,
                    "nb_edges" => g.nb_edges = arg_num,
                    "nodes" => g.node_stmts += 1,
                    "edge" => g.edges += 1,
                    "property" => g.properties += 1,
                    "cluster" => g.clusters += 1,
                    _ => {
                        g.other_forms += 1;
                    }
                }
            }
            b')' if !in_str => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    // Count node ids inside `(nodes …)` lists on a second cheap pass.
    for line in t.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("(nodes") {
            let rest = rest.trim_start();
            let rest = rest.trim_end_matches(')');
            g.node_ids += rest
                .split_whitespace()
                .filter(|w| !w.is_empty() && w.bytes().all(|c| c.is_ascii_digit() || c == b'-'))
                .count();
        }
    }
    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"(tlp \"2\x2e0\"\n(nb_nodes 3)\n(nodes 0 1 2)\n(nb_edges 2)\n(edge 1 0 1)\n(edge 2 1 2)\n)\n";

    #[test]
    fn detects_tlp() {
        assert!(detect(D));
        assert!(!detect(b"(cluster 0)"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_census() {
        let g = parse(D).unwrap();
        assert_eq!(g.version, "2.0");
        assert_eq!(g.nb_nodes, 3);
        assert_eq!(g.nb_edges, 2);
        assert_eq!(g.edges, 2);
        assert_eq!(g.node_ids, 3);
        assert!(g.max_depth >= 1);
    }
}
