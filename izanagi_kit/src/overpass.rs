//! Overpass QL クエリ 検出モジュール。
//!
//! Overpass API (OpenStreetMap クエリ) の QL 形式は
//! `[out:json]`/`[timeout:25]` 設定ブロック、`node`/`way`/
//! `relation`/`area`/`nwr`/`nw`/`wr` のクエリ文、`out`/`out body`/
//! `out skel qt` 出力文、`.` 参照、`_` 入力、`->` 代入で構成される。
//!
//! ```
//! let b = br#"[out:json][timeout:25];
//! node["amenity"="cafe"]({{bbox}});
//! way["amenity"="cafe"]({{bbox}});
//! out center;
//! "#;
//! let c = izanagi_kit::overpass::parse(b);
//! assert!(izanagi_kit::overpass::detect(b));
//! assert_eq!(c.statements, 4);
//! ```

const STATEMENTS: &[&str] = &[
    "area",
    "count",
    "difference",
    "foreach",
    "if",
    "is_in",
    "make",
    "map_to_area",
    "node",
    "nwr",
    "nw",
    "out",
    "polyline",
    "rel",
    "relation",
    "timeline",
    "way",
    "wr",
];

fn is_query_stmt(t: &str) -> bool {
    // `stmt[...]` または `stmt;` または `stmt expr;`
    let head: String = t
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || *c == '_' || *c == '.')
        .collect();
    !head.is_empty()
        && STATEMENTS.contains(&head.as_str())
        && t[head.len()..].starts_with(['[', '(', ' ', ';', '{', '.'])
}

fn is_setting(t: &str) -> bool {
    t.starts_with("[out:")
        || t.starts_with("[timeout:")
        || t.starts_with("[date:")
        || t.starts_with("[bbox:")
        || t.starts_with("[maxsize:")
        || t.starts_with("[adiff:")
        || t.starts_with("[diff:")
}

/// `b` が Overpass QL クエリに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut stmts = 0usize;
    let mut settings = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("//") || tr.starts_with("/*") {
            continue;
        }
        if is_setting(tr) {
            settings += 1;
            stmts += 1;
        } else if is_query_stmt(tr) || tr == "out;" || tr.starts_with("out ") {
            stmts += 1;
        }
    }
    (settings >= 1 && stmts >= 2) || stmts >= 3
}

/// Overpass QL クエリの統計。
#[derive(Debug, Default, Clone)]
pub struct Overpass {
    /// クエリ文/設定行数。
    pub statements: usize,
    /// `[key:value]` 設定ブロック行数。
    pub settings: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Overpass QL クエリとして統計する。
pub fn parse(b: &[u8]) -> Overpass {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Overpass::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with("//") || tr.starts_with("/*") {
            c.comments += 1;
            continue;
        }
        if is_setting(tr) {
            c.settings += 1;
            c.statements += 1;
        } else if is_query_stmt(tr) || tr == "out;" || tr.starts_with("out ") {
            c.statements += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"[out:json];
node["amenity"="cafe"];
out;
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.statements, 3);
    }

    #[test]
    fn detects_out_only() {
        let b = br#"node[highway=bus_stop];
way[highway=bus_stop];
out;
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[out:json];\n"));
        assert!(!detect(b"node\nway\nout\n")); // no semicolons? still matches
        assert!(!detect(b"SELECT * FROM cafes\nWHERE amenity = 'cafe';\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.statements, 0);
    }
}
