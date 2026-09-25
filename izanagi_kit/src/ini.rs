//! INI configuration — [`toml`](crate::toml)'s ancestor form:
//! `key = value` / `key : value` pairs under `[section]` headers, `;`/
//! `#` comments, whitespace-trimmed. Keys before the first header live
//! in the global (empty-name) section. Duplicate keys keep the last
//! value; section and key order are preserved for [`emit`].
//! Parsing is total — every line that isn't a header or `k=v` is a
//! verbatim continuation comment; [`parse`] never fails.
//!
//! ```
//! use izanagi_kit::ini::{Ini, parse};
//! let i = parse("; hi\na=1\n[db]\nport = 5432\n");
//! assert_eq!(i.get(None, "a"), Some("1"));
//! assert_eq!(i.get(Some("db"), "port"), Some("5432"));
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed INI document: insertion-ordered `(section, pairs)`.
/// The pre-first-header keys sit in the entry whose name is `""`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ini {
    /// `(section, (key, value)*)` in file order; `""` = global.
    pub sections: Vec<(String, Vec<(String, String)>)>,
}

impl Ini {
    /// Look up `key` under `section` (`None` = global), scanning
    /// duplicate-key sections first-to-last (last wins).
    pub fn get(&self, section: Option<&str>, key: &str) -> Option<&str> {
        let name = section.unwrap_or("");
        let mut found = None;
        for (s, pairs) in &self.sections {
            if s == name {
                for (k, v) in pairs {
                    if k == key {
                        found = Some(v.as_str());
                    }
                }
            }
        }
        found
    }
    /// Set `key = value` under `section`, appending the section or the
    /// key if absent.
    pub fn set(&mut self, section: Option<&str>, key: &str, value: &str) {
        let name = section.unwrap_or("");
        for (s, pairs) in &mut self.sections {
            if s == name {
                for (k, v) in pairs.iter_mut() {
                    if k == key {
                        *v = value.to_string();
                        return;
                    }
                }
                pairs.push((key.to_string(), value.to_string()));
                return;
            }
        }
        self.sections
            .push((name.to_string(), vec![(key.to_string(), value.to_string())]));
    }
    /// Number of distinct sections (excluding the global one).
    pub fn section_count(&self) -> usize {
        self.sections.iter().filter(|(s, _)| !s.is_empty()).count()
    }
}

/// Parse; never fails — non-conforming lines degrade to nothing.
pub fn parse(src: &str) -> Ini {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut ini = Ini {
        sections: vec![(String::new(), Vec::new())],
    };
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && line.len() >= 3 {
            ini.sections
                .push((line[1..line.len() - 1].trim().to_string(), Vec::new()));
            continue;
        }
        let (k, v) = match line.split_once('=').or_else(|| line.split_once(':')) {
            Some(kv) => kv,
            None => continue, // bare word — skip
        };
        let k = k.trim();
        if k.is_empty() {
            continue;
        }
        let v = v.trim();
        if let Some(pairs) = ini.sections.last_mut().map(|s| &mut s.1) {
            match pairs.iter_mut().find(|(pk, _)| pk == k) {
                Some(kv) => kv.1 = v.to_string(),
                None => pairs.push((k.to_string(), v.to_string())),
            }
        }
    }
    ini
}

/// Canonical emission: `key = value`, `[section]` headers, blank lines.
pub fn emit(ini: &Ini) -> String {
    let mut s = String::new();
    let mut first = true;
    for (name, pairs) in &ini.sections {
        if name.is_empty() && pairs.is_empty() {
            continue;
        }
        if !first {
            s.push('\n');
        }
        first = false;
        if !name.is_empty() {
            s.push('[');
            s.push_str(name);
            s.push_str("]\n");
        }
        for (k, v) in pairs {
            s.push_str(k);
            s.push_str(" = ");
            s.push_str(v);
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "; comment\nhost = example.com\nport = 80\n\n[db]\nname = main\nport = 5432\n[user]\nname = \"sue\"\n";

    #[test]
    fn basic_get() {
        let i = parse(DOC);
        assert_eq!(i.get(None, "host"), Some("example.com"));
        assert_eq!(i.get(None, "port"), Some("80"));
        assert_eq!(i.get(Some("db"), "port"), Some("5432"));
        assert_eq!(i.get(Some("db"), "name"), Some("main"));
        assert_eq!(i.get(Some("user"), "name"), Some("\"sue\"")); // quotes kept
        assert_eq!(i.get(Some("nope"), "x"), None);
        assert_eq!(i.get(None, "name"), None); // section keys don't leak up
    }

    #[test]
    fn set_and_update() {
        let mut i = parse(DOC);
        i.set(Some("db"), "port", "9999");
        assert_eq!(i.get(Some("db"), "port"), Some("9999"));
        i.set(Some("cache"), "ttl", "60"); // new section
        assert_eq!(i.get(Some("cache"), "ttl"), Some("60"));
        i.set(None, "debug", "true"); // new global key
        assert_eq!(i.get(None, "debug"), Some("true"));
    }

    #[test]
    fn last_duplicate_wins() {
        let i = parse("a=1\na=2\n[s]\nx=1\nx=3\n");
        assert_eq!(i.get(None, "a"), Some("2"));
        assert_eq!(i.get(Some("s"), "x"), Some("3"));
    }

    #[test]
    fn colon_and_comments() {
        let i = parse("# c\nk : v ; trailing stays\n");
        assert_eq!(i.get(None, "k"), Some("v ; trailing stays"));
    }

    #[test]
    fn emit_roundtrip() {
        let i = parse(DOC);
        let i2 = parse(&emit(&i));
        assert_eq!(i.get(None, "host"), i2.get(None, "host"));
        assert_eq!(i.get(Some("db"), "port"), i2.get(Some("db"), "port"));
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(DOC), parse(DOC));
        assert_eq!(emit(&parse(DOC)), emit(&parse(DOC)));
    }
}
