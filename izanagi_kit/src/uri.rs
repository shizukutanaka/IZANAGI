//! RFC 3986 URI parsing, percent-decoding, and reference resolution.
//!
//! [`Uri`] splits a reference into `scheme / authority / path / query /
//! fragment` (authority further into `userinfo@host:port`); [`resolve`]
//! merges a reference against a base URI exactly as §5.3/§5.2.2
//! (the `merge` + `remove_dot_segments` algorithms) — the function HTML
//! `href`, redirects, and `xml:base` depend on.
//!
//! All comparisons are byte-wise and allocation-light; malformed input
//! still parses (missing pieces stay `None` / empty) rather than failing.
//!
//! ```
//! use izanagi_kit::uri::Uri;
//!
//! let u = Uri::parse("https://u:p@host.io:8443/a/b?q=1#frag");
//! assert_eq!(u.scheme.as_deref(), Some("https"));
//! assert_eq!(u.host.as_deref(), Some("host.io"));
//! assert_eq!(u.port, Some(8443));
//! assert_eq!(u.path, "/a/b");
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed URI reference (all slices owned — offsets would outlive `&str`).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Uri {
    /// `scheme:` prefix, lower-case-preserved, without the colon.
    pub scheme: Option<String>,
    /// `userinfo@` inside authority (may contain `:` for user:pass).
    pub userinfo: Option<String>,
    /// Host part of authority (no brackets for IPv6 literals kept in host).
    pub host: Option<String>,
    /// Numeric port (`None` when absent or not digits).
    pub port: Option<u16>,
    /// Path component (always present, possibly empty).
    pub path: String,
    /// Query after `?`.
    pub query: Option<String>,
    /// Fragment after `#`.
    pub fragment: Option<String>,
}

fn is_alpha(b: u8) -> bool {
    b.is_ascii_alphabetic()
}
fn is_scheme_tail(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.'
}
fn unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}
/// Characters legal in a path segment (sub-delims + `:@` + pct).
fn pchar_extra(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b':' | b'@'
    )
}

