//! XQuery `.xq`/`.xqy` — prolog `xquery version`/`declare`
//! `namespace`/`variable`/`function`/`module`/`import`, FLWOR
//! `for`/`let`/`where`/`order by`/`return`, and `fn:` calls.
//!
//! ```
//! let d = b"xquery version \"3.1\";\ndeclare variable $x := 1;\n\
//! for $i in (1,2) let $j := $i return $j\n";
//! let q = izanagi_kit::xq::parse(d).unwrap();
//! assert_eq!(q.version, Some(3));
//! assert_eq!(q.declares, 1);
//! assert_eq!(q.fors, 1);
//! assert_eq!(q.lets, 1);
//! assert_eq!(q.returns, 1);
//! assert!(izanagi_kit::xq::detect(d));
//! ```

/// Census of an XQuery module.
#[derive(Debug, Clone)]
pub struct Xq {
    /// `xquery version "N…"` value length.
    pub version: Option<usize>,
    /// `declare` prolog statements.
    pub declares: usize,
    /// `declare namespace` statements.
    pub namespaces: usize,
    /// `declare variable` statements.
    pub variables: usize,
    /// `declare function` definitions.
    pub functions: usize,
    /// `module namespace` (library modules).
    pub modules: usize,
    /// `import module`/`import schema`.
    pub imports: usize,
    /// `for` FLWOR clauses.
    pub fors: usize,
    /// `let` FLWOR clauses.
    pub lets: usize,
    /// `where` clauses.
    pub wheres: usize,
    /// `order by` clauses.
    pub order_bys: usize,
    /// `return` clauses.
    pub returns: usize,
    /// `if`/`then`/`else` expressions (count of `if (`).
    pub ifs: usize,
    /// `fn:`-prefixed function calls.
    pub fn_calls: usize,
    /// `$name` variable references.
    pub var_refs: usize,
    /// `(:` comment starts.
    pub comments: usize,
}

fn xquery_version(t: &str) -> Option<&str> {
    let at = t.find("xquery version")? + "xquery version".len();
    let r = t.get(at..)?.trim_start();
    let q = r.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let e = r[1..].find(q)?;
    Some(&r[1..1 + e])
}

/// Detects XQuery: `xquery version` or `declare` + FLWOR/`fn:` shape.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("xquery version")
        || (t.contains("declare") && (t.contains("return") || t.contains("fn:")))
        || (t.contains("for $") && t.contains("return"))
}

/// Parses an `.xq`; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xq> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut var_refs = 0;
    for w in t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')) {
        if w.starts_with('$') && w.len() > 1 {
            var_refs += 1;
        }
    }
    Some(Xq {
        version: xquery_version(t).map(str::len),
        declares: t.matches("declare").count(),
        namespaces: t.matches("declare namespace").count(),
        variables: t.matches("declare variable").count(),
        functions: t.matches("declare function").count(),
        modules: t.matches("module namespace").count(),
        imports: t.matches("import module").count() + t.matches("import schema").count(),
        fors: t.matches("for $").count() + t.matches("for(").count(),
        lets: t.matches("let $").count() + t.matches("let(").count(),
        wheres: t.matches("where ").count() + t.matches("where\t").count(),
        order_bys: t.matches("order by").count(),
        returns: t.matches("return").count(),
        ifs: t.matches("if (").count() + t.matches("if(").count(),
        fn_calls: t.matches("fn:").count(),
        var_refs,
        comments: t.matches("(:").count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"xquery version \"3.1\";\ndeclare namespace n=\"u\";\ndeclare variable $x := 1;\ndeclare function f($a) { $a };\nimport module \"m\";\n(: comment :)\nfor $i in (1,2) let $j := $i where $j > 0 order by $j return fn:concat($j, $x)\nif (1) then 2 else 3\n";

    #[test]
    fn parses() {
        let q = parse(D).unwrap();
        assert_eq!(q.version, Some(3));
        assert!(q.declares >= 3);
        assert_eq!(q.namespaces, 1);
        assert_eq!(q.variables, 1);
        assert_eq!(q.functions, 1);
        assert_eq!(q.imports, 1);
        assert_eq!(q.fors, 1);
        assert_eq!(q.lets, 1);
        assert_eq!(q.wheres, 1);
        assert_eq!(q.order_bys, 1);
        assert!(q.returns >= 1);
        assert_eq!(q.ifs, 1);
        assert_eq!(q.fn_calls, 1);
        assert!(q.var_refs >= 5);
        assert_eq!(q.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"for $x in (1) return $x"));
        assert!(!detect(b"plain text"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"select * from t").is_none());
    }
}
