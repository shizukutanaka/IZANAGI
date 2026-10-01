//! Parser for Filebeat configuration files (`filebeat.yml`).
//!
//! Counts `filebeat.inputs:`/`- type:` entries, `paths:` list items, inputs
//! under `filebeat.config.inputs`, enabled modules, `output.*` backends,
//! `setup.kibana`/`processors:` entries, and prospector-era keys.
//!
//! ```
//! let b = b"filebeat.inputs:\n- type: filestream\n  enabled: true\n  paths:\n    - /var/log/*.log\noutput.elasticsearch:\n  hosts: [\"es\"]\n";
//! assert!(izanagi_kit::filebeat::detect(b));
//! let c = izanagi_kit::filebeat::Filebeat::parse(b).unwrap();
//! assert_eq!(c.inputs, 1);
//! assert_eq!(c.outputs, 1);
//! ```

/// Parsed Filebeat configuration summary.
#[derive(Debug, Clone)]
pub struct Filebeat {
    /// `filebeat.inputs:`/`filebeat.config.inputs` entries (`- type:` items).
    pub inputs: usize,
    /// `type:` values observed on input items (`filestream`/`log`/…).
    pub input_types: usize,
    /// `paths:` list items (`- /path`).
    pub paths: usize,
    /// `enabled:` toggles.
    pub enabled_flags: usize,
    /// `filebeat.modules:` entries.
    pub modules: usize,
    /// `output.X:` top-level backends (elasticsearch/logstash/kafka/redis/console/file).
    pub outputs: usize,
    /// `hosts:` entries under outputs.
    pub hosts: usize,
    /// `setup.kibana` host entries.
    pub kibana: usize,
    /// `processors:` list entries (`- add_fields:`…).
    pub processors: usize,
    /// `logging.*` keys.
    pub logging_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

fn key_of(l: &str) -> (&str, &str) {
    let tr = l.trim_start();
    let i = l.len() - tr.len();
    let _ = i;
    let mut it = tr.splitn(2, ':');
    (it.next().unwrap_or(""), it.next().unwrap_or(""))
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Returns `true` when the bytes look like a Filebeat YAML config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("filebeat.inputs")
        || t.contains("filebeat.config.inputs")
        || t.contains("filebeat.prospectors")
        || (t.contains("filebeat.") && t.contains("output."))
}

impl Filebeat {
    /// Parses a Filebeat configuration, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            inputs: 0,
            input_types: 0,
            paths: 0,
            enabled_flags: 0,
            modules: 0,
            outputs: 0,
            hosts: 0,
            kibana: 0,
            processors: 0,
            logging_keys: 0,
            comments: 0,
        };
        let mut in_inputs = false;
        let mut in_modules = false;
        let mut in_output = false;
        let mut in_kibana = false;
        let mut in_processors = false;
        let mut in_logging = false;
        let mut in_paths = false;
        let mut paths_indent = 0usize;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let i = indent(l);
            if i == 0 && tr.starts_with("- ") && (in_inputs || in_modules || in_processors) {
                if in_inputs {
                    c.inputs += 1;
                    let inner = tr.trim_start_matches('-').trim_start();
                    if inner.split(':').next().unwrap_or("").trim() == "type" {
                        c.input_types += 1;
                    }
                } else if in_modules {
                    c.modules += 1;
                } else {
                    c.processors += 1;
                }
                continue;
            }
            if i == 0 {
                in_inputs = false;
                in_modules = false;
                in_output = false;
                in_kibana = false;
                in_processors = false;
                in_logging = false;
                in_paths = false;
                let (k, _) = key_of(l);
                match k {
                    "filebeat.inputs" | "filebeat.config.inputs" | "filebeat.prospectors" => {
                        in_inputs = true;
                    }
                    "filebeat.modules" => in_modules = true,
                    "setup.kibana" => in_kibana = true,
                    "processors" => in_processors = true,
                    "logging" | "logging.level" | "logging.to_files" | "logging.files" => {
                        in_logging = true;
                        c.logging_keys += 1;
                    }
                    _ => {
                        if let Some(o) = k.strip_prefix("output.") {
                            let known = [
                                "elasticsearch",
                                "logstash",
                                "kafka",
                                "redis",
                                "console",
                                "file",
                                "cloud",
                            ];
                            if known.iter().any(|n| o.starts_with(n)) {
                                c.outputs += 1;
                                in_output = true;
                            }
                        }
                    }
                }
                continue;
            }
            if in_paths {
                if tr.starts_with("- ") && i > paths_indent {
                    c.paths += 1;
                    continue;
                }
                if i <= paths_indent {
                    in_paths = false;
                }
            }
            if in_inputs {
                if tr.starts_with("- ") || tr.starts_with("-type") || tr.starts_with("- type") {
                    c.inputs += 1;
                    let inner = tr.trim_start_matches('-').trim_start();
                    if inner.split(':').next().unwrap_or("").trim() == "type" {
                        c.input_types += 1;
                    }
                    continue;
                }
                let (k, _) = key_of(l);
                if k == "type" {
                    c.input_types += 1;
                } else if k == "paths" {
                    in_paths = true;
                    paths_indent = i;
                } else if k == "enabled" {
                    c.enabled_flags += 1;
                }
                continue;
            }
            if in_modules && (tr.starts_with("- ") || tr.ends_with(':')) {
                c.modules += 1;
                continue;
            }
            if in_kibana {
                let (k, _) = key_of(l);
                if k == "host" || k == "hosts" {
                    c.kibana += 1;
                }
                continue;
            }
            if in_processors && tr.starts_with("- ") {
                c.processors += 1;
                continue;
            }
            if in_output {
                let (k, _) = key_of(l);
                if k == "hosts" || k == "host" {
                    c.hosts += 1;
                }
                if i <= 1 && !tr.starts_with('-') {
                    in_output = false;
                }
            }
            if in_logging {
                c.logging_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"filebeat.inputs:\n- type: filestream\n  enabled: true\n  paths:\n    - /var/log/a.log\n    - /var/log/b.log\n- type: log\n  paths:\n    - /tmp/x.log\nfilebeat.modules:\n- module: nginx\noutput.elasticsearch:\n  hosts: [\"e1\", \"e2\"]\n  username: u\nprocessors:\n  - add_fields:\n      target: t\n# note\n";

    #[test]
    fn parses_filebeat() {
        let c = Filebeat::parse(CONF).unwrap();
        assert_eq!(c.inputs, 2);
        assert_eq!(c.input_types, 2);
        assert_eq!(c.paths, 3);
        assert_eq!(c.enabled_flags, 1);
        assert_eq!(c.modules, 1);
        assert_eq!(c.outputs, 1);
        assert_eq!(c.hosts, 1);
        assert_eq!(c.processors, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_filebeat() {
        assert!(!detect(b"name: x\nversion: 1\n"));
        assert!(Filebeat::parse(b"x").is_none());
    }
}
