//! Parser for Elastic APM Server configuration (`apm-server.yml`).
//!
//! Counts `key: value` entries and classifies the well-known blocks:
//! `apm-server:` (host/secret_token/rum/kibana/agentconfig/tls/instrumentation),
//! `output.elasticsearch:` / `output.kafka:` / `output.logstash:` /
//! `output.redis:` / `output.console:` / `output.file:`, `queue:`,
//! `logging:` / `logging.metrics:`, `instrumentation:`, `setup:`,
//! `monitoring:`, `max_procs`, `pprof`, plus `- item` list entries
//! (`hosts:`, `protocols:`, `paths:`, `index_pattern` rolls) and `#` comments.
//!
//! ```
//! let b = b"apm-server:\n  host: localhost:8200\noutput.elasticsearch:\n  hosts: [e:9200]\n";
//! assert!(izanagi_kit::apmserver::detect(b));
//! let c = izanagi_kit::apmserver::ApmServer::parse(b).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// Parsed apm-server conf summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApmServer {
    /// Total `key:`/`key: value` entries.
    pub keys: usize,
    /// Top-level named blocks (`apm-server:`/`output.*:`/`queue:`/`logging:`/`instrumentation:`/`setup:`/`monitoring:`/`pprof:`/`max_procs:` — any depth-0 `key:` block).
    pub sections: usize,
    /// `output.*` section names seen (elasticsearch/kafka/logstash/redis/console/file).
    pub outputs: usize,
    /// `enabled:`/`enabled`-suffixed toggle keys.
    pub toggles: usize,
    /// `- item` list entries.
    pub list_items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const OUTPUTS: &[&str] = &[
    "elasticsearch",
    "kafka",
    "logstash",
    "redis",
    "console",
    "file",
    "cloud",
];

const TOGGLE_KEYS: &[&str] = &[
    "enabled",
    "ssl.enabled",
    "rum.enabled",
    "instrumentation.enabled",
    "monitoring.enabled",
];

fn key_of(s: &str) -> &str {
    match s.find(':') {
        Some(c) => s[..c]
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-'))
            .next()
            .unwrap_or(""),
        None => "",
    }
}

/// Detects apm-server yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("apm-server:") || s.starts_with("apm-server.") {
            hits += 2;
        }
        if s.starts_with("output.elasticsearch")
            || s.starts_with("secret_token")
            || s.contains(":8200")
        {
            hits += 1;
        }
    }
    hits >= 2
}

impl ApmServer {
    /// Parse apm-server yaml; `None` when no `key:` entries found.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let Ok(t) = core::str::from_utf8(b) else {
            return None;
        };
        let mut c = Self {
            keys: 0,
            sections: 0,
            outputs: 0,
            toggles: 0,
            list_items: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") || s == "-" {
                c.list_items += 1;
                continue;
            }
            let key = key_of(s);
            if key.is_empty() {
                continue;
            }
            c.keys += 1;
            if l.trim_end().ends_with(':') {
                c.sections += 1;
            }
            if let Some(o) = key.strip_prefix("output.") {
                if OUTPUTS.contains(&o) {
                    c.outputs += 1;
                }
            }
            if key == "enabled" || TOGGLE_KEYS.contains(&key) {
                c.toggles += 1;
            }
        }
        (c.keys > 0).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_apm() {
        assert!(detect(b"apm-server:\n  host: localhost:8200\n"));
        assert!(!detect(b"server:\n  port: 80\n"));
    }

    #[test]
    fn counts_outputs() {
        let c = ApmServer::parse(
            b"apm-server:\n  host: h\noutput.elasticsearch:\n  hosts:\n    - e1\n    - e2\noutput.kafka:\n  enabled: false\nlogging:\n  level: info\n",
        )
        .unwrap();
        assert_eq!(c.outputs, 2);
        assert_eq!(c.sections, 5);
        assert_eq!(c.list_items, 2);
        assert_eq!(c.toggles, 1);
    }

    #[test]
    fn rejects_empty() {
        assert!(ApmServer::parse(b"x\n").is_none());
    }
}
