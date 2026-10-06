//! Winlogbeat `winlogbeat.yml` config census.
//!
//! Winlogbeat top-level keys: `winlogbeat.event_logs`,
//! `winlogbeat.registry_file`, `winlogbeat.shutdown_timeout`,
//! `winlogbeat` section, `output.*`, `processors`,
//! `setup`, `logging`, `queue`, `path`, `monitoring`.
//!
//! ```rust
//! let k = b"winlogbeat.event_logs:\n- name: Application\n  ignore_older: 72h\noutput.elasticsearch:\n  hosts: ['es:9200']\n";
//! assert!(izanagi_kit::winlogbeat::detect(k));
//! ```

/// Winlogbeat config census.
#[derive(Debug, Clone)]
pub struct Winlogbeat {
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
    "winlogbeat.event_logs",
    "winlogbeat.registry_file",
    "winlogbeat.shutdown_timeout",
    "winlogbeat",
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

/// Detect a Winlogbeat config.
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

impl Winlogbeat {
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
        let b = b"winlogbeat.event_logs:\n- name: Application\n  ignore_older: 72h\noutput.elasticsearch:\n  hosts: ['es:9200']\n";
        assert!(detect(b));
        let c = Winlogbeat::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"output.logstash:\n  hosts: ['ls']\n"));
        assert!(!detect(b"# winlogbeat.event_logs:\n- x\n"));
    }
}
