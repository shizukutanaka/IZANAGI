//! Census of an AWS Cedar policy file (`.cedar` / `.cedarschema`).
//!
//! Cedar policies: `permit`/`forbid` heads, `principal`/`action`/`resource`
//! scope clauses (bare, `==`, `in`, `is`, `has`), `when { … }`/`unless { … }`
//! conditions, entity refs `Namespace::Type::"id"` and `Uid { … }`,
//! `context` conditions, operators `&&`/`||`/`!`/`==`/`!=`/`in`/`like`/`has`,
//! `principal is User::Group`/`resource in Folder::"x"`, set literals
//! `[a, b]`, record `{ k: v }`, `//` comments.
//!
//! ```rust
//! let c = izanagi_kit::cedar::Cedar::parse(
//!     b"permit(principal, action, resource) when { principal.isAdmin };\n",
//! ).unwrap();
//! assert_eq!(c.permits, 1);
//! ```
#![forbid(unsafe_code)]

/// Cedar policy census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cedar {
    /// `permit(` policies.
    pub permits: usize,
    /// `forbid(` policies.
    pub forbids: usize,
    /// `when {`/`unless {` condition blocks.
    pub conditions: usize,
    /// `::` entity-type references seen.
    pub entities: usize,
    /// `principal`/`action`/`resource`/`context` scope tokens.
    pub scopes: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

/// Scope clause keywords.
const SCOPE: &[&str] = &["principal", "action", "resource", "context"];

/// True if `b` looks like Cedar.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("permit(") || t.contains("forbid("))
        && t.contains("principal")
        && t.contains("resource")
}

impl Cedar {
    /// Parse a `.cedar` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            permits: 0,
            forbids: 0,
            conditions: 0,
            entities: 0,
            scopes: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("//") || l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if l.starts_with("permit") {
                c.permits += 1;
            } else if l.starts_with("forbid") {
                c.forbids += 1;
            }
            if l.contains("when") || l.contains("unless") {
                c.conditions += 1;
            }
            for s in SCOPE {
                if l.contains(s) {
                    c.scopes += 1;
                }
            }
            c.entities += l.matches("::").count();
        }
        if c.permits + c.forbids == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "// cedar\n",
            "permit(principal, action, resource) when {\n",
            "    principal is User::Admin\n",
            "};\n",
            "forbid(principal, action, resource) when {\n",
            "    resource in Folder::\"secret\" && !context.ip.inIp(\"10.0.0.0/8\")\n",
            "};\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Cedar::parse(b.as_bytes()).unwrap();
        assert_eq!(c.permits, 1);
        assert_eq!(c.forbids, 1);
        assert_eq!(c.conditions, 2);
        assert!(c.entities >= 2);
        assert!(c.scopes >= 4);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"fn main() {}\n"));
        assert!(Cedar::parse(b"# none\n").is_none());
    }
}
