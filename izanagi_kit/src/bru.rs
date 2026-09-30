//! Bruno `.bru` request files — `name { key: value }` block format.
//! `meta {`, `get {`/`post {`/`put {`/`delete {`/`patch {`/`head {`/`options {`,
//! `headers {`, `query {`/`params {`, `body:json {`, `auth:* {`, `script:* {`,
//! `assert {`, `tests {`, `docs {`, `vars:* {`.

use core::str::from_utf8;

const METHODS: [&str; 7] = ["get", "post", "put", "delete", "patch", "head", "options"];

#[derive(Debug, Clone)]
/// Parsed census of a Bruno `.bru` request file.
pub struct Bru {
    /// `meta { name: … }`.
    pub name: String,
    /// `meta { type: http|graphql }`.
    pub meta_type: String,
    /// `meta { seq: N }`.
    pub seq: usize,
    /// HTTP method blocks present (`get {}`…).
    pub method: String,
    /// `url:` inside the method block.
    pub url: String,
    /// All `key { … }` blocks.
    pub blocks: usize,
    /// `key: value` scalar entries.
    pub entries: usize,
    /// `headers {` block.
    pub headers: usize,
    /// `query {`/`params {` blocks.
    pub params: usize,
    /// `body:* {` / `body {` blocks.
    pub bodies: usize,
    /// `auth:* {` blocks.
    pub auths: usize,
    /// `script:pre-request`/`script:post-response` blocks.
    pub scripts: usize,
    /// `assert {`/`tests {` blocks.
    pub checks: usize,
    /// `docs {`/`vars:* {`/`settings {}` blocks.
    pub misc: usize,
}

/// `true` when the text looks like a Bruno file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    let has_meta = t.lines().any(|l| l.trim() == "meta {");
    let has_method = t
        .lines()
        .any(|l| METHODS.iter().any(|m| l.trim() == [*m, " {"].concat()));
    has_meta && has_method
}

fn val_in<'a>(body: &[&'a str], key: &str) -> Option<&'a str> {
    body.iter().find_map(|l| {
        l.trim()
            .strip_prefix(key)
            .map(|r| r.trim_start_matches(':').trim())
    })
}

impl Bru {
    #[must_use]
    /// Parses `b` into `Bru`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut blocks = 0usize;
        let mut entries = 0usize;
        let mut name = String::new();
        let mut meta_type = String::new();
        let mut seq = 0usize;
        let mut method = String::new();
        let mut url = String::new();
        let mut headers = 0;
        let mut params = 0;
        let mut bodies = 0;
        let mut auths = 0;
        let mut scripts = 0;
        let mut checks = 0;
        let mut misc = 0;
        let mut cur: Option<(&str, Vec<&str>)> = None;
        let mut blocks_vec: Vec<(String, Vec<&str>)> = Vec::new();
        for l in t.lines() {
            let tr = l.trim_end();
            if let Some((head, body)) = &mut cur {
                if tr.trim() == "}" {
                    blocks_vec.push(((*head).to_string(), body.clone()));
                    cur = None;
                } else {
                    body.push(l);
                }
                continue;
            }
            if let Some(head) = tr.strip_suffix(" {") {
                cur = Some((head.trim(), Vec::new()));
            } else if tr.contains(':') && !tr.is_empty() {
                entries += 1;
            }
        }
        for (head, body) in &blocks_vec {
            blocks += 1;
            let key = head.split(':').next().unwrap_or("");
            match key {
                "meta" => {
                    name = val_in(body, "name").unwrap_or("").to_string();
                    meta_type = val_in(body, "type").unwrap_or("").to_string();
                    seq = val_in(body, "seq")
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);
                }
                "headers" => headers += 1,
                "query" | "params" => params += 1,
                "body" => bodies += 1,
                "auth" => auths += 1,
                "script" => scripts += 1,
                "assert" | "tests" => checks += 1,
                "docs" | "vars" | "settings" => misc += 1,
                _ => {}
            }
            if METHODS.contains(&key) {
                method = key.to_string();
                url = val_in(body, "url").unwrap_or("").to_string();
            }
            entries += body
                .iter()
                .filter(|l| l.trim().contains(':') && !l.trim().is_empty())
                .count();
        }
        Some(Self {
            name,
            meta_type,
            seq,
            method,
            url,
            blocks,
            entries,
            headers,
            params,
            bodies,
            auths,
            scripts,
            checks,
            misc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"meta {\n  name: List users\n  type: http\n  seq: 1\n}\n\nget {\n  url: https://api/users\n  body: none\n  auth: none\n}\n\nheaders {\n  accept: application/json\n}\n\nauth:bearer {\n  token: abc\n}\n\nscript:pre-request {\n  bru.setVar('x', 1)\n}\n\ntests {\n  test('ok', () => {})\n}\n";

    #[test]
    fn detects_bru() {
        assert!(detect(FIX));
        assert!(!detect(b"toml = true"));
    }

    #[test]
    fn parses_bru() {
        let br = Bru::parse(FIX).unwrap();
        assert_eq!(br.name, "List users");
        assert_eq!(br.method, "get");
        assert_eq!(br.url, "https://api/users");
        assert_eq!(br.blocks, 6);
        assert_eq!(br.headers, 1);
        assert_eq!(br.auths, 1);
        assert_eq!(br.scripts, 1);
        assert_eq!(br.checks, 1);
        assert!(Bru::parse(b"").is_none());
    }
}
