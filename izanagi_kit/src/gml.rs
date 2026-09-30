//! Graph Modelling Language (GML) — nested `key [ … ]` bracket blocks:
//! `graph [ directed 1 node [ id 0 label "a" ] edge [ source 0 target 1 ] ]`.
//!
//! ```
//! let d = b"Creator \"gml2pajek\"\ngraph [\n directed 1\n node [\n  id 0\n  label \"a\"\n ]\n node [\n  id 1\n  label \"b\"\n ]\n edge [\n  source 0\n  target 1\n  weight 7\n ]\n]\n";
//! let g = izanagi_kit::gml::parse(d).unwrap();
//! assert_eq!(g.nodes, 2);
//! assert_eq!(g.edges, 1);
//! assert!(g.directed);
//! assert!(izanagi_kit::gml::detect(d));
//! ```

/// A parsed GML document census.
#[derive(Debug, Clone)]
pub struct Gml {
    /// `graph [ … ]` top-level blocks.
    pub graphs: usize,
    /// `node [ … ]` entries.
    pub nodes: usize,
    /// `edge [ … ]` entries.
    pub edges: usize,
    /// `directed 1` flag seen (defaults undirected).
    pub directed: bool,
    /// `id N` keys inside node blocks.
    pub ids: usize,
    /// `label "…"` keys.
    pub labels: usize,
    /// `source`/`target` endpoint keys.
    pub endpoints: usize,
    /// `weight` keys.
    pub weights: usize,
    /// Distinct lowercase key names seen.
    pub distinct_keys: usize,
    /// Deepest `[` nesting level.
    pub max_depth: usize,
    /// Quoted-string values seen.
    pub strings: usize,
}

/// Splits `b` into `(kind, text)` word tokens: brackets, quoted strings, ids/numbers.
fn tokens(b: &[u8]) -> Vec<(u8, &[u8])> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'[' | b']' => {
                out.push((b[i], &b[i..i + 1]));
                i += 1;
            }
            b'"' => {
                let s = i;
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += 1;
                }
                out.push((b'"', &b[s..i.min(b.len())]));
                i = i.saturating_add(1).min(b.len());
            }
            _ => {
                let s = i;
                while i < b.len()
                    && !matches!(b[i], b' ' | b'\t' | b'\r' | b'\n' | b'[' | b']' | b'"')
                {
                    i += 1;
                }
                out.push((b'w', &b[s..i]));
            }
        }
    }
    out
}

/// Detects GML: first token is `graph` (or `Version`/`Creator` header word)
/// followed within a few tokens by `[`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let toks = tokens(b);
    // Scan the first handful of tokens for `graph` then `[`.
    let mut seen_graph = false;
    for (k, w) in toks.iter().take(24) {
        if *k == b'w' && w.eq_ignore_ascii_case(b"graph") {
            seen_graph = true;
        }
        if seen_graph && *k == b'[' {
            return true;
        }
    }
    false
}

/// Parses a GML document into a census of blocks, keys, and values.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Gml> {
    if !detect(b) {
        return None;
    }
    let toks = tokens(b);
    let mut g = Gml {
        graphs: 0,
        nodes: 0,
        edges: 0,
        directed: false,
        ids: 0,
        labels: 0,
        endpoints: 0,
        weights: 0,
        distinct_keys: 0,
        max_depth: 0,
        strings: 0,
    };
    let mut keys: Vec<&[u8]> = Vec::new();
    let mut depth = 0usize;
    let mut prev_word: &[u8] = b"";
    let mut prev_kind = 0u8;
    for (k, w) in &toks {
        match *k {
            b'[' => {
                depth += 1;
                g.max_depth = g.max_depth.max(depth);
                if prev_kind == b'w' {
                    if prev_word.eq_ignore_ascii_case(b"graph") {
                        g.graphs += 1;
                    } else if prev_word.eq_ignore_ascii_case(b"node") {
                        g.nodes += 1;
                    } else if prev_word.eq_ignore_ascii_case(b"edge") {
                        g.edges += 1;
                    }
                }
            }
            b']' => depth = depth.saturating_sub(1),
            b'"' => {
                g.strings += 1;
                if prev_kind == b'w' && prev_word.eq_ignore_ascii_case(b"label") {
                    g.labels += 1;
                }
            }
            _ => {
                if !keys.iter().any(|k2| k2.eq_ignore_ascii_case(w)) {
                    keys.push(w);
                }
                g.distinct_keys = keys.len();
                if w.eq_ignore_ascii_case(b"directed") {
                    // Peek at the next word token handled on the next pass.
                } else if w.eq_ignore_ascii_case(b"id") {
                    g.ids += 1;
                } else if w.eq_ignore_ascii_case(b"source") || w.eq_ignore_ascii_case(b"target") {
                    g.endpoints += 1;
                } else if w.eq_ignore_ascii_case(b"weight") {
                    g.weights += 1;
                } else if prev_kind == b'w'
                    && prev_word.eq_ignore_ascii_case(b"directed")
                    && *w == b"1"
                {
                    g.directed = true;
                }
            }
        }
        prev_kind = *k;
        prev_word = w;
    }
    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"graph [\n directed 1\n node [ id 0 label \"a\" ]\n node [ id 1 label \"b\" ]\n edge [ source 0 target 1 ]\n]\n";

    #[test]
    fn detects_gml() {
        assert!(detect(D));
        assert!(detect(b"graph [ node [ id 0 ] ]"));
        assert!(!detect(b"[graph 1]"));
        assert!(!detect(b""));
        assert!(!detect(b"graphology 5"));
    }

    #[test]
    fn parses_census() {
        let g = parse(D).unwrap();
        assert_eq!(g.graphs, 1);
        assert_eq!(g.nodes, 2);
        assert_eq!(g.edges, 1);
        assert!(g.directed);
        assert_eq!(g.ids, 2);
        assert_eq!(g.labels, 2);
        assert_eq!(g.endpoints, 2);
        assert!(g.max_depth >= 2);
    }

    #[test]
    fn undirected_default() {
        let g = parse(b"graph [ directed 0 node [ id 0 ] ]").unwrap();
        assert!(!g.directed);
    }
}
