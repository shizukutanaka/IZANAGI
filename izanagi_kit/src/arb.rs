//! Flutter ARB (Application Resource Bundle) — JSON where top-level
//! `key` members are messages and `@key` members carry that message's
//! metadata (`description`, `placeholders`). `@@locale` names the file
//! locale.
//!
//! ```
//! use izanagi_kit::arb::parse;
//!
//! let d = br#"{"@@locale": "en", "hello": "Hello", "@hello": {"description": "d"}}"#;
//! let a = parse(d).unwrap();
//! assert_eq!(a.locale.as_deref(), Some("en"));
//! assert_eq!(a.messages[0].key, "hello");
//! ```

use crate::json::{parse as jparse, Json};
use std::collections::BTreeMap;

/// One message with its optional `@`-metadata object.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Message key.
    pub key: String,
    /// Message string.
    pub value: String,
    /// `@key`'s `description` if present.
    pub description: Option<String>,
    /// `@key`'s `placeholders` member names.
    pub placeholders: Vec<String>,
}

/// Parsed ARB file.
#[derive(Debug, Clone)]
pub struct Arb {
    /// `@@locale` if present.
    pub locale: Option<String>,
    /// Messages in sorted key order (the underlying object is a `BTreeMap`).
    pub messages: Vec<Entry>,
}

/// Parse ARB. `None` when the JSON is invalid, the root is not an
/// object, or a non-`@` member is not a string.
pub fn parse(d: &[u8]) -> Option<Arb> {
    let j = jparse(d).ok()?;
    let Json::Obj(map) = j else {
        return None;
    };
    let mut locale = None;
    let mut metas: BTreeMap<String, &BTreeMap<String, Json>> = BTreeMap::new();
    for (k, v) in &map {
        if k == "@@locale" {
            if let Json::Str(s) = v {
                locale = Some(s.clone());
            }
            continue;
        }
        if let Json::Obj(m) = v {
            if let Some(name) = k.strip_prefix('@') {
                metas.insert(name.to_string(), m);
            }
        }
    }
    let mut messages = Vec::new();
    for (k, v) in &map {
        if k.starts_with('@') {
            continue;
        }
        let Json::Str(value) = v else {
            return None;
        };
        let meta = metas.get(k.as_str());
        let mut description = None;
        let mut placeholders = Vec::new();
        if let Some(m) = meta {
            if let Some(Json::Str(s)) = m.get("description") {
                description = Some(s.clone());
            }
            if let Some(Json::Obj(ph)) = m.get("placeholders") {
                placeholders = ph.keys().cloned().collect();
            }
        }
        messages.push(Entry {
            key: k.clone(),
            value: value.clone(),
            description,
            placeholders,
        });
    }
    Some(Arb { locale, messages })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = br#"{"greet": "Hi {name}", "@greet": {"placeholders": {"name": {}}},
        "bye": "Bye", "@x": {"description": "orphan"}}"#;
        let a = parse(d).unwrap();
        assert_eq!(a.messages.len(), 2);
        assert_eq!(a.messages[0].key, "bye");
        assert_eq!(a.messages[1].placeholders, vec!["name"]);
        assert!(a.locale.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"[]").is_none());
        assert!(parse(b"{\"k\": 3}").is_none());
        assert!(parse(b"nope").is_none());
    }
}
