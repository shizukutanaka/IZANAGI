//! IVOA VOTable (`*.vot`) XML census.
//!
//! Counts `RESOURCE`/`TABLE` structure, `FIELD`/`PARAM`/`INFO`/`GROUP`
//! metadata, `COOSYS`/`LINK`/`VALUES` ranges and data payloads
//! (`TABLEDATA`/`TR`/`TD`, `BINARY`, `FITS` references).
//!
//! ```
//! let s = br#"<VOTABLE><RESOURCE><TABLE><FIELD name="ra" datatype="double" ucd="pos.eq.ra" unit="deg"/><FIELD name="dec" datatype="double" ucd="pos.eq.dec"/><PARAM name="t" datatype="int"/><INFO name="q" value="1"/><DATA><TABLEDATA><TR><TD>1</TD><TD>2</TD></TR></TABLEDATA></DATA></TABLE></RESOURCE></VOTABLE>"#;
//! assert!(izanagi_kit::votable::detect(s));
//! let v = izanagi_kit::votable::Votable::parse(s).unwrap();
//! assert_eq!(v.tables, 1);
//! assert_eq!(v.fields, 2);
//! assert_eq!(v.params, 1);
//! assert_eq!(v.trs, 1);
//! assert_eq!(v.tds, 2);
//! ```

/// Parsed census of a VOTable document.
#[derive(Debug, Clone)]
pub struct Votable {
    /// `<RESOURCE` elements.
    pub resources: usize,
    /// `<TABLE` elements.
    pub tables: usize,
    /// `<FIELD` elements.
    pub fields: usize,
    /// `<PARAM` elements.
    pub params: usize,
    /// `<INFO` elements.
    pub infos: usize,
    /// `<DESCRIPTION` elements.
    pub descriptions: usize,
    /// `<GROUP` elements.
    pub groups: usize,
    /// `<LINK` elements.
    pub links: usize,
    /// `<VALUES` elements.
    pub values: usize,
    /// `<MIN` elements.
    pub mins: usize,
    /// `<MAX` elements.
    pub maxs: usize,
    /// `<COOSYS` elements.
    pub coosys: usize,
    /// `<TABLEDATA` elements.
    pub tabledata: usize,
    /// `<TR` row elements.
    pub trs: usize,
    /// `<TD` cell elements.
    pub tds: usize,
    /// `<BINARY` elements.
    pub binary: usize,
    /// `<FITS` elements.
    pub fits: usize,
    /// `<STREAM` elements.
    pub streams: usize,
    /// `ucd=` attributes.
    pub ucds: usize,
    /// `utype=` attributes.
    pub utypes: usize,
    /// `unit=` attributes.
    pub units: usize,
    /// `datatype=` attributes.
    pub datatypes: usize,
    /// `<DEFINITIONS` elements.
    pub definitions: usize,
}

/// Reports whether `b` looks like a VOTable document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("<VOTABLE") || (t.contains("<FIELD") && t.contains("ucd="))
}

fn count(t: &str, key: &str) -> usize {
    t.matches(key).count()
}

impl Votable {
    /// Parses `b` as a VOTable document, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Votable {
            resources: count(t, "<RESOURCE"),
            tables: count(t, "<TABLE") - count(t, "<TABLEDATA"),
            fields: count(t, "<FIELD"),
            params: count(t, "<PARAM"),
            infos: count(t, "<INFO"),
            descriptions: count(t, "<DESCRIPTION"),
            groups: count(t, "<GROUP"),
            links: count(t, "<LINK"),
            values: count(t, "<VALUES"),
            mins: count(t, "<MIN"),
            maxs: count(t, "<MAX"),
            coosys: count(t, "<COOSYS"),
            tabledata: count(t, "<TABLEDATA"),
            trs: count(t, "<TR>") + count(t, "<TR "),
            tds: count(t, "<TD>") + count(t, "<TD "),
            binary: count(t, "<BINARY"),
            fits: count(t, "<FITS"),
            streams: count(t, "<STREAM"),
            ucds: count(t, "ucd="),
            utypes: count(t, "utype="),
            units: count(t, "unit="),
            datatypes: count(t, "datatype="),
            definitions: count(t, "<DEFINITIONS"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = br#"<VOTABLE><RESOURCE><TABLE><FIELD name="ra" datatype="double" ucd="pos.eq.ra" unit="deg"/><FIELD name="dec" datatype="double" ucd="pos.eq.dec"/><PARAM name="t" datatype="int"/><INFO name="q" value="1"/><DATA><TABLEDATA><TR><TD>1</TD><TD>2</TD></TR></TABLEDATA></DATA></TABLE></RESOURCE></VOTABLE>"#;

    #[test]
    fn parses_votable() {
        assert!(detect(S));
        let v = Votable::parse(S).unwrap();
        assert_eq!(v.tables, 1);
        assert_eq!(v.fields, 2);
        assert_eq!(v.params, 1);
        assert_eq!(v.trs, 1);
        assert_eq!(v.tds, 2);
    }

    #[test]
    fn rejects_non_votable() {
        assert!(!detect(b"<xml/>"));
        assert!(Votable::parse(b"{}").is_none());
    }
}
