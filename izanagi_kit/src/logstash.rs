//! Parser for Logstash pipeline configuration files (`logstash.conf`).
//!
//! Counts `input {}`/`filter {}`/`output {}` sections, plugin blocks
//! (`grok {`/`beats {`/`elasticsearch {` …), `=>` option assignments,
//! `if`/`else if`/`else` conditionals, and comments.
//!
//! ```
//! let b = b"input {\n  beats {\n    port => 5044\n  }\n}\noutput {\n  elasticsearch {\n    hosts => [\"es\"]\n  }\n}\n";
//! assert!(izanagi_kit::logstash::detect(b));
//! let c = izanagi_kit::logstash::Logstash::parse(b).unwrap();
//! assert_eq!(c.inputs, 1);
//! assert_eq!(c.outputs, 1);
//! assert_eq!(c.plugins, 2);
//! ```

/// Parsed Logstash configuration summary.
#[derive(Debug, Clone)]
pub struct Logstash {
    /// `input {` sections.
    pub inputs: usize,
    /// `filter {` sections.
    pub filters: usize,
    /// `output {` sections.
    pub outputs: usize,
    /// Named plugin blocks inside sections (`name {`).
    pub plugins: usize,
    /// `option => value` assignments.
    pub assignments: usize,
    /// `if`/`else if`/`else` conditional statements.
    pub conditionals: usize,
    /// `[` array literals opened with `[`.
    pub arrays: usize,
    /// `{`/`}` brace-balance check (0 when balanced).
    pub brace_delta: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Returns `true` when the bytes look like a Logstash pipeline config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let secs = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            tr.starts_with("input {") || tr.starts_with("filter {") || tr.starts_with("output {")
        })
        .count();
    secs > 0 && t.contains("=>")
}

impl Logstash {
    /// Parses a Logstash configuration, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            inputs: 0,
            filters: 0,
            outputs: 0,
            plugins: 0,
            assignments: 0,
            conditionals: 0,
            arrays: 0,
            brace_delta: 0,
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
            if tr.starts_with("input {") {
                c.inputs += 1;
                continue;
            }
            if tr.starts_with("filter {") {
                c.filters += 1;
                continue;
            }
            if tr.starts_with("output {") {
                c.outputs += 1;
                continue;
            }
            if tr.starts_with("if ")
                || tr.starts_with("if(")
                || tr.starts_with("if (")
                || tr.starts_with("else")
            {
                c.conditionals += 1;
            }
            if tr.ends_with('{') || tr.ends_with("{ }") {
                // `plugin {`/`plugin { }` inside a section
                let name = tr.split('{').next().unwrap_or("").trim();
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.')
                {
                    c.plugins += 1;
                }
            }
            if tr.contains("=>") {
                c.assignments += 1;
            }
            c.brace_delta += tr.matches('{').count();
            c.brace_delta = c.brace_delta.saturating_sub(tr.matches('}').count());
            c.arrays += tr
                .matches('[')
                .count()
                .saturating_sub(tr.matches("[...").count());
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"input {\n  beats {\n    port => 5044\n  }\n  tcp {\n    port => 5000\n  }\n}\nfilter {\n  grok {\n    match => { \"message\" => \"%{COMBINEDAPACHELOG}\" }\n  }\n  if [level] == \"debug\" {\n    drop { }\n  }\n}\n# out\noutput {\n  elasticsearch {\n    hosts => [\"a\", \"b\"]\n  }\n}\n";

    #[test]
    fn parses_logstash() {
        let c = Logstash::parse(CONF).unwrap();
        assert_eq!(c.inputs, 1);
        assert_eq!(c.filters, 1);
        assert_eq!(c.outputs, 1);
        assert_eq!(c.plugins, 5);
        assert_eq!(c.assignments, 4);
        assert_eq!(c.conditionals, 1);
        assert_eq!(c.comments, 1);
        assert_eq!(c.brace_delta, 0);
    }

    #[test]
    fn rejects_non_logstash() {
        assert!(!detect(b"server { listen 80; }"));
        assert!(!detect(b"{}"));
        assert!(Logstash::parse(b"plain").is_none());
    }
}
