//! Astropy ECSV (`*.ecsv`) census.
//!
//! `# %ECSV` magic, YAML comment header (`delimiter:`, `datatype:`,
//! `meta:`, `schema:`), a CSV column header and comma-separated data rows.
//!
//! ```
//! let s = b"# %ECSV 1\n# ---\n# delimiter: ','\n# datatype: [{name: ra, datatype: int64, unit: deg}, {name: dec, datatype: string}]\n# meta: !!omap\n# schema: ecsv\nra,dec\n1,2\n3,4\n";
//! assert!(izanagi_kit::ecsv::detect(s));
//! let e = izanagi_kit::ecsv::Ecsv::parse(s).unwrap();
//! assert_eq!(e.version, 1);
//! assert_eq!(e.columns, 2);
//! assert_eq!(e.datatypes, 2);
//! assert_eq!(e.data_rows, 2);
//! assert_eq!(e.comments, 6);
//! ```

/// Parsed census of an ECSV file.
#[derive(Debug, Clone)]
pub struct Ecsv {
    /// `# %ECSV` version digits, or 0.
    pub version: usize,
    /// `# ` comment/header lines.
    pub comments: usize,
    /// Columns in the CSV header row.
    pub columns: usize,
    /// `{name:` datatype entries.
    pub datatypes: usize,
    /// `datatype:` values.
    pub type_names: usize,
    /// `unit:` entries.
    pub units: usize,
    /// `description:` entries.
    pub descriptions: usize,
    /// `subtype:` entries.
    pub subtypes: usize,
    /// `meta:` lines.
    pub meta: usize,
    /// `schema:` lines.
    pub schema: usize,
    /// `delimiter:` lines.
    pub delimiters: usize,
    /// `!!omap`/YAML tag markers.
    pub yaml_tags: usize,
    /// CSV data rows after the header.
    pub data_rows: usize,
}

/// Reports whether `b` looks like an ECSV file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("# %ECSV")
}

impl Ecsv {
    /// Parses `b` as an ECSV file, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut e = Ecsv {
            version: 0,
            comments: 0,
            columns: 0,
            datatypes: 0,
            type_names: 0,
            units: 0,
            descriptions: 0,
            subtypes: 0,
            meta: 0,
            schema: 0,
            delimiters: 0,
            yaml_tags: 0,
            data_rows: 0,
        };
        let mut saw_header = false;
        for l in t.lines() {
            if l.starts_with('#') {
                e.comments += 1;
                if l.starts_with("# %ECSV") {
                    e.version = l
                        .chars()
                        .skip_while(|c| !c.is_ascii_digit())
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse()
                        .unwrap_or(0);
                }
                if l.contains("delimiter:") {
                    e.delimiters += 1;
                }
                if l.contains("meta:") {
                    e.meta += 1;
                }
                if l.contains("schema:") {
                    e.schema += 1;
                }
                if l.contains("!!omap") || l.contains("!core") || l.contains("!") && l.contains(":")
                {
                    e.yaml_tags += 1;
                }
                e.datatypes += l.matches("{name:").count();
                e.type_names += l.matches("datatype:").count();
                e.units += l.matches("unit:").count();
                e.descriptions += l.matches("description:").count();
                e.subtypes += l.matches("subtype:").count();
                continue;
            }
            if !saw_header && !l.trim().is_empty() {
                e.columns = l.split(',').count();
                saw_header = true;
                continue;
            }
            if !l.trim().is_empty() {
                e.data_rows += 1;
            }
        }
        Some(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"# %ECSV 1\n# ---\n# delimiter: ','\n# datatype: [{name: ra, datatype: int64, unit: deg}, {name: dec, datatype: string}]\n# meta: !!omap\n# schema: ecsv\nra,dec\n1,2\n3,4\n";

    #[test]
    fn parses_ecsv() {
        assert!(detect(S));
        let e = Ecsv::parse(S).unwrap();
        assert_eq!(e.version, 1);
        assert_eq!(e.columns, 2);
        assert_eq!(e.datatypes, 2);
        assert_eq!(e.data_rows, 2);
        assert_eq!(e.comments, 6);
    }

    #[test]
    fn rejects_non_ecsv() {
        assert!(!detect(b"a,b\n1,2\n"));
        assert!(Ecsv::parse(b"").is_none());
    }
}
