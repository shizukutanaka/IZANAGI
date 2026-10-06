//! Metricbeat `metricbeat.yml` config census.
//!
//! Metricbeat top-level keys: `metricbeat.modules`,
//! `metricbeat.autodetect`, `metricbeat.config.modules`,
//! `output.elasticsearch`/`output.logstash`/`output.kafka`/
//! `output.console`, `processors`, `setup`, `logging`,
//! `queue`, `path`, `monitoring`, `http`, `fields`.
//!
//! ```rust
//! let k = b"metricbeat.modules:\n- module: system\n  metricsets: [cpu, memory]\noutput.elasticsearch:\n  hosts: ['es:9200']\n";
//! assert!(izanagi_kit::metricbeat::detect(k));
//! ```

/// Metricbeat config census.
#[derive(Debug, Clone)]
pub struct Metricbeat {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "metricbeat.modules",
    "metricbeat.autodetect",
    "metricbeat.config.modules",
    "metricbeat.max_start_delay",
    "metricbeat.shutdown_timeout",
    "metricbeat",
];

const WEAK: &[&str] = &[
    "output.elasticsearch",
    "output.logstash",
    "output.kafka",
    "output.console",
    "output.redis",
    "output.file",
    "processors",
    "setup",
    "logging",
    "queue",
    "path",
    "monitoring",
    "http",
    "fields",
    "seccomp",
    "instrumentation",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect a Metricbeat config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut strong = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            }
        }
    }
    strong >= 1
}

impl Metricbeat {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"metricbeat.modules:\n- module: system\n  metricsets: [cpu, memory]\noutput.elasticsearch:\n  hosts: ['es:9200']\n";
        assert!(detect(b));
        let c = Metricbeat::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"output.elasticsearch:\n  hosts: ['es']\n"));
        assert!(!detect(b"# metricbeat.modules:\n- x\n"));
    }
}