fn hexdig(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

impl Uri {
    /// Parse `s` per RFC 3986 §3/§4. Everything is optional but `path`.
    pub fn parse(s: &str) -> Self {
        let b = s.as_bytes();
        let mut u = Uri::default();
        let mut i = 0usize;

        // scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"
        if !b.is_empty() && is_alpha(b[0]) {
            let mut j = 1;
            while j < b.len() && is_scheme_tail(b[j]) {
                j += 1;
            }
            if j < b.len() && b[j] == b':' && j > 0 {
                u.scheme = Some(String::from(&s[..j]));
                i = j + 1;
            }
        }
        // `//` authority
        if b[i..].starts_with(b"//") {
            i += 2;
            let start = i;
            while i < b.len() && !matches!(b[i], b'/' | b'?' | b'#') {
                i += 1;
            }
            let auth = &b[start..i];
            // userinfo = last '@' (userinfo may contain encoded '@'? no — raw '@' splits)
            let (ui, hostport) = match auth.iter().rposition(|&c| c == b'@') {
                Some(at) => (Some(&auth[..at]), &auth[at + 1..]),
                None => (None, auth),
            };
            u.userinfo = ui.map(|v| String::from_utf8_lossy(v).into_owned());
            // host[:port]; IPv6 literal "[..]" may contain ':'.
            if hostport.first() == Some(&b'[') {
                if let Some(close) = hostport.iter().position(|&c| c == b']') {
                    u.host = Some(String::from_utf8_lossy(&hostport[..=close]).into_owned());
                    if hostport.get(close + 1) == Some(&b':') {
                        u.port = parse_port(&hostport[close + 2..]);
                    }
                } else {
                    u.host = Some(String::from_utf8_lossy(hostport).into_owned());
                }
            } else if let Some(c) = hostport.iter().rposition(|&c| c == b':') {
                u.host = Some(String::from_utf8_lossy(&hostport[..c]).into_owned());
                u.port = parse_port(&hostport[c + 1..]);
            } else {
                u.host = Some(String::from_utf8_lossy(hostport).into_owned());
            }
        }
        // path until ? or #
        let start = i;
        while i < b.len() && !matches!(b[i], b'?' | b'#') {
            i += 1;
        }
        u.path = String::from(&s[start..i]);
        if i < b.len() && b[i] == b'?' {
            i += 1;
            let q = i;
            while i < b.len() && b[i] != b'#' {
                i += 1;
            }
            u.query = Some(String::from(&s[q..i]));
        }
        if i < b.len() && b[i] == b'#' {
            u.fragment = Some(String::from(&s[i + 1..]));
        }
        u
    }

    /// Percent-decode `s`: `%41` → `A`, `+` stays `+` (form encoding is
    /// `application/x-www-form-urlencoded`, not RFC 3986 — decode it yourself).
    /// Invalid `%` sequences pass through literally.
    pub fn pct_decode(s: &str) -> Vec<u8> {
        let b = s.as_bytes();
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' && i + 3 <= b.len() {
                if let (Some(h), Some(l)) = (hexdig(b[i + 1]), hexdig(b[i + 2])) {
                    out.push(h * 16 + l);
                    i += 3;
                    continue;
                }
            }
            out.push(b[i]);
            i += 1;
        }
        out
    }

    /// Percent-decode to a `String` when the result is UTF-8.
    pub fn pct_decode_str(s: &str) -> String {
        String::from_utf8_lossy(&Self::pct_decode(s)).into_owned()
    }

    /// Percent-encode `s` keeping unreserved bytes; every other byte → `%XX`.
    pub fn pct_encode(s: &str) -> String {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let mut out = String::new();
        for &b in s.as_bytes() {
            if unreserved(b) {
                out.push(b as char);
            } else {
                out.push('%');
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 15) as usize] as char);
            }
        }
        out
    }

    /// Is `s` a valid scheme name (no `:`)?
    pub fn is_scheme(s: &str) -> bool {
        let b = s.as_bytes();
        !b.is_empty() && is_alpha(b[0]) && b[1..].iter().all(|&c| is_scheme_tail(c))
    }

    /// Is `b` allowed literally inside a path segment?
    pub fn is_pchar(b: u8) -> bool {
        unreserved(b) || pchar_extra(b) || b == b'%'
    }

    /// `true` when the reference is absolute (has a scheme).
    pub fn is_absolute(&self) -> bool {
        self.scheme.is_some()
    }

    /// Re-serialize (scheme + `//` authority + path + `?query` + `#frag`).
    pub fn to_uri_string(&self) -> String {
        let mut out = String::new();
        if let Some(s) = &self.scheme {
            out.push_str(s);
            out.push(':');
        }
        if self.host.is_some() {
            out.push_str("//");
            if let Some(ui) = &self.userinfo {
                out.push_str(ui);
                out.push('@');
            }
            if let Some(h) = &self.host {
                out.push_str(h);
            }
            if let Some(p) = self.port {
                out.push(':');
                out.push_str(&std::format!("{p}"));
            }
        }
        out.push_str(&self.path);
        if let Some(q) = &self.query {
            out.push('?');
            out.push_str(q);
        }
        if let Some(f) = &self.fragment {
            out.push('#');
            out.push_str(f);
        }
        out
    }
}

fn parse_port(b: &[u8]) -> Option<u16> {
    if b.is_empty() || b.len() > 5 || !b.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut v: u32 = 0;
    for &c in b {
        v = v * 10 + (c - b'0') as u32;
    }
    if v > u16::MAX as u32 {
        return None;
    }
    Some(v as u16)
}

