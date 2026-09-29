//! SPARQL 1.1 query — `PREFIX`/`BASE`/`SELECT`/`ASK`/`CONSTRUCT`/`DESCRIBE`
//! forms with `WHERE`/`OPTIONAL`/`FILTER`/`UNION`/`ORDER BY`/`LIMIT`/`OFFSET`/
//! `BIND`/`VALUES`/`MINUS`/`SERVICE` clauses.
//!
//! ```
//! let d = b"PREFIX foaf: <http://xmlns.com/foaf/>\nSELECT ?n WHERE { ?x foaf:name ?n } LIMIT 5";
//! let s = izanagi_kit::sparql::parse(d).unwrap();
//! assert!(s.select);
//! assert_eq!(s.prefixes, 1);
//! assert_eq!(s.limit, Some(5));
//! assert!(izanagi_kit::sparql::detect(d));
//! ```

/// Census of a SPARQL query.
#[derive(Debug, Clone)]
pub struct Sparql {
    /// `PREFIX` declarations.
    pub prefixes: usize,
    /// `BASE` declaration.
    pub base: usize,
    /// `SELECT` form.
    pub select: bool,
    /// `ASK` form.
    pub ask: bool,
    /// `CONSTRUCT` form.
    pub construct: bool,
    /// `DESCRIBE` form.
    pub describe: bool,
    /// `WHERE` keyword occurrences.
    pub wheres: usize,
    /// `OPTIONAL` blocks.
    pub optionals: usize,
    /// `FILTER` expressions.
    pub filters: usize,
    /// `UNION` branches.
    pub unions: usize,
    /// `ORDER BY` clauses.
    pub order_bys: usize,
    /// `LIMIT n` value (first).
    pub limit: Option<u32>,
    /// `OFFSET n` value (first).
    pub offset: Option<u32>,
    /// `BIND` assignments.
    pub binds: usize,
    /// `VALUES` clauses.
    pub values: usize,
    /// `MINUS` blocks.
    pub minuses: usize,
    /// `SERVICE` remote calls.
    pub services: usize,
    /// `?`/`$` variables.
    pub variables: usize,
    /// `a`/`rdf:type` triple-predicate `a` token count.
    pub type_as: usize,
    /// `GRAPH`/`FROM`/`NAMED` dataset keywords.
    pub datasets: usize,
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

fn num_after(t: &str, k: &str) -> Option<u32> {
    let at = t.find(k)? + k.len();
    let rest = t[at..].trim_start();
    let e = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..e].parse().ok()
}

/// Detects a SPARQL query: a form keyword plus braces or `PREFIX`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    kw(t, "ASK") > 0
        || kw(t, "DESCRIBE") > 0
        || kw(t, "CONSTRUCT") > 0
        || (kw(t, "SELECT") > 0 && (kw(t, "WHERE") > 0 || t.contains('{')))
        || kw(t, "PREFIX") > 0
}

/// Parses a SPARQL query; `None` on non-UTF-8 or missing forms.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Sparql> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Sparql {
        prefixes: kw(t, "PREFIX"),
        base: kw(t, "BASE"),
        select: kw(t, "SELECT") > 0,
        ask: kw(t, "ASK") > 0,
        construct: kw(t, "CONSTRUCT") > 0,
        describe: kw(t, "DESCRIBE") > 0,
        wheres: kw(t, "WHERE"),
        optionals: kw(t, "OPTIONAL"),
        filters: kw(t, "FILTER"),
        unions: kw(t, "UNION"),
        order_bys: kw(t, "ORDER"),
        limit: num_after(t, "LIMIT"),
        offset: num_after(t, "OFFSET"),
        binds: kw(t, "BIND"),
        values: kw(t, "VALUES"),
        minuses: kw(t, "MINUS"),
        services: kw(t, "SERVICE"),
        variables: t.matches('?').count() + t.matches('$').count(),
        type_as: kw(t, "a"),
        datasets: kw(t, "GRAPH") + kw(t, "FROM") + kw(t, "NAMED"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"PREFIX foaf: <http://xmlns.com/foaf/>\nSELECT ?n ?m WHERE { ?x foaf:name ?n . OPTIONAL { ?x foaf:mbox ?m } FILTER(?n != \"\") } ORDER BY ?n LIMIT 5 OFFSET 2";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.prefixes, 1);
        assert!(s.select);
        assert_eq!(s.wheres, 1);
        assert_eq!(s.optionals, 1);
        assert_eq!(s.filters, 1);
        assert_eq!(s.order_bys, 1);
        assert_eq!(s.limit, Some(5));
        assert_eq!(s.offset, Some(2));
        assert!(s.variables >= 3);
    }

    #[test]
    fn forms() {
        assert!(parse(b"ASK { ?s ?p ?o }").unwrap().ask);
        assert!(
            parse(b"CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")
                .unwrap()
                .construct
        );
        assert!(parse(b"DESCRIBE <http://x>").unwrap().describe);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"PREFIX a: <b>"));
        assert!(!detect(b"select * from t"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello").is_none());
    }
}
