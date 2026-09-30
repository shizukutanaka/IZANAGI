//! Amazon Braket IR (JSON) parser.
//!
//! Reads `braketSchemaHeader.name` (`braket.ir.jaqcd.program`,
//! `braket.ir.openqasm.program`, `braket.ir.pulse.*`, ...), then counts
//! `instructions`/`action` entries and `"targets"`/`"target"` occurrences.
//!
//! ```
//! use izanagi_kit::braket::Braket;
//! let src = br#"{"braketSchemaHeader":{"name":"braket.ir.jaqcd.program","version":"1"},"instructions":[{"type":"h","target":0},{"type":"cnot","control":0,"target":1}]}"#;
//! assert!(izanagi_kit::braket::detect(src));
//! let q = Braket::parse(src).unwrap();
//! assert_eq!(q.schema_name, "braket.ir.jaqcd.program");
//! assert_eq!(q.instructions, 2);
//! assert_eq!(q.targets, 3);
//! ```

/// Parsed census of a Braket IR document.
#[derive(Debug, Clone)]
pub struct Braket {
    /// `braketSchemaHeader.name` value.
    pub schema_name: String,
    /// `braketSchemaHeader.version` value.
    pub schema_version: String,
    /// Entries of the `instructions`/`action` array.
    pub instructions: usize,
    /// `"target"`/`"targets"`/`"control"`/`"controls"` operand occurrences.
    pub targets: usize,
    /// `"source"` / `"sourceLocation"` occurrences.
    pub sources: usize,
}

fn jstr<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\":", key);
    let i = t.find(&pat)?;
    let mut s = t[i + pat.len()..].trim_start();
    if !s.starts_with('"') {
        return None;
    }
    s = &s[1..];
    let end = s.find('"')?;
    Some(&s[..end])
}

/// Returns `true` when `b` looks like a Braket IR document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.contains("braketSchemaHeader") || t.contains("\"braket.ir.")
}

impl Braket {
    /// Parses a Braket IR document; `None` without a schema header.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !t.contains("braketSchemaHeader") {
            return None;
        }
        let schema_name = jstr(t, "name").unwrap_or("").to_string();
        let schema_version = jstr(t, "version").unwrap_or("").to_string();
        let typed = t.matches("\"type\":").count();
        let named = t.matches("\"name\":").count().saturating_sub(1);
        let instructions = typed.max(named);
        let targets = t.matches("\"target\":").count()
            + t.matches("\"targets\":").count()
            + t.matches("\"control\":").count()
            + t.matches("\"controls\":").count();
        let sources = t.matches("\"source\":").count() + t.matches("\"sourceLocation\":").count();
        Some(Self {
            schema_name,
            schema_version,
            instructions,
            targets,
            sources,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_header() {
        assert!(detect(
            br#"{"braketSchemaHeader":{"name":"braket.ir.pulse.frame_v1","version":"1"}}"#
        ));
        assert!(!detect(b"{\"name\":\"other\"}"));
    }

    #[test]
    fn counts_ops() {
        let src = br#"{"braketSchemaHeader":{"name":"braket.ir.jaqcd.program","version":"1"},"instructions":[{"type":"rx","angle":1,"target":0}]}"#;
        let q = Braket::parse(src).unwrap();
        assert_eq!(q.instructions, 1);
        assert_eq!(q.targets, 1);
    }
}
