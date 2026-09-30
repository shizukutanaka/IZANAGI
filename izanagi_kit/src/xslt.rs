//! XSLT stylesheet `.xsl`/`.xslt` — `<xsl:stylesheet>` or
//! `<xsl:transform>` with `version="…"` and `template`/`apply-templates`
//! `for-each`/`if`/`choose`/`when`/`otherwise`/`variable`/`param`
//! `call-template`/`value-of`/`copy-of`/`text`/`number`/`sort`/`key`
//! `output`/`import`/`include`/`strip-space` instructions.
//!
//! ```
//! let d = b"<xsl:stylesheet xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\" version=\"1.0\">\
//! <xsl:template match=\"/\"><xsl:value-of select=\"a\"/></xsl:template></xsl:stylesheet>";
//! let x = izanagi_kit::xslt::parse(d).unwrap();
//! assert_eq!(x.stylesheet, true);
//! assert_eq!(x.version, Some(3));
//! assert_eq!(x.templates, 1);
//! assert_eq!(x.value_ofs, 1);
//! assert!(izanagi_kit::xslt::detect(d));
//! ```

/// Census of an XSLT stylesheet.
#[derive(Debug, Clone)]
pub struct Xslt {
    /// `stylesheet` (vs `transform`) root seen.
    pub stylesheet: bool,
    /// `version="…"` attribute value length.
    pub version: Option<usize>,
    /// `<*:template` declarations.
    pub templates: usize,
    /// `<*:apply-templates` instructions.
    pub apply_templates: usize,
    /// `<*:for-each` loops.
    pub for_eaches: usize,
    /// `<*:if` conditionals.
    pub ifs: usize,
    /// `<*:choose` branches.
    pub chooses: usize,
    /// `<*:when` branches.
    pub whens: usize,
    /// `<*:otherwise` branches.
    pub otherwises: usize,
    /// `<*:variable` bindings.
    pub variables: usize,
    /// `<*:param` parameters.
    pub params: usize,
    /// `<*:call-template` calls.
    pub call_templates: usize,
    /// `<*:value-of` instructions.
    pub value_ofs: usize,
    /// `<*:copy-of` instructions.
    pub copy_ofs: usize,
    /// `<*:text` output.
    pub texts: usize,
    /// `<*:sort` keys.
    pub sorts: usize,
    /// `<*:key` declarations.
    pub keys: usize,
    /// `<*:output` declarations.
    pub outputs: usize,
    /// `<*:import` + `<*:include`.
    pub external: usize,
}

const BOUND: &[char] = &[' ', '\t', '\n', '/', '>', ':'];

fn count_tag(t: &str, name: &str) -> usize {
    let mut n = 0;
    let mut from = 0;
    while let Some(slice) = t.get(from..) {
        let Some(p) = slice.find(name) else {
            break;
        };
        let a = from + p;
        let close = t[..a]
            .rfind('<')
            .is_some_and(|i| t[i + 1..].starts_with('/'));
        let bounded = !close
            && t[..a].chars().last().is_some_and(|c| c == '<' || c == ':')
            && t[a + name.len()..]
                .chars()
                .next()
                .is_some_and(|c| BOUND.contains(&c));
        if bounded {
            n += 1;
        }
        from = a + name.len();
    }
    n
}

fn attr<'a>(t: &'a str, k: &str) -> Option<&'a str> {
    let at = t.find(k)? + k.len();
    let r = t.get(at..)?.trim_start();
    let r = r.strip_prefix('=')?.trim_start();
    let q = r.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let e = r[1..].find(q)?;
    Some(&r[1..1 + e])
}

/// Detects an XSLT sheet: `stylesheet`/`transform` root + XSL namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains(":stylesheet") || t.contains(":transform")) && t.contains("XSL")
}

/// Parses an `.xsl`; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xslt> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Xslt {
        stylesheet: t.contains("stylesheet"),
        version: attr(t, "version").map(str::len),
        templates: count_tag(t, "template"),
        apply_templates: count_tag(t, "apply-templates"),
        for_eaches: count_tag(t, "for-each"),
        ifs: count_tag(t, "if"),
        chooses: count_tag(t, "choose"),
        whens: count_tag(t, "when"),
        otherwises: count_tag(t, "otherwise"),
        variables: count_tag(t, "variable"),
        params: count_tag(t, "param"),
        call_templates: count_tag(t, "call-template"),
        value_ofs: count_tag(t, "value-of"),
        copy_ofs: count_tag(t, "copy-of"),
        texts: count_tag(t, "text"),
        sorts: count_tag(t, "sort"),
        keys: count_tag(t, "key"),
        outputs: count_tag(t, "output"),
        external: count_tag(t, "import") + count_tag(t, "include"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<xsl:stylesheet xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\" version=\"1.0\"><xsl:output method=\"xml\"/><xsl:key name=\"k\" match=\"m\" use=\"u\"/><xsl:param name=\"p\"/><xsl:template match=\"/\"><xsl:for-each select=\"i\"><xsl:if test=\"t\"><xsl:value-of select=\"v\"/></xsl:if><xsl:choose><xsl:when test=\"w\"/><xsl:otherwise/></xsl:choose><xsl:sort/><xsl:text>x</xsl:text></xsl:for-each><xsl:apply-templates select=\"n\"/><xsl:call-template name=\"h\"/><xsl:copy-of select=\"c\"/><xsl:variable name=\"y\"/></xsl:template><xsl:include href=\"o.xsl\"/><xsl:import href=\"i.xsl\"/></xsl:stylesheet>";

    #[test]
    fn parses() {
        let x = parse(D).unwrap();
        assert!(x.stylesheet);
        assert_eq!(x.version, Some(3));
        assert_eq!(x.templates, 1);
        assert_eq!(x.apply_templates, 1);
        assert_eq!(x.for_eaches, 1);
        assert_eq!(x.ifs, 1);
        assert_eq!(x.chooses, 1);
        assert_eq!(x.whens, 1);
        assert_eq!(x.otherwises, 1);
        assert_eq!(x.variables, 1);
        assert_eq!(x.params, 1);
        assert_eq!(x.call_templates, 1);
        assert_eq!(x.value_ofs, 1);
        assert_eq!(x.copy_ofs, 1);
        assert_eq!(x.texts, 1);
        assert_eq!(x.sorts, 1);
        assert_eq!(x.keys, 1);
        assert_eq!(x.outputs, 1);
        assert_eq!(x.external, 2);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(
            b"<xsl:transform xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\"/>"
        ));
        assert!(!detect(b"<xsl:stylesheet/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<other/>").is_none());
    }
}
