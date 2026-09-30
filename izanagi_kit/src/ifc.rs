//! IFC (Industry Foundation Classes) — buildingSMART's BIM exchange
//! model, serialized as an ISO 10303-21 (STEP) physical file:
//! `ISO-10303-21;` … `HEADER; FILE_DESCRIPTION… FILE_SCHEMA(('IFC4'));`
//! `END_HEADER; DATA; #id=IFCWALL(…); … ENDSEC; END-ISO-10303-21;`.
//!
//! `parse` requires the STEP prolog, the `FILE_SCHEMA` schema name to
//! contain `IFC`, and a `DATA` section; it counts `#n=ENTITY(...)`
//! instances.
//!
//! ```
//! let f = b"ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC4'));\nEND_HEADER;\nDATA;\n#1=IFCWALL('g');\n#2=IFCDOOR('d');\nENDSEC;\nEND-ISO-10303-21;\n";
//! let i = izanagi_kit::ifc::parse(f).unwrap();
//! assert_eq!(i.schema, "IFC4");
//! assert_eq!(i.instances, 2);
//! assert!(izanagi_kit::ifc::parse(b"ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('AUTOMOTIVE'));\nEND_HEADER;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n").is_none());
//! ```

/// Parsed IFC file summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Ifc {
    /// Schema declared in `FILE_SCHEMA(('…'))`, e.g. `IFC4`, `IFC2X3`.
    pub schema: String,
    /// Number of `#id=ENTITY(...)` instance statements.
    pub instances: usize,
    /// True when a `DATA` section was present.
    pub has_data: bool,
}

fn schema(s: &str) -> Option<String> {
    let i = s.find("FILE_SCHEMA(('")?;
    let rest = s[i + 13..].trim_start_matches('\'');
    let e = rest.find('\'')?;
    Some(rest[..e].to_string())
}

/// Parse an IFC STEP file; `None` without IFC schema + `DATA`.
pub fn parse(d: &[u8]) -> Option<Ifc> {
    let s = std::str::from_utf8(d).ok()?;
    if !s.trim_start().starts_with("ISO-10303-21;") {
        return None;
    }
    let schema = schema(s)?;
    if !schema.contains("IFC") {
        return None;
    }
    let has_data = s.contains("DATA;");
    if !has_data {
        return None;
    }
    let mut instances = 0usize;
    for line in s.lines() {
        let l = line.trim_start();
        if l.starts_with('#') && l.contains('=') {
            instances += 1;
        }
    }
    Some(Ifc {
        schema,
        instances,
        has_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC2X3'));\nEND_HEADER;\nDATA;\n#1=IFCWALL('g');\nENDSEC;\nEND-ISO-10303-21;\n";

    #[test]
    fn basic() {
        let i = parse(DOC).unwrap();
        assert_eq!(i.schema, "IFC2X3");
        assert_eq!(i.instances, 1);
        assert!(i.has_data);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"HEADER;\nFILE_SCHEMA(('IFC4'));\n").is_none());
        assert!(parse(b"ISO-10303-21;\nFILE_SCHEMA(('IFC4'));\n").is_none()); // no DATA
    }
}