/// RFC 3986 §5.2.3 `merge`: base authority path + relative path.
fn merge(base_path: &str, base_has_authority: bool, rel: &str) -> String {
    if base_has_authority && base_path.is_empty() {
        let mut s = String::from("/");
        s.push_str(rel);
        return s;
    }
    let mut out = match base_path.rfind('/') {
        Some(k) => String::from(&base_path[..=k]),
        None => String::new(),
    };
    out.push_str(rel);
    out
}

/// RFC 3986 §5.2.4 `remove_dot_segments` — resolves `.`/`..` in-place.
pub fn remove_dot_segments(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let absolute = path.starts_with('/');
    let segs: Vec<&str> = path.split('/').collect();
    let trailing_slash = path.ends_with('/');
    let last = segs.last().copied().unwrap_or("");
    let ended_with_dot = last == "." || last == "..";
    for seg in segs {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    let mut r = String::new();
    if absolute {
        r.push('/');
    }
    r.push_str(&out.join("/"));
    if (trailing_slash || ended_with_dot) && !r.is_empty() && !r.ends_with('/') {
        r.push('/');
    }
    r
}

/// RFC 3986 §5.2.2 strict resolution of `rel` against `base`.
/// Returns the resolved absolute URI string.
///
/// ```
/// use izanagi_kit::uri::resolve;
///
/// assert_eq!(
///     resolve("http://a/b/c/d;p?q", "../g"),
///     "http://a/b/g".to_string()
/// );
/// ```
pub fn resolve(base: &str, rel: &str) -> String {
    let b = Uri::parse(base);
    let r = Uri::parse(rel);
    let mut t = Uri::default();
    if r.scheme.is_some() {
        t.scheme = r.scheme;
        t.userinfo = r.userinfo;
        t.host = r.host;
        t.port = r.port;
        t.path = remove_dot_segments(&r.path);
        t.query = r.query;
    } else {
        if r.host.is_some() {
            t.userinfo = r.userinfo;
            t.host = r.host;
            t.port = r.port;
            t.path = remove_dot_segments(&r.path);
            t.query = r.query;
        } else {
            if r.path.is_empty() {
                t.path = b.path.clone();
                t.query = r.query.or(b.query);
            } else {
                if r.path.starts_with('/') {
                    t.path = remove_dot_segments(&r.path);
                } else {
                    t.path = remove_dot_segments(&merge(&b.path, b.host.is_some(), &r.path));
                }
                t.query = r.query;
            }
            t.userinfo = b.userinfo;
            t.host = b.host;
            t.port = b.port;
        }
        t.scheme = b.scheme;
    }
    t.fragment = r.fragment;
    t.to_uri_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_uri() {
        let u = Uri::parse("https://u:p@host.io:8443/a/b?q=1#frag");
        assert_eq!(u.scheme.as_deref(), Some("https"));
        assert_eq!(u.userinfo.as_deref(), Some("u:p"));
        assert_eq!(u.host.as_deref(), Some("host.io"));
        assert_eq!(u.port, Some(8443));
        assert_eq!(u.path, "/a/b");
        assert_eq!(u.query.as_deref(), Some("q=1"));
        assert_eq!(u.fragment.as_deref(), Some("frag"));
    }

    #[test]
    fn parses_minimal_and_relative() {
        let u = Uri::parse("g");
        assert_eq!(u.scheme, None);
        assert_eq!(u.host, None);
        assert_eq!(u.path, "g");
        let u = Uri::parse("//h/x");
        assert_eq!(u.host.as_deref(), Some("h"));
        assert_eq!(u.path, "/x");
        let u = Uri::parse("mailto:a@b");
        assert_eq!(u.scheme.as_deref(), Some("mailto"));
        assert_eq!(u.path, "a@b");
        let u = Uri::parse("a?b#c");
        assert_eq!(u.query.as_deref(), Some("b"));
        assert_eq!(u.fragment.as_deref(), Some("c"));
        assert!(!u.is_absolute());
    }

    #[test]
    fn ipv6_and_bad_port() {
        let u = Uri::parse("http://[::1]:8080/x");
        assert_eq!(u.host.as_deref(), Some("[::1]"));
        assert_eq!(u.port, Some(8080));
        let u = Uri::parse("http://h:99999/x");
        assert_eq!(u.port, None);
        let u = Uri::parse("http://h:/x");
        assert_eq!(u.port, None);
    }

    #[test]
    fn pct_round_trip() {
        assert_eq!(Uri::pct_decode_str("%41%7e%zz%25"), "A~%zz%");
        assert_eq!(Uri::pct_encode("a b/日本"), "a%20b%2F%E6%97%A5%E6%9C%AC");
        assert_eq!(Uri::pct_encode("a-b_c.d~e"), "a-b_c.d~e");
        assert!(Uri::is_scheme("x+y-z.1"));
        assert!(!Uri::is_scheme("1x"));
        assert!(!Uri::is_scheme("x:y"));
        assert!(Uri::is_pchar(b'@'));
        assert!(!Uri::is_pchar(b' '));
    }

    #[test]
    fn rfc3986_52_examples() {
        // The RFC's own resolution table against base "http://a/b/c/d;p?q".
        let b = "http://a/b/c/d;p?q";
        let cases: &[(&str, &str)] = &[
            ("g:h", "g:h"),
            ("g", "http://a/b/c/g"),
            ("./g", "http://a/b/c/g"),
            ("g/", "http://a/b/c/g/"),
            ("/g", "http://a/g"),
            ("//g", "http://g"),
            ("?y", "http://a/b/c/d;p?y"),
            ("g?y", "http://a/b/c/g?y"),
            ("#s", "http://a/b/c/d;p?q#s"),
            ("g#s", "http://a/b/c/g#s"),
            ("g?y#s", "http://a/b/c/g?y#s"),
            (".", "http://a/b/c/"),
            ("./", "http://a/b/c/"),
            ("..", "http://a/b/"),
            ("../", "http://a/b/"),
            ("../g", "http://a/b/g"),
            ("../..", "http://a/"),
            ("../../", "http://a/"),
            ("../../g", "http://a/g"),
            ("../../../g", "http://a/g"),
            ("./g.", "http://a/b/c/g."),
            ("/./g", "http://a/g"),
            ("g/../h", "http://a/b/c/h"),
            ("g;x=1/../y", "http://a/b/c/y"),
        ];
        for (rel, want) in cases {
            assert_eq!(resolve(b, rel), *want, "rel {rel}");
        }
    }

    #[test]
    fn remove_dot_handles_edges() {
        assert_eq!(remove_dot_segments("/a/b/c/./../../g"), "/a/g");
        assert_eq!(remove_dot_segments("mid/content=5/../6"), "mid/6");
        assert_eq!(remove_dot_segments("/../g"), "/g");
        assert_eq!(remove_dot_segments(""), "");
        assert_eq!(remove_dot_segments(".."), "");
        assert_eq!(remove_dot_segments("/.."), "/");
    }

    #[test]
    fn serialize_round_trip() {
        let s = "https://u:p@h.io:9/x/y?q#f";
        let u = Uri::parse(s);
        assert_eq!(u.to_uri_string(), s);
        let u = Uri::parse("http://a/b/c/../d");
        assert_eq!(u.to_uri_string(), "http://a/b/c/../d"); // no auto-normalize
    }

    #[test]
    fn pct_decode_pairs() {
        assert_eq!(Uri::pct_decode("%41%42"), b"AB".to_vec());
        assert_eq!(Uri::pct_decode("%e3%81%82"), vec![0xe3, 0x81, 0x82]);
        assert_eq!(Uri::pct_decode("a+b%20"), b"a+b ".to_vec());
        // Malformed sequences are kept literal rather than panicking.
        assert_eq!(Uri::pct_decode("%zz"), b"%zz".to_vec());
        assert_eq!(Uri::pct_decode("%4"), b"%4".to_vec());
        assert_eq!(Uri::pct_decode_str("%41%42"), "AB");
    }
}
