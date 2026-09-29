//! Neo4j Cypher query — `MATCH`/`OPTIONAL MATCH`/`WHERE`/`RETURN`/`CREATE`/
//! `MERGE`/`SET`/`DELETE`/`DETACH`/`WITH`/`UNWIND`/`CALL`/`UNION` + ASCII-art
//! patterns `(a)-[:R]->(b)`.
//!
//! ```
//! let d = b"MATCH (n:Person)-[:KNOWS]->(m) WHERE n.age > 30 RETURN n, m";
//! let c = izanagi_kit::cypher::parse(d).unwrap();
//! assert_eq!(c.matches, 1);
//! assert_eq!(c.returns, 1);
//! assert_eq!(c.rels, 2);
//! assert!(izanagi_kit::cypher::detect(d));
//! ```

/// Census of a Cypher query.
#[derive(Debug, Clone)]
pub struct Cypher {
    /// `MATCH` clauses.
    pub matches: usize,
    /// `OPTIONAL` blocks (`OPTIONAL MATCH`).
    pub optionals: usize,
    /// `WHERE` clauses.
    pub wheres: usize,
    /// `RETURN` clauses.
    pub returns: usize,
    /// `CREATE` clauses.
    pub creates: usize,
    /// `MERGE` clauses.
    pub merges: usize,
    /// `SET` clauses.
    pub sets: usize,
    /// `DELETE`/`DETACH` clauses.
    pub deletes: usize,
    /// `WITH` projection clauses.
    pub withs: usize,
    /// `UNWIND` clauses.
    pub unwinds: usize,
    /// `CALL` subqueries/procedures.
    pub calls: usize,
    /// `UNION`/`UNION ALL`.
    pub unions: usize,
    /// `(n`/`(:Label` node-pattern opens.
    pub nodes: usize,
    /// `-[`/`-[r:`/`-->` relationship arrows.
    pub rels: usize,
    /// `ORDER BY`/`SKIP`/`LIMIT` tail clauses.
    pub tails: usize,
    /// `CASE`/`AND`/`OR`/`NOT`/`IN` predicates.
    pub predicates: usize,
    /// `DISTINCT` keyword.
    pub distinct: usize,
    /// `ASC`/`DESC` ordering.
    pub orderings: usize,
}

fn kw(t: &str, k: &str) -> usize {
    let mut n = 0;
    let mut from = 0;
    while let Some(slice) = t.get(from..) {
        let Some(p) = slice.find(k) else {
            break;
        };
        let a = from + p;
        let before = a == 0
            || t[..a]
                .chars()
                .last()
                .is_some_and(|c| !c.is_alphanumeric() && c != '_');
        let after = t[a + k.len()..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric() && c != '_');
        if before && after {
            n += 1;
        }
        from = a + k.len();
    }
    n
}

/// Detects Cypher: a match/create keyword plus `(` pattern or `RETURN`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (kw(t, "MATCH") + kw(t, "CREATE") + kw(t, "MERGE") > 0 && t.contains('('))
        || (kw(t, "RETURN") > 0 && t.contains('-'))
}

/// Parses a Cypher query; `None` on non-UTF-8 or missing keywords.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Cypher> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Cypher {
        matches: kw(t, "MATCH"),
        optionals: kw(t, "OPTIONAL"),
        wheres: kw(t, "WHERE"),
        returns: kw(t, "RETURN"),
        creates: kw(t, "CREATE"),
        merges: kw(t, "MERGE"),
        sets: kw(t, "SET"),
        deletes: kw(t, "DELETE") + kw(t, "DETACH"),
        withs: kw(t, "WITH"),
        unwinds: kw(t, "UNWIND"),
        calls: kw(t, "CALL"),
        unions: kw(t, "UNION"),
        nodes: t.matches('(').count(),
        rels: t.matches("-[").count() + t.matches("->").count(),
        tails: kw(t, "ORDER") + kw(t, "SKIP") + kw(t, "LIMIT"),
        predicates: kw(t, "CASE") + kw(t, "AND") + kw(t, "OR") + kw(t, "NOT") + kw(t, "IN"),
        distinct: kw(t, "DISTINCT"),
        orderings: kw(t, "ASC") + kw(t, "DESC"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"MATCH (n:Person)-[:KNOWS]->(m:Person) WHERE n.age > 30 AND m.age < 90 OPTIONAL MATCH (n)-[:LIVES_IN]->(c) RETURN DISTINCT n, m ORDER BY n.name DESC LIMIT 10";

    #[test]
    fn parses() {
        let c = parse(D).unwrap();
        assert_eq!(c.matches, 2); // MATCH + OPTIONAL MATCH
        assert_eq!(c.wheres, 1);
        assert_eq!(c.returns, 1);
        assert_eq!(c.optionals, 1);
        assert_eq!(c.nodes, 4);
        assert!(c.rels >= 2);
        assert_eq!(c.distinct, 1);
        assert_eq!(c.orderings, 1);
        assert!(c.predicates >= 1);
    }

    #[test]
    fn mutations() {
        let c = parse(b"CREATE (n:X {a:1}) MERGE (m:Y) ON MATCH SET m.b = 2 RETURN m").unwrap();
        assert_eq!(c.creates, 1);
        assert_eq!(c.merges, 1);
        assert_eq!(c.sets, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"CREATE (n)"));
        assert!(!detect(b"select *"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RETURN 1").is_none());
    }
}
