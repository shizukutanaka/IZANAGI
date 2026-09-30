//! HTTP Archive (HAR) 1.x JSON — `<name>.har`
//! Detects `{ "log": { "version": …, "entries": […] } }` and counts
//! pages, entries, requests, responses, and timings via key scans.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of a HAR archive.
pub struct Har {
    /// `log.version`.
    pub version: String,
    /// `log.creator.name`.
    pub creator: String,
    /// `log.pages[]` entries (counted by `pageTimings` objects).
    pub pages: usize,
    /// `log.entries[]` entries (counted by `startedDateTime`).
    pub entries: usize,
    /// `"method"` occurrences across entries.
    pub methods: usize,
    /// `"headers"` arrays (request + response header lists).
    pub header_lists: usize,
    /// `"response"` objects.
    pub responses: usize,
    /// `"cookies"` arrays.
    pub cookie_lists: usize,
    /// `"postData"` bodies.
    pub post_data: usize,
    /// `"timings"` objects.
    pub timings: usize,
    /// `"cache"` objects.
    pub caches: usize,
    /// `"pageref"` links.
    pub page_refs: usize,
    /// `"comment"` annotations at any depth.
    pub comments: usize,
}

fn jstr<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let pat = ["\"", key, "\""].concat();
    let i = t.find(&pat)?;
    let r = &t[i + pat.len()..];
    let r = r.trim_start_matches(|c: char| c.is_whitespace() || c == ':');
    let r = r.strip_prefix('"')?;
    let end = r.find('"')?;
    Some(&r[..end])
}

/// `true` when the text looks like a HAR archive.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    t.contains("\"log\"") && t.contains("\"entries\"") && t.contains("\"startedDateTime\"")
}

impl Har {
    #[must_use]
    /// Parses `b` into `Har` when it satisfies the HAR shape.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Self {
            version: jstr(t, "version").unwrap_or("").to_string(),
            creator: jstr(t, "name").unwrap_or("").to_string(),
            pages: t.matches("\"pageTimings\"").count(),
            entries: t.matches("\"startedDateTime\"").count(),
            methods: t.matches("\"method\"").count(),
            header_lists: t.matches("\"headers\"").count(),
            responses: t.matches("\"response\"").count(),
            cookie_lists: t.matches("\"cookies\"").count(),
            post_data: t.matches("\"postData\"").count(),
            timings: t.matches("\"timings\"").count(),
            caches: t.matches("\"cache\"").count(),
            page_refs: t.matches("\"pageref\"").count(),
            comments: t.matches("\"comment\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = br#"{
  "log": {
    "version": "1",
    "creator": { "name": "curl", "version": "8" },
    "pages": [ { "id": "p1", "pageTimings": {} } ],
    "entries": [
      {
        "pageref": "p1",
        "startedDateTime": "2024-01-01T00:00:00Z",
        "request": { "method": "GET", "url": "https://x/", "headers": [], "cookies": [] },
        "response": { "status": 200, "headers": [], "cookies": [] },
        "timings": { "send": 1, "wait": 2 },
        "cache": {}
      },
      {
        "startedDateTime": "2024-01-01T00:00:01Z",
        "request": { "method": "POST", "url": "https://x/p", "headers": [], "postData": {} },
        "response": { "status": 201, "headers": [] },
        "timings": { "send": 1, "wait": 2 }
      }
    ]
  }
}"#;

    #[test]
    fn detects_har() {
        assert!(detect(FIX));
        assert!(!detect(b"plain json {}"));
    }

    #[test]
    fn parses_har() {
        let h = Har::parse(FIX).unwrap();
        assert_eq!(h.version, "1");
        assert_eq!(h.creator, "curl");
        assert_eq!(h.pages, 1);
        assert_eq!(h.entries, 2);
        assert_eq!(h.methods, 2);
        assert_eq!(h.header_lists, 4);
        assert_eq!(h.responses, 2);
        assert_eq!(h.cookie_lists, 2);
        assert_eq!(h.post_data, 1);
        assert_eq!(h.timings, 2);
        assert_eq!(h.caches, 1);
        assert_eq!(h.page_refs, 1);
        assert!(Har::parse(b"").is_none());
    }
}
