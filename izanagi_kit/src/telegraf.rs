//! Parser for Telegraf configuration files (`telegraf.conf`, TOML).
//!
//! Counts `[agent]` keys, `[[inputs.*]]`/`[[outputs.*]]`/`[[processors.*]]`/
//! `[[aggregators.*]]`/`[[global_tags]]` array-of-tables headers, plugin
//! options, `interval`/`flush_interval` settings, and comments.
//!
//! ```
//! let b = b"[agent]\n  interval = \"10s\"\n[[inputs.cpu]]\n  percpu = true\n[[outputs.influxdb]]\n  urls = [\"u\"]\n";
//! assert!(izanagi_kit::telegraf::detect(b));
//! let c = izanagi_kit::telegraf::Telegraf::parse(b).unwrap();
//! assert_eq!(c.inputs, 1);
//! assert_eq!(c.outputs, 1);
//! ```

/// Parsed Telegraf configuration summary.
#[derive(Debug, Clone)]
pub struct Telegraf {
    /// `[[inputs.X]]` plugin sections.
    pub inputs: usize,
    /// `[[outputs.X]]` plugin sections.
    pub outputs: usize,
    /// `[[processors.X]]` plugin sections.
    pub processors: usize,
    /// `[[aggregators.X]]` plugin sections.
    pub aggregators: usize,
    /// `[[inputs.X.Y]]` sub-table sections (inner sub-config).
    pub subtables: usize,
    /// `[[global_tags]]` table.
    pub global_tags: usize,
    /// `[agent]` keys (`interval`, `flush_interval`, …).
    pub agent_keys: usize,
    /// `interval`/`flush_interval`/`flush_jitter`/`precision`/`collection_jitter` keys anywhere.
    pub timing_keys: usize,
    /// `urls`/`hosts`/`servers`/`address` option keys.
    pub endpoint_keys: usize,
    /// Total `key = value` option lines.
    pub options: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TIMING: &[&str] = &[
    "interval",
    "flush_interval",
    "flush_jitter",
    "precision",
    "collection_jitter",
    "metric_batch_size",
    "metric_buffer_limit",
];

const ENDPOINTS: &[&str] = &[
    "urls",
    "hosts",
    "servers",
    "address",
    "addresses",
    "broker",
    "brokers",
    "endpoint",
];

/// Returns `true` when the bytes look like a Telegraf TOML config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[[inputs.") || t.contains("[[outputs."))
        || (t.contains("[agent]") && (t.contains("inputs.") || t.contains("outputs.")))
}

impl Telegraf {
    /// Parses a Telegraf configuration, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            inputs: 0,
            outputs: 0,
            processors: 0,
            aggregators: 0,
            subtables: 0,
            global_tags: 0,
            agent_keys: 0,
            timing_keys: 0,
            endpoint_keys: 0,
            options: 0,
            comments: 0,
        };
        let mut in_agent = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("[[") {
                in_agent = false;
                let inner = tr.trim_start_matches('[').trim_end_matches(']');
                let mut parts = inner.split('.');
                let fam = parts.next().unwrap_or("");
                match fam {
                    "inputs" => c.inputs += 1,
                    "outputs" => c.outputs += 1,
                    "processors" => c.processors += 1,
                    "aggregators" => c.aggregators += 1,
                    "global_tags" => c.global_tags += 1,
                    _ => {}
                }
                if inner.matches('.').count() > 1 {
                    c.subtables += 1;
                }
                continue;
            }
            if tr.starts_with('[') {
                in_agent = tr.starts_with("[agent]");
                continue;
            }
            if let Some((k, _)) = tr.split_once('=') {
                c.options += 1;
                let key = k.trim();
                if in_agent {
                    c.agent_keys += 1;
                }
                if TIMING.contains(&key) {
                    c.timing_keys += 1;
                }
                if ENDPOINTS.contains(&key) || key.starts_with("url") {
                    c.endpoint_keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# t\n[agent]\n  interval = \"10s\"\n  flush_interval = \"10s\"\n  hostname = \"h\"\n[[global_tags]]\n  dc = \"east\"\n[[inputs.cpu]]\n  percpu = true\n  totalcpu = true\n[[inputs.mem]]\n[[inputs.disk]]\n  mount_points = [\"/\"]\n[[outputs.influxdb]]\n  urls = [\"http://a\"]\n[[processors.printer]]\n";

    #[test]
    fn parses_telegraf() {
        let c = Telegraf::parse(CONF).unwrap();
        assert_eq!(c.inputs, 3);
        assert_eq!(c.outputs, 1);
        assert_eq!(c.processors, 1);
        assert_eq!(c.global_tags, 1);
        assert_eq!(c.agent_keys, 3);
        assert_eq!(c.timing_keys, 2);
        assert_eq!(c.endpoint_keys, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_telegraf() {
        assert!(!detect(b"[package]\nname = \"x\""));
        assert!(Telegraf::parse(b"x").is_none());
    }
}
