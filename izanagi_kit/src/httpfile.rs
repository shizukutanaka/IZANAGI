//! REST Client `.http` / `.rest` request files —
//! `###`-separated `METHOD url` blocks with `@var =` variables and headers.

use core::str::from_utf8;

const METHODS: [&str; 8] = [
    "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "GRAPHQL",
];

#[derive(Debug, Clone)]
/// Parsed census of a `.http`/`.rest` file.
pub struct Httpfile {
    /// `@name = value` variable assignments.
    pub variables: usize,
    /// Request lines (`GET https://…`).
    pub requests: usize,
    /// `###` separators / comments.
    pub separators: usize,
    /// `# comment` / `// comment` lines.
    pub comments: usize,
    /// `Header-Name: value` lines inside requests.
    pub headers: usize,
    /// Request bodies (non-empty line runs after a blank line in a block).
    pub bodies: usize,
    /// Requests using `{{var}}` interpolation.
    pub interpolations: usize,
    /// `> {%` response handler scripts.
    pub handlers: usize,
    /// Requests to `https://` URLs.
    pub https: usize,
    /// Distinct method tokens seen.
    pub method_kinds: usize,
    /// Requests carrying query strings (`?x=`).
    pub queries: usize,
    /// Multipart/form markers (`boundary=`/`multipart`).
    pub multipart: usize,
}

fn is_request(l: &str) -> bool {
    let mut it = l.split_whitespace();
    it.next().is_some_and(|m| METHODS.contains(&m)) && it.next().is_some()
}

/// `true` when the text looks like a `.http` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    t.lines().filter(|l| is_request(l.trim())).count() >= 1
        && (t.contains("###")
            || t.lines()
                .any(|l| l.trim_start().starts_with('@') && l.contains('='))
            || t.contains("{{"))
}

impl Httpfile {
    #[must_use]
    /// Parses `b` into `Httpfile`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut r = Self {
            variables: 0,
            requests: 0,
            separators: 0,
            comments: 0,
            headers: 0,
            bodies: 0,
            interpolations: 0,
            handlers: 0,
            https: 0,
            method_kinds: 0,
            queries: 0,
            multipart: 0,
        };
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut in_req = false;
        let mut body = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                body = in_req;
                continue;
            }
            if let Some(rest) = tr.strip_prefix('@') {
                if rest.contains('=') {
                    r.variables += 1;
                    continue;
                }
            }
            if tr.starts_with("###") {
                r.separators += 1;
                in_req = false;
                body = false;
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("//") {
                r.comments += 1;
                continue;
            }
            if tr.starts_with("> {%") {
                r.handlers += 1;
                continue;
            }
            if tr.contains("{{") {
                r.interpolations += 1;
            }
            if tr.contains("https://") {
                r.https += 1;
            }
            if is_request(tr) {
                r.requests += 1;
                in_req = true;
                body = false;
                let m = tr.split_whitespace().next().unwrap_or("");
                seen.insert(m);
                if tr.contains("?") && tr.contains('=') {
                    r.queries += 1;
                }
                continue;
            }
            if in_req && !body && tr.contains(':') && !tr.contains("://") {
                r.headers += 1;
                if tr.contains("multipart") || tr.contains("boundary=") {
                    r.multipart += 1;
                }
                continue;
            }
            if body {
                r.bodies += 1;
                body = false;
            }
        }
        r.method_kinds = seen.len();
        Some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"@host = https://api.example.com\n@token = abc\n\n### list users\nGET {{host}}/users?limit=2\nAccept: application/json\nAuthorization: Bearer {{token}}\n\n### create\nPOST {{host}}/users\nContent-Type: application/json\n\n{\"name\": \"x\"}\n";

    #[test]
    fn detects_httpfile() {
        assert!(detect(FIX));
        assert!(!detect(b"just text"));
    }

    #[test]
    fn parses_httpfile() {
        let h = Httpfile::parse(FIX).unwrap();
        assert_eq!(h.variables, 2);
        assert_eq!(h.requests, 2);
        assert_eq!(h.separators, 2);
        assert_eq!(h.headers, 3);
        assert_eq!(h.bodies, 1);
        assert_eq!(h.queries, 1);
        assert_eq!(h.method_kinds, 2);
        assert!(Httpfile::parse(b"").is_none());
    }
}
