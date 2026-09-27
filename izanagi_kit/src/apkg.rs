//! Anki `.apkg` decks — a ZIP containing `collection.anki2` (SQLite)
//! and a `media` JSON map of `{ordinal: filename}`. Verified through
//! `crate::zip` + `crate::json`; the SQLite payload is only located.
//!
//! ```
//! use izanagi_kit::apkg::parse;
//!
//! let mut z = izanagi_kit::zip::ZipWriter::new();
//! z.add_stored("collection.anki2", b"SQLite format 3\0");
//! z.add_stored("media", br#"{"0":"a.png"}"#);
//! let a = parse(&z.finish()).unwrap();
//! assert_eq!(a.media.get("0").map(|s| s.as_str()), Some("a.png"));
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// Parsed `.apkg` container info.
#[derive(Clone, Debug)]
pub struct Apkg {
    /// `collection.anki2` payload bytes (the SQLite database).
    pub collection: Vec<u8>,
    /// `media` JSON map: `"0"` → `"file.png"`.
    pub media: BTreeMap<String, String>,
    /// All member names (for `collection.anki21` etc.).
    pub members: Vec<String>,
    /// True when the collection begins with the SQLite magic.
    pub sqlite_valid: bool,
}

/// Parse an `.apkg` file.
pub fn parse(d: &[u8]) -> Option<Apkg> {
    let entries = crate::zip::list(d)?;
    let members: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
    let collection = crate::zip::extract(d, "collection.anki2")
        .or_else(|| crate::zip::extract(d, "collection.anki21"))?;
    let mut media = BTreeMap::new();
    if let Some(m) = crate::zip::extract(d, "media") {
        if let Ok(crate::json::Json::Obj(map)) = crate::json::parse(&m) {
            for (k, v) in map {
                if let crate::json::Json::Str(f) = v {
                    media.insert(k, f);
                }
            }
        }
    }
    let sqlite_valid = collection.starts_with(b"SQLite format 3\0");
    Some(Apkg {
        collection,
        media,
        members,
        sqlite_valid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck() {
        let mut z = crate::zip::ZipWriter::new();
        z.add_stored("collection.anki2", b"SQLite format 3\0rest");
        z.add_stored("media", br#"{"0":"a.png","1":"b.jpg"}"#);
        let a = parse(&z.finish()).unwrap();
        assert!(a.sqlite_valid);
        assert_eq!(a.media.len(), 2);
        assert!(a.members.contains(&"media".to_string()));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"PK\x05\x06notazip").is_none());
        let mut z = crate::zip::ZipWriter::new();
        z.add_stored("other.txt", b"x");
        assert!(parse(&z.finish()).is_none()); // no collection
    }
}
