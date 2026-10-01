//! Parser for Fluentd configuration files (`fluent.conf`).
//!
//! Counts `<source>`/`</…>` sections, `<match pattern>`/`filter`/`label`/`system`
//! directives, `@type`-style parameters, `**` wildcard match patterns, and
//! comments/includes.
//!
//! ```
//! let b = b"<source>\n  @type forward\n  port 24224\n</source>\n<match **>\n  @type stdout\n</match>\n";
//! assert!(izanagi_kit::fluentd::detect(b));
//! let c = izanagi_kit::fluentd::Fluentd::parse(b).unwrap();
//! assert_eq!(c.sources, 1);
//! assert_eq!(c.matches, 1);
//! assert_eq!(c.types, 2);
//! ```

/// Parsed Fluentd configuration summary.
#[derive(Debug, Clone)]
pub struct Fluentd {
    /// `<source>` sections.
    pub sources: usize,
    /// `<match …>` sections.
    pub matches: usize,
    /// `<filter …>` sections.
    pub filters: usize,
    /// `<system>`/`<label …>` sections.
    pub systems: usize,
    /// `</…>` close tags (0-delta when balanced).
    pub closes: usize,
    /// `@param value` plugin parameters (`@type`, `@id`, `@label`…).
    pub at_params: usize,
    /// `@type` values specifically.
    pub types: usize,
    /// `**` wildcard match patterns.
    pub wildcards: usize,
    /// `@include`/`include` lines.
    pub includes: usize,
    /// `bind`/`port`/`path`-like bare parameters (word + value).
    pub params: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Returns `true` when the bytes look like a Fluentd configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let secs = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            tr.starts_with("<source>")
                || tr.starts_with("<match")
                || tr.starts_with("<filter")
                || tr.starts_with("<system>")
        })
        .count();
    secs > 0 && t.contains("@type")
}

impl Fluentd {
    /// Parses a Fluentd configuration, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            sources: 0,
            matches: 0,
            filters: 0,
            systems: 0,
            closes: 0,
            at_params: 0,
            types: 0,
            wildcards: 0,
            includes: 0,
            params: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("</") {
                c.closes += 1;
                continue;
            }
            if tr.starts_with('<') {
                let body = tr.trim_start_matches('<').trim_end_matches('>');
                let mut it = body.split_whitespace();
                let name = it.next().unwrap_or("");
                match name {
                    "source" => c.sources += 1,
                    "match" => {
                        c.matches += 1;
                        if body.contains("**") {
                            c.wildcards += 1;
                        }
                    }
                    "filter" => c.filters += 1,
                    "system" | "label" | "worker" => c.systems += 1,
                    _ => {}
                }
                continue;
            }
            let mut w = tr.split_whitespace();
            let key = w.next().unwrap_or("");
            if key == "@include" || key == "include" {
                c.includes += 1;
                continue;
            }
            if let Some(p) = key.strip_prefix('@') {
                if !p.is_empty() && p.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
                    c.at_params += 1;
                    if p == "type" {
                        c.types += 1;
                    }
                    continue;
                }
            }
            if !key.is_empty() && w.next().is_some() {
                c.params += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# conf\n<source>\n  @type forward\n  port 24224\n  bind 0.0.0.0\n</source>\n<match app.**>\n  @type file\n  path /var/log/app.log\n</match>\n";

    #[test]
    fn parses_fluentd() {
        let c = Fluentd::parse(CONF).unwrap();
        assert_eq!(c.sources, 1);
        assert_eq!(c.matches, 1);
        assert_eq!(c.closes, 2);
        assert_eq!(c.at_params, 2);
        assert_eq!(c.types, 2);
        assert_eq!(c.wildcards, 1);
        assert_eq!(c.params, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_fluentd() {
        assert!(!detect(b"<xml><a/></xml>"));
        assert!(Fluentd::parse(b"x").is_none());
    }
}
