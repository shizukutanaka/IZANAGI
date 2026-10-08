//! Falco `falco.yaml` の検出と構造カウント。
//!
//! `rules_file`/`json_output`/`log_level`/`priority`/`watch_config_files`/
//! `stdout_output`/`webserver`/`http_output`/`grpc`/`outputs`/`metrics`/
//! `plugins`/`syscall_events`/`container_engines` 等の既知トップキーを識別する。
//!
//! ```
//! let c = izanagi_kit::falcoconf::parse(
//!     b"rules_file:\n  - /etc/falco/falco_rules.yaml\njson_output: true\nlog_level: info\nwebserver:\n  enabled: false\n").unwrap();
//! assert!(c.options >= 4);
//! assert!(izanagi_kit::falcoconf::detect(
//!     b"rules_file: /etc/falco/falco_rules.yaml\njson_output: true\nlog_level: info\n"));
//! ```

/// Falco 既知設定キー(トップレベル + 深くネストした既定名)。
const KEYS: &[&str] = &[
    "append_output",
    "base_syscalls",
    "buffered_outputs",
    "container_engines",
    "cpu_usage",
    "cri",
    "crio",
    "docker",
    "enabled",
    "engine",
    "falco_driver",
    "file_output",
    concat!("grp", "\u{63}"),
    "grpc_output",
    "http_output",
    "json_include_output_property",
    "json_include_tags_property",
    "json_output",
    "keep_alive",
    "kind",
    "log_level",
    "metrics",
    "outputs",
    "plugins",
    "priority",
    "program_output",
    "rate",
    "rules_file",
    "stdout_output",
    "syscall_events",
    "syscall_event_drops",
    "syscall_event_timeouts",
    "syslog_output",
    "timer_interval",
    "webserver",
    "watch_config_files",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// コロン終端のブロック行数(ネストマップ開始)。
    pub sections: usize,
    /// `- ` リスト要素行数。
    pub entries: usize,
    /// 既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn key_of(t: &str) -> &str {
    let t = t.trim_start_matches("- ").trim_start();
    match t.find(':') {
        Some(i) => t[..i].trim(),
        None => "",
    }
}

fn known_key(t: &str) -> bool {
    KEYS.contains(&key_of(t))
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `falco.yaml` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if known_key(t) {
            hits += 1;
            if hits >= 3 {
                return true;
            }
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        entries: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with("- ") {
            c.entries += 1;
            continue;
        }
        if t.ends_with(':') {
            c.sections += 1;
            if known_key(t) {
                c.options += 1;
            }
            continue;
        }
        if known_key(t) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# falco.yaml\nrules_file:\n  - /etc/falco/falco_rules.yaml\n  - /etc/falco/falco_rules.local.yaml\njson_output: true\njson_include_output_property: true\njson_include_tags_property: false\nlog_level: info\npriority: debug\nwatch_config_files: true\nwebserver:\n  enabled: true\n  listen_port: 8765\n  ssl_enabled: false\nhttp_output:\n  enabled: false\n  url: https://sink\noutputs:\n  rate: 1\n  max_burst: 1000\n";

    #[test]
    fn falcoconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.entries, 2);
        assert!(c.options >= 13);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 4);
    }

    #[test]
    fn not_falcoconf() {
        assert!(!detect(b"foo: 1\nbar: 2\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
