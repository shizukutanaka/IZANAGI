//! Kusto KQL — `Table | where | project | extend | summarize | sort | take |
//! join | union | count | render` pipeline with `by`, `ago()`, `datatable`,
//! `let`, `==`, `=~`, `!~`, `in`, `between`, `has`, `contains`.
//!
//! ```
//! let d = b"Events | where ts > ago(1d) | summarize c=count() by user | take 10";
//! let k = izanagi_kit::kql::parse(d).unwrap();
//! assert_eq!(k.pipes, 3);
//! assert_eq!(k.wheres, 1);
//! assert_eq!(k.summarizes, 2);
//! assert!(izanagi_kit::kql::detect(d));
//! ```

/// Census of a KQL query.
#[derive(Debug, Clone)]
pub struct Kql {
    /// `|` pipeline stages.
    pub pipes: usize,
    /// `where`/`filter`.
    pub wheres: usize,
    /// `project`/`project-away`/`project-rename`/`project-keep`.
    pub projects: usize,
    /// `extend`/`evaluate`.
    pub extends: usize,
    /// `summarize`/`count`/`distinct`.
    pub summarizes: usize,
    /// `sort`/`order`.
    pub sorts: usize,
    /// `take`/`limit`/`top`.
    pub takes: usize,
    /// `join`/`union`/`mv-expand`/`mv-apply`.
    pub joins: usize,
    /// `render`/`print`.
    pub renders: usize,
    /// `let` statements.
    pub lets: usize,
    /// `by` groupings.
    pub bys: usize,
    /// `ago()`/`now()`/`datetime()`/`timespan` calls.
    pub times: usize,
    /// `==`/`!=`/`=~`/`!~`/`>=`/`<=`/`>`/`<`.
    pub comparisons: usize,
    /// `and`/`or`/`not`/`in`/`between`/`has`/`contains`/`startswith`/`endswith`.
    pub predicates: usize,
    /// `datatable`/`externaldata`/`materialize`.
    pub datatables: usize,
    /// `count()`/`sum()`/`avg()`/`min()`/`max()`/`dcount()`/`percentile*()`.
    pub aggregates: usize,
    /// `asc`/`desc`.
    pub orderings: usize,
}

fn word(t: &str, k: &str) -> usize {
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
                .is_some_and(|c| !c.is_alphanumeric() && c != '_' && c != '-');
        let after = t[a + k.len()..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric() && c != '_' && c != '-');
        if before && after {
            n += 1;
        }
        from = a + k.len();
    }
    n
}

/// Detects KQL: a `|` stage keyword or `let`/`datatable` statement.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains('|')
        && (word(t, "where") > 0
            || word(t, "project") > 0
            || word(t, "summarize") > 0
            || word(t, "extend") > 0
            || word(t, "take") > 0
            || word(t, "count") > 0
            || word(t, "render") > 0)
        || word(t, "datatable") > 0
}

/// Parses a KQL query; `None` on non-UTF-8 or missing pipe keywords.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Kql> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Kql {
        pipes: t.matches('|').count() - t.matches("||").count(),
        wheres: word(t, "where") + word(t, "filter"),
        projects: word(t, "project")
            + word(t, "project-away")
            + word(t, "project-rename")
            + word(t, "project-keep"),
        extends: word(t, "extend") + word(t, "evaluate"),
        summarizes: word(t, "summarize") + word(t, "count") + word(t, "distinct"),
        sorts: word(t, "sort") + word(t, "order"),
        takes: word(t, "take") + word(t, "limit") + word(t, "top"),
        joins: word(t, "join") + word(t, "union") + word(t, "mv-expand") + word(t, "mv-apply"),
        renders: word(t, "render") + word(t, "print"),
        lets: word(t, "let"),
        bys: word(t, "by"),
        times: word(t, "ago") + word(t, "now") + word(t, "datetime") + word(t, "timespan"),
        comparisons: t.matches("==").count()
            + t.matches("!=").count()
            + t.matches("=~").count()
            + t.matches("!~").count()
            + t.matches("<=").count()
            + t.matches(">=").count()
            + (t.matches('<').count() - t.matches("<=").count())
            + (t.matches('>').count() - t.matches(">=").count()),
        predicates: word(t, "and")
            + word(t, "or")
            + word(t, "not")
            + word(t, "in")
            + word(t, "between")
            + word(t, "has")
            + word(t, "contains")
            + word(t, "startswith")
            + word(t, "endswith"),
        datatables: word(t, "datatable") + word(t, "externaldata") + word(t, "materialize"),
        aggregates: word(t, "count")
            + word(t, "sum")
            + word(t, "avg")
            + word(t, "min")
            + word(t, "max")
            + word(t, "dcount")
            + word(t, "percentile"),
        orderings: word(t, "asc") + word(t, "desc"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"let t = datatable(x:int)[1]; Events | where ts > ago(1d) and user has \"a\" | summarize c=count(), m=max(lat) by user | sort by c desc | take 10";

    #[test]
    fn parses() {
        let k = parse(D).unwrap();
        assert_eq!(k.lets, 1);
        assert_eq!(k.datatables, 1);
        assert_eq!(k.pipes, 4);
        assert_eq!(k.wheres, 1);
        assert_eq!(k.summarizes, 2); // summarize + count() occurrence
        assert_eq!(k.bys, 2);
        assert_eq!(k.takes, 1);
        assert_eq!(k.orderings, 1);
        assert!(k.predicates >= 2);
        assert!(k.aggregates >= 2);
        assert!(k.comparisons >= 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"T | count"));
        assert!(detect(b"datatable(x:1)[]"));
        assert!(!detect(b"a | b"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"ls | grep").is_none());
    }
}
