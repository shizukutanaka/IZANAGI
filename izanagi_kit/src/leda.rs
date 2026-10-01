//! LEDA `.gw` graph — `LEDA.GRAPH` signature, node/edge label-type lines,
//! `-1`/`-2` direction flag, then node `|…|` records and `(s,t,…)` edges.
//!
//! ```
//! let d = b"LEDA.GRAPH\nstring\nint\n-2\n5\n|{a}|\n|{b}|\n|{c}|\n|{d}|\n|{e}|\n4\n(1,2,0,{3})\n(2,3,0,{4})\n(3,4,0,{5})\n(4,5,0,{6})\n";
//! let g = izanagi_kit::leda::parse(d).unwrap();
//! assert_eq!(g.nodes_declared, 5);
//! assert_eq!(g.edges_declared, 4);
//! assert_eq!(g.edge_records, 4);
//! assert!(!g.directed);
//! assert!(izanagi_kit::leda::detect(d));
//! ```

/// A parsed LEDA graph census.
#[derive(Debug, Clone)]
pub struct Leda {
    /// Node label-type header (`string`/`int`/`void`/…).
    pub node_type: String,
    /// Edge label-type header.
    pub edge_type: String,
    /// `-1` flag (directed) versus `-2` (undirected).
    pub directed: bool,
    /// Node count read from the node-count line.
    pub nodes_declared: usize,
    /// `|…|` node payload records seen.
    pub node_records: usize,
    /// Edge count read from the edge-count line.
    pub edges_declared: usize,
    /// `(s,t,…)` edge tuples seen.
    pub edge_records: usize,
    /// `#Nodes:`/`#Edges:` annotation lines.
    pub annotations: usize,
}

fn first_int(s: &str) -> Option<usize> {
    let mut n = 0usize;
    let mut any = false;
    for c in s.trim().chars() {
        if c.is_ascii_digit() {
            n = n.checked_mul(10)?.checked_add(c as usize - '0' as usize)?;
            any = true;
        } else {
            break;
        }
    }
    any.then_some(n)
}

/// Detects a LEDA graph: file begins with the `LEDA.GRAPH` signature.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"LEDA.GRAPH")
}

/// Parses a LEDA graph into a census of header fields and records.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Leda> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut lines = t.lines();
    let _sig = lines.next();
    let node_type = lines.next().unwrap_or("").trim().to_string();
    let edge_type = lines.next().unwrap_or("").trim().to_string();
    let flag = lines.next().unwrap_or("").trim();
    let directed = flag == "-1";
    let mut g = Leda {
        node_type,
        edge_type,
        directed,
        nodes_declared: 0,
        node_records: 0,
        edges_declared: 0,
        edge_records: 0,
        annotations: 0,
    };
    #[derive(Clone, Copy, PartialEq)]
    enum Phase {
        NodesCount,
        Nodes(usize),
        EdgesCount,
        Edges(usize),
    }
    let mut phase = Phase::NodesCount;
    for line in lines {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if l.starts_with('#') {
            g.annotations += 1;
            continue;
        }
        match phase {
            Phase::NodesCount => {
                if let Some(n) = first_int(l) {
                    g.nodes_declared = n;
                    phase = Phase::Nodes(n);
                }
            }
            Phase::Nodes(n) => {
                if l.starts_with('|') {
                    g.node_records += 1;
                }
                if g.node_records >= n {
                    phase = Phase::EdgesCount;
                }
            }
            Phase::EdgesCount => {
                if let Some(m) = first_int(l) {
                    g.edges_declared = m;
                    phase = Phase::Edges(m);
                }
            }
            Phase::Edges(m) => {
                if l.starts_with('(') || first_int(l).is_some() {
                    g.edge_records += 1;
                }
                if g.edge_records >= m {
                    break;
                }
            }
        }
    }
    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"LEDA.GRAPH\nvoid\nvoid\n-1\n3\n|{}|\n|{}|\n|{}|\n2\n(1,2)\n(2,3)\n";

    #[test]
    fn detects_leda() {
        assert!(detect(D));
        assert!(!detect(b"LEDA\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_census() {
        let g = parse(D).unwrap();
        assert_eq!(g.nodes_declared, 3);
        assert_eq!(g.node_records, 3);
        assert_eq!(g.edges_declared, 2);
        assert_eq!(g.edge_records, 2);
        assert!(g.directed);
    }
}
