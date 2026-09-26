//! Minimal reader for STEP Part 21 exchange files (`ISO-10303-21;`).
//!
//! Structure: a `HEADER;`/`ENDSEC;` section (with `FILE_DESCRIPTION`,
//! `FILE_NAME`, `FILE_SCHEMA`) then `DATA;`/`ENDSEC;` holding
//! `#id = ENTITY(arg, arg, ...);` rows. Args are kept verbatim — no
//! evaluation. `END-ISO-10303-21;` closes the file.
//!
//! ```
//! use izanagi_kit::step::parse;
//!
//! let s = parse(
//!     b"ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\
//!       \nDATA;\n#1 = CARTESIAN_POINT('P', (1., 2., 3.));\n#2 = DIRECTION('', (0., 0., 1.));\nENDSEC;\nEND-ISO-10303-21;",
//! )
//! .unwrap();
//! assert_eq!(s.entities.len(), 2);
//! assert_eq!(s.entities[0].kind, "CARTESIAN_POINT");
//! assert_eq!(s.schema.as_deref(), Some("AUTOMOTIVE_DESIGN"));
//! ```

/// One `#id = KIND(args);` entity.
#[derive(Debug)]
pub struct Entity {
    /// Numeric instance id (`#42` → 42).
    pub id: u64,
    /// Uppercase entity name (`CARTESIAN_POINT`, ...).
    pub kind: String,
    /// Everything between the outer parentheses, verbatim.
    pub args: String,
}

/// A parsed Part-21 file.
#[derive(Debug)]
pub struct Step {
    /// `FILE_SCHEMA(('NAME'))` → first schema name.
    pub schema: Option<String>,
    /// `FILE_NAME('...')` → the name field.
    pub file_name: Option<String>,
    /// Entities from the `DATA;` section, in file order.
    pub entities: Vec<Entity>,
}

fn quoted(s: &str) -> Option<&str> {
    let a = s.find('\'')? + 1;
    let b = s[a..].find('\'')? + a;
    Some(&s[a..b])
}

fn section<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let a = src.find(name)? + name.len();
    let end = src[a..].find("ENDSEC;")? + a;
    Some(&src[a..end])
}

fn entity(line: &str) -> Option<Entity> {
    let line = line.trim();
    let hash = line.strip_prefix('#')?;
    let eq = hash.find('=')?;
    let id: u64 = hash[..eq].trim().parse().ok()?;
    let rest = hash[eq + 1..].trim();
    let rest = rest.strip_suffix(';')?.trim();
    let open = rest.find('(')?;
    let kind = rest[..open].trim();
    if kind.is_empty() || !kind.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
        return None;
    }
    let close = rest.rfind(')')?;
    let args = rest[open + 1..close].to_string();
    Some(Entity {
        id,
        kind: kind.to_string(),
        args,
    })
}

/// Parse a Part-21 file. `None` without the `ISO-10303-21;` banner or a
/// `DATA;` section; malformed `#...` lines also fail.
pub fn parse(data: &[u8]) -> Option<Step> {
    let src = std::str::from_utf8(data).ok()?;
    if !src.contains("ISO-10303-21;") {
        return None;
    }
    let mut step = Step {
        schema: None,
        file_name: None,
        entities: Vec::new(),
    };
    if let Some(header) = section(src, "HEADER;") {
        if let Some(a) = header.find("FILE_SCHEMA(") {
            step.schema = quoted(&header[a + 12..]).map(str::to_string);
        }
        if let Some(a) = header.find("FILE_NAME(") {
            step.file_name = quoted(&header[a + 10..]).map(str::to_string);
        }
    }
    let data = section(src, "DATA;")?;
    for stmt in data.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        step.entities.push(entity(&format!("{stmt};"))?);
    }
    Some(step)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''), '2;1');\nFILE_NAME('cube.step', '');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n#10 = CARTESIAN_POINT('A', (0., 0., 0.));\n#20 = DIRECTION('', (0., 0., 1.));\nENDSEC;\nEND-ISO-10303-21;";

    #[test]
    fn parses() {
        let s = parse(DOC).unwrap();
        assert_eq!(s.schema.as_deref(), Some("AUTOMOTIVE_DESIGN"));
        assert_eq!(s.file_name.as_deref(), Some("cube.step"));
        assert_eq!(s.entities.len(), 2);
        assert_eq!(s.entities[0].id, 10);
        assert_eq!(s.entities[0].kind, "CARTESIAN_POINT");
        assert_eq!(s.entities[0].args, "'A', (0., 0., 0.)");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"not a step file").is_none()); // no banner
        assert!(parse(b"ISO-10303-21;\n").is_none()); // no DATA
                                                      // broken entity line
        assert!(parse(b"ISO-10303-21;\nDATA;\n#x = FOO();\nENDSEC;\n").is_none());
        assert!(parse(b"ISO-10303-21;\nDATA;\nfoo\nENDSEC;\n").is_none());
    }
}
