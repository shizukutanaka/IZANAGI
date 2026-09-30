//! IVOA ADQL (`*.adql`) query census.
//!
//! Case-insensitive keyword census: `SELECT`/`TOP`/`FROM`/`WHERE`/`JOIN`
//! clauses plus geometry builtins (`CONTAINS`, `POINT`, `CIRCLE`, `BOX`,
//! `POLYGON`, `REGION`, `AREA`, `CENTROID`, `COORD1`, `COORD2`,
//! `DISTANCE`, `INTERSECTS`) and `'ICRS'`/`'GALACTIC'` frames.
//!
//! ```
//! let s = b"SELECT TOP 10 * FROM ivoa.ObsCore WHERE CONTAINS(POINT('ICRS', 12, 34), CIRCLE('ICRS', 12, 34, 1)) = 1 AND s_region IS NOT NULL ORDER BY ra";
//! assert!(izanagi_kit::adql::detect(s));
//! let a = izanagi_kit::adql::Adql::parse(s).unwrap();
//! assert_eq!(a.selects, 1);
//! assert_eq!(a.contains, 1);
//! assert_eq!(a.points, 1);
//! assert_eq!(a.circles, 1);
//! assert_eq!(a.icrs, 2);
//! ```

/// Parsed census of an ADQL query.
#[derive(Debug, Clone)]
pub struct Adql {
    /// `SELECT` occurrences.
    pub selects: usize,
    /// `TOP` clauses.
    pub tops: usize,
    /// `DISTINCT` occurrences.
    pub distinct: usize,
    /// `FROM` occurrences.
    pub froms: usize,
    /// `WHERE` occurrences.
    pub wheres: usize,
    /// `JOIN` occurrences.
    pub joins: usize,
    /// `GROUP BY` occurrences.
    pub group_bys: usize,
    /// `ORDER BY` occurrences.
    pub order_bys: usize,
    /// `HAVING` occurrences.
    pub having: usize,
    /// `UNION`/`INTERSECT`/`EXCEPT` set operators.
    pub set_ops: usize,
    /// `CONTAINS(` calls.
    pub contains: usize,
    /// `INTERSECTS(` calls.
    pub intersects: usize,
    /// `POINT(` calls.
    pub points: usize,
    /// `CIRCLE(` calls.
    pub circles: usize,
    /// `BOX(` calls.
    pub boxes: usize,
    /// `POLYGON(` calls.
    pub polygons: usize,
    /// `REGION(` calls.
    pub regions: usize,
    /// `AREA(` calls.
    pub areas: usize,
    /// `CENTROID(` calls.
    pub centroids: usize,
    /// `COORD1(`/`COORD2(` calls.
    pub coords: usize,
    /// `DISTANCE(` calls.
    pub distances: usize,
    /// `'ICRS'` frame references.
    pub icrs: usize,
    /// `'GALACTIC'` frame references.
    pub galactic: usize,
    /// ` AS ` aliases.
    pub aliases: usize,
    /// ` AND ` operators.
    pub ands: usize,
    /// ` OR ` operators.
    pub ors: usize,
    /// `NOT ` operators.
    pub nots: usize,
    /// `LIKE` operators.
    pub likes: usize,
    /// `BETWEEN` operators.
    pub betweens: usize,
    /// ` IN ` operators.
    pub ins: usize,
}

/// Reports whether `b` looks like an ADQL query.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let u = t.to_uppercase();
    u.contains("SELECT")
        && u.contains("FROM")
        && (u.contains("CONTAINS")
            || u.contains("POINT(")
            || u.contains("'ICRS'")
            || u.contains("REGION")
            || u.contains("CIRCLE("))
}

fn count(u: &str, key: &str) -> usize {
    u.matches(key).count()
}

impl Adql {
    /// Parses `b` as an ADQL query, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let u = t.to_uppercase();
        Some(Adql {
            selects: count(&u, "SELECT"),
            tops: count(&u, "TOP "),
            distinct: count(&u, "DISTINCT"),
            froms: count(&u, "FROM"),
            wheres: count(&u, "WHERE"),
            joins: count(&u, "JOIN"),
            group_bys: count(&u, "GROUP BY"),
            order_bys: count(&u, "ORDER BY"),
            having: count(&u, "HAVING"),
            set_ops: count(&u, "UNION") + count(&u, "INTERSECT") - count(&u, "INTERSECTS")
                + count(&u, "EXCEPT"),
            contains: count(&u, "CONTAINS"),
            intersects: count(&u, "INTERSECTS"),
            points: count(&u, "POINT("),
            circles: count(&u, "CIRCLE("),
            boxes: count(&u, "BOX("),
            polygons: count(&u, "POLYGON("),
            regions: count(&u, "REGION("),
            areas: count(&u, "AREA("),
            centroids: count(&u, "CENTROID("),
            coords: count(&u, "COORD1(") + count(&u, "COORD2("),
            distances: count(&u, "DISTANCE("),
            icrs: count(&u, "'ICRS'"),
            galactic: count(&u, "'GALACTIC'"),
            aliases: count(&u, " AS "),
            ands: count(&u, " AND "),
            ors: count(&u, " OR "),
            nots: count(&u, "NOT "),
            likes: count(&u, "LIKE"),
            betweens: count(&u, "BETWEEN"),
            ins: count(&u, " IN "),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"SELECT TOP 10 * FROM ivoa.ObsCore WHERE CONTAINS(POINT('ICRS', 12, 34), CIRCLE('ICRS', 12, 34, 1)) = 1 AND s_region IS NOT NULL ORDER BY ra";

    #[test]
    fn parses_adql() {
        assert!(detect(S));
        let a = Adql::parse(S).unwrap();
        assert_eq!(a.selects, 1);
        assert_eq!(a.contains, 1);
        assert_eq!(a.points, 1);
        assert_eq!(a.circles, 1);
        assert_eq!(a.icrs, 2);
    }

    #[test]
    fn rejects_non_adql() {
        assert!(!detect(b"SELECT 1 FROM t"));
        assert!(Adql::parse(b"").is_none());
    }
}
