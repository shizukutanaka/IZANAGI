//! Netscape `cookies.txt` — tab-separated lines
//! `domain<TAB>flag<TAB>path<TAB>secure<TAB>expires<TAB>name<TAB>value`
//! with `#HttpOnly_` prefixed host lines and `# Netscape HTTP Cookie
//! File` banner comments.
//!
//! ```
//! let d = b"# Netscape HTTP Cookie File\n.example.com\tTRUE\t/\tFALSE\t1893456000\tn\tv\n";
//! let c = izanagi_kit::cookiejar::parse(d).unwrap();
//! assert_eq!(c.entries, 1);
//! assert_eq!(c.banner, true);
//! assert_eq!(c.secure_flags, 0);
//! assert!(izanagi_kit::cookiejar::detect(d));
//! ```

/// Census of a `cookies.txt` jar.
#[derive(Debug, Clone)]
pub struct Cookiejar {
    /// `# Netscape HTTP Cookie File` / Mozilla banner seen.
    pub banner: bool,
    /// `#HttpOnly_` host entries.
    pub http_only: usize,
    /// Data lines with ≥ 6 TABs (7 fields).
    pub entries: usize,
    /// `TRUE` flag in the include-subdomains column.
    pub subdomains: usize,
    /// `TRUE` flag in the secure column.
    pub secure_flags: usize,
    /// Session cookies (expires field `0` or empty).
    pub session: usize,
    /// Comment lines (`#` non-HttpOnly).
    pub comments: usize,
    /// Malformed lines (fewer than 7 fields).
    pub bad_lines: usize,
}

/// Detects a `cookies.txt`: banner or ≥1 7-field tab line.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        !l.starts_with('#')
            && l.matches('\t').count() >= 6
            && l.split('\t')
                .nth(4)
                .is_some_and(|e| e.bytes().all(|c| c.is_ascii_digit()))
    })
}

/// Parses a `cookies.txt`; `None` on non-UTF-8 or no valid lines.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Cookiejar> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut c = Cookiejar {
        banner: t.contains("HTTP Cookie File"),
        http_only: 0,
        entries: 0,
        subdomains: 0,
        secure_flags: 0,
        session: 0,
        comments: 0,
        bad_lines: 0,
    };
    for l in t.lines() {
        if l.is_empty() {
            continue;
        }
        if l.starts_with("#HttpOnly_") {
            c.http_only += 1;
        } else if l.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let fields: Vec<&str> = l.split('\t').collect();
        if fields.len() < 7 {
            if !l.starts_with('#') {
                c.bad_lines += 1;
            }
            continue;
        }
        if fields[3] != "TRUE" && fields[3] != "FALSE" {
            c.bad_lines += 1;
            continue;
        }
        c.entries += 1;
        if fields[1] == "TRUE" {
            c.subdomains += 1;
        }
        if fields[3] == "TRUE" {
            c.secure_flags += 1;
        }
        if fields[4].is_empty() || fields[4] == "0" {
            c.session += 1;
        }
    }
    (c.entries > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"# Netscape HTTP Cookie File\n# comment\n.example.com\tTRUE\t/\tTRUE\t1893456000\tn\tv\nsub.org\tFALSE\t/p\tFALSE\t0\ts\tu\n#HttpOnly_x.io\tTRUE\t/\tFALSE\t1\th\tw\nbad\n";

    #[test]
    fn parses() {
        let c = parse(D).unwrap();
        assert!(c.banner);
        assert_eq!(c.http_only, 1);
        assert_eq!(c.entries, 3);
        assert_eq!(c.subdomains, 2);
        assert_eq!(c.secure_flags, 1);
        assert_eq!(c.session, 1);
        assert_eq!(c.comments, 2);
        assert_eq!(c.bad_lines, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b".a\tTRUE\t/\tFALSE\t1\tn\tv"));
        assert!(!detect(b"# Netscape HTTP Cookie File\nbad"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"a\tb\tc").is_none());
    }
}
