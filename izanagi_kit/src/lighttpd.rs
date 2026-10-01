//! Census of a lighttpd `lighttpd.conf`.
//!
//! lighttpd config: `var.x = value`/`server.modules = ( … )`/`key = value`
//! assignments, conditional blocks `$HTTP["host"] =~ "…" { … }`/`else {`,
//! array syntax `( "a", "b" )`, `server.*`/`mod_*`/`url.*`/`cgi.*`/
//! `auth.*`/`fastcgi.*`/`proxy.*`/`scgi.*`/`ssl.*`/`mimetype.*` namespaces,
//! `include`/`include_shell`, `#` comments, `+=` append operator.
//!
//! ```rust
//! let c = izanagi_kit::lighttpd::Lighttpd::parse(
//!     b"server.modules = ( \"mod_rewrite\" )\nserver.document-root = \"/srv\"\n",
//! ).unwrap();
//! assert_eq!(c.assignments, 2);
//! ```
#![forbid(unsafe_code)]

/// lighttpd config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lighttpd {
    /// `key = value`/`key += value` assignments.
    pub assignments: usize,
    /// `$VAR["x"] cond {`/`else` conditional blocks.
    pub conditionals: usize,
    /// `include`/`include_shell` lines.
    pub includes: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like lighttpd config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("server.") && t.contains('=')) || t.contains("$HTTP[") || t.contains("mod_")
}

impl Lighttpd {
    /// Parse a lighttpd.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            conditionals: 0,
            includes: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with('$') || l.starts_with("else") || l.starts_with("}") {
                if l.starts_with('$') || l.starts_with("else") {
                    c.conditionals += 1;
                }
            } else if l.starts_with("include") {
                c.includes += 1;
            } else if l.contains('=') {
                c.assignments += 1;
            }
        }
        if c.assignments + c.conditionals == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# lighttpd\n",
            "server.modules = (\n",
            "    \"mod_rewrite\",\n",
            "    \"mod_proxy\",\n",
            ")\n",
            "server.document-root = \"/srv/www\"\n",
            "server.port = 8080\n",
            "mimetype.assign += ( \".html\" => \"text/html\" )\n",
            "$HTTP[\"host\"] =~ \"example\\\\.com\" {\n",
            "    proxy.server = ( \"\" => ((\"host\" => \"127.0.0.1\", \"port\" => 9000)) )\n",
            "}\n",
            "else $HTTP[\"url\"] =~ \"^/static\" {\n",
            "    server.document-root = \"/static\"\n",
            "}\n",
            "include \"mime.conf\"\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Lighttpd::parse(b.as_bytes()).unwrap();
        assert!(c.assignments >= 5);
        assert_eq!(c.conditionals, 2);
        assert_eq!(c.includes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"fn main() {}\n"));
        assert!(Lighttpd::parse(b"# none\n").is_none());
    }
}
