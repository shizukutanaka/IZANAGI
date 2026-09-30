//! Hoppscotch REST collection JSON — `hoppscotch-collection.json`
//! `{ "v": 6, "name": …, "folders": […], "requests": […] }`.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of a Hoppscotch collection.
pub struct Hoppscotch {
    /// `"v"` schema version.
    pub version: usize,
    /// Collection `"name"`.
    pub name: String,
    /// `"requests"` objects (counted by `"method"` keys at request depth).
    pub requests: usize,
    /// `"folders"` objects (counted by `"name"` keys inside folders —
    /// approximated by `"v"` + `"name"` pairs after the collection root).
    pub folders: usize,
    /// `"headers"` arrays.
    pub header_lists: usize,
    /// `"params"` arrays.
    pub param_lists: usize,
    /// `"variables"` arrays.
    pub variables: usize,
    /// `"auth"` objects.
    pub auths: usize,
    /// `"body"` objects.
    pub bodies: usize,
    /// `"endpoint"` strings.
    pub endpoints: usize,
    /// `"testScript"` / `"preRequestScript"` scripts.
    pub scripts: usize,
}

fn jstr<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let pat = ["\"", key, "\""].concat();
    let i = t.find(&pat)?;
    let r = &t[i + pat.len()..];
    let r = r.trim_start_matches(|c: char| c.is_whitespace() || c == ':');
    if let Some(r) = r.strip_prefix('"') {
        let end = r.find('"')?;
        return Some(&r[..end]);
    }
    let end = r.find(|c: char| c == ',' || c == '}' || c == ']' || c.is_whitespace())?;
    Some(&r[..end])
}

/// `true` when the text looks like a Hoppscotch collection.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    t.contains("\"v\"") && t.contains("\"folders\"") && t.contains("\"requests\"")
}

impl Hoppscotch {
    #[must_use]
    /// Parses `b` into `Hoppscotch`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Self {
            version: jstr(t, "v")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0),
            name: jstr(t, "name").unwrap_or("").to_string(),
            requests: t.matches("\"method\"").count(),
            // every object carries "name" — subtract collection root + requests
            folders: t
                .matches("\"name\"")
                .count()
                .saturating_sub(t.matches("\"method\"").count() + 1),
            header_lists: t.matches("\"headers\"").count(),
            param_lists: t.matches("\"params\"").count(),
            variables: t.matches("\"variables\"").count(),
            auths: t.matches("\"auth\"").count(),
            bodies: t.matches("\"body\"").count(),
            endpoints: t.matches("\"endpoint\"").count(),
            scripts: t.matches("\"testScript\"").count()
                + t.matches("\"preRequestScript\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = br#"{
  "v": 6,
  "name": "shop",
  "folders": [ { "v": 6, "name": "users", "requests": [ { "v": "1", "name": "list", "method": "GET", "endpoint": "https://x/u", "headers": [], "params": [] } ] } ],
  "requests": [ { "v": "1", "name": "ping", "method": "POST", "endpoint": "https://x/p", "headers": [], "params": [], "body": {}, "auth": {}, "preRequestScript": "", "testScript": "" } ],
  "variables": []
}"#;

    #[test]
    fn detects_hoppscotch() {
        assert!(detect(FIX));
        assert!(!detect(b"{}"));
    }

    #[test]
    fn parses_hoppscotch() {
        let h = Hoppscotch::parse(FIX).unwrap();
        assert_eq!(h.version, 6);
        assert_eq!(h.name, "shop");
        assert_eq!(h.requests, 2);
        assert_eq!(h.folders, 1);
        assert_eq!(h.header_lists, 2);
        assert_eq!(h.param_lists, 2);
        assert_eq!(h.endpoints, 2);
        assert_eq!(h.scripts, 2);
        assert!(Hoppscotch::parse(b"").is_none());
    }
}
