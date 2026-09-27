//! systemd unit file parsing (INI superset).
//!
//! `[Section]` headers, `Key=Value` lines, `#`/`;` comments,
//! empty `Key=` clears the list, duplicate keys append. Common
//! sections (`Unit`, `Service`, `Install`, `Socket`, `Timer`,
//! `Mount`, `Path`, `Slice`, `Scope`) are recognised.
//!
//! ```
//! use izanagi_kit::systemd;
//! let d = b"[Unit]\nDescription=demo\n\n[Service]\nExecStart=/bin/x\n\n[Install]\nWantedBy=multi-user.target\n";
//! let u = systemd::parse(d).unwrap();
//! assert_eq!(u.get("Service", "ExecStart").unwrap(), "/bin/x");
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// One `Key=Value` directive with its section.
#[derive(Clone, Debug, PartialEq)]
pub struct Directive {
    /// Section name (`Unit`, `Service`…).
    pub section: String,
    /// Key (case-sensitive).
    pub key: String,
    /// Value (may be empty — resets list semantics).
    pub value: String,
    /// 1-based line.
    pub line: usize,
}

/// A parsed unit file.
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    /// Section names in order of first appearance.
    pub sections: Vec<String>,
    /// All directives in file order.
    pub directives: Vec<Directive>,
    /// `(section, key)` → last non-empty value index lookup table.
    pub index: BTreeMap<(String, String), usize>,
}

impl Unit {
    /// Returns the last non-empty value for `section.key`.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.index
            .get(&(section.to_string(), key.to_string()))
            .and_then(|&i| {
                let v = self.directives[i].value.as_str();
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            })
    }
    /// True when `[Install]` is present.
    pub fn installable(&self) -> bool {
        self.sections.iter().any(|s| s == "Install")
    }
}

/// Parses a systemd unit file: needs ≥1 section and ≥1 directive.
pub fn parse(d: &[u8]) -> Option<Unit> {
    let text = std::str::from_utf8(d).ok()?;
    let mut sections: Vec<String> = Vec::new();
    let mut directives = Vec::new();
    let mut index = BTreeMap::new();
    let mut cur = String::new();
    let mut in_section = false;
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if !(line.ends_with(']') && line.len() > 2) {
                return None;
            }
            cur = line[1..line.len() - 1].trim().to_string();
            if cur.is_empty() {
                return None;
            }
            if !sections.iter().any(|s| s == &cur) {
                sections.push(cur.clone());
            }
            in_section = true;
            continue;
        }
        if !in_section {
            return None;
        }
        let (k, v) = line.split_once('=')?;
        let key = k.trim();
        if key.is_empty() {
            return None;
        }
        directives.push(Directive {
            section: cur.clone(),
            key: key.to_string(),
            value: v.trim().to_string(),
            line: i + 1,
        });
        index.insert((cur.clone(), key.to_string()), directives.len() - 1);
    }
    if sections.is_empty() || directives.is_empty() {
        return None;
    }
    Some(Unit {
        sections,
        directives,
        index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT: &[u8] = b"[Unit]\nDescription=x\nAfter=network.target\n\n[Service]\nExecStart=/bin/x\nRestart=always\n\n[Install]\nWantedBy=multi-user.target\n";

    #[test]
    fn parses() {
        let u = parse(UNIT).unwrap();
        assert_eq!(u.sections.len(), 3);
        assert_eq!(u.get("Service", "Restart"), Some("always"));
        assert!(u.installable());
        assert_eq!(u.directives[1].line, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[X\nk=v\n").is_none()); // unclosed section
        assert!(parse(b"key=value\n").is_none()); // directive w/o section
        assert!(parse(b"[Unit]\nnoeq\n").is_none());
        assert!(parse(b"[Unit]\n").is_none()); // section but no directives
    }
}
