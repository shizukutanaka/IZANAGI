//! GDF (GUESS Data Format) — `nodedef> col,col,…` header, node rows,
//! `edgedef> node1,node2,…` header, edge rows (CSV-with-types dialect).
//!
//! ```
//! let d = b"nodedef> name VARCHAR, label VARCHAR, weight DOUBLE\nn0, a, 1\nn1, b, 2\nedgedef> node1 VARCHAR, node2 VARCHAR, directed BOOLEAN\nn0, n1, true\n";
//! let g = izanagi_kit::gdf::parse(d).unwrap();
//! assert_eq!(g.nodes, 2);
//! assert_eq!(g.edges, 1);
//! assert_eq!(g.node_columns, 3);
//! assert!(izanagi_kit::gdf::detect(d));
//! ```

/// A parsed GDF census.
#[derive(Debug, Clone)]
pub struct Gdf {
    /// Columns declared by `nodedef>`.
    pub node_columns: usize,
    /// Columns declared by `edgedef>`.
    pub edge_columns: usize,
    /// Node data rows.
    pub nodes: usize,
    /// Edge data rows.
    pub edges: usize,
    /// `directed` column present in edgedef.
    pub directed_column: bool,
    /// `weight`-like columns (name contains `weight`).
    pub weight_columns: usize,
    /// Distinct SQL-ish type names used in headers.
    pub distinct_types: usize,
}

const TYPES: &[&str] = &[
    "VARCHAR", "INT", "INTEGER", "DOUBLE", "FLOAT", "BOOLEAN", "TINYINT", "SMALLINT", "BIGINT",
    "DATE", "TIME", "DATETIME",
];

/// Detects a GDF file: a `nodedef>` header and an `edgedef>` header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut node = false;
    let mut edge = false;
    for line in t.lines() {
        let l = line.trim_start();
        if l.starts_with("nodedef>") {
            node = true;
        } else if l.starts_with("edgedef>") {
            edge = true;
        }
    }
    node && edge
}

/// Parses a GDF file into a census of declared columns and data rows.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Gdf> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut g = Gdf {
        node_columns: 0,
        edge_columns: 0,
        nodes: 0,
        edges: 0,
        directed_column: false,
        weight_columns: 0,
        distinct_types: 0,
    };
    let mut types: Vec<String> = Vec::new();
    #[derive(Clone, Copy, PartialEq)]
    enum Sec {
        None,
        Node,
        Edge,
    }
    let mut sec = Sec::None;
    for line in t.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(hdr) = l.strip_prefix("nodedef>") {
            sec = Sec::Node;
            g.node_columns = hdr.split(',').count();
            for col in hdr.split(',') {
                let mut it = col.split_whitespace();
                let _name = it.next().unwrap_or("");
                let ty = it.next().unwrap_or("").to_ascii_uppercase();
                if !ty.is_empty() && !types.contains(&ty) {
                    types.push(ty);
                }
                if col.to_ascii_lowercase().contains("weight") {
                    g.weight_columns += 1;
                }
            }
            continue;
        }
        if let Some(hdr) = l.strip_prefix("edgedef>") {
            sec = Sec::Edge;
            g.edge_columns = hdr.split(',').count();
            for col in hdr.split(',') {
                let mut it = col.split_whitespace();
                let name = it.next().unwrap_or("");
                let ty = it.next().unwrap_or("").to_ascii_uppercase();
                if !ty.is_empty() && !types.contains(&ty) {
                    types.push(ty);
                }
                if name.eq_ignore_ascii_case("directed") {
                    g.directed_column = true;
                }
                if col.to_ascii_lowercase().contains("weight") {
                    g.weight_columns += 1;
                }
            }
            continue;
        }
        match sec {
            Sec::Node => g.nodes += 1,
            Sec::Edge => g.edges += 1,
            Sec::None => {}
        }
    }
    // Keep only recognised type names so arbitrary words don't inflate the set.
    g.distinct_types = types
        .iter()
        .filter(|t2| TYPES.contains(&t2.as_str()))
        .count();
    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"nodedef> name VARCHAR, label VARCHAR\nn0, a\nn1, b\nedgedef> node1 VARCHAR, node2 VARCHAR, weight DOUBLE, directed BOOLEAN\nn0, n1, 1, true\nn1, n0, 2, false\n";

    #[test]
    fn detects_gdf() {
        assert!(detect(D));
        assert!(!detect(b"a,b,c\n1,2,3\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_census() {
        let g = parse(D).unwrap();
        assert_eq!(g.node_columns, 2);
        assert_eq!(g.edge_columns, 4);
        assert_eq!(g.nodes, 2);
        assert_eq!(g.edges, 2);
        assert!(g.directed_column);
        assert_eq!(g.weight_columns, 1);
        assert!(g.distinct_types >= 3);
    }
}
