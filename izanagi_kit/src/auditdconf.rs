//! auditd デーモン設定ファイル (`auditd.conf`) の解析。
//!
//! `key = value` 形式で `log_file`/`num_logs`/`flush`/`space_left_action`
//! 等の既知キーを持つ設定を検出し、エントリ数・既知キー数・アクション値
//! 付きキー数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::auditdconf;
//!
//! let text = br#"log_file = /var/log/audit/audit.log
//! flush = incremental_async
//! num_logs = 5
//! space_left_action = SYSLOG
//! "#;
//!
//! assert!(auditdconf::detect(text));
//! let c = auditdconf::parse(text).unwrap();
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.known_keys, 4);
//! assert_eq!(c.action_keys, 1);
//! ```

/// auditd.conf の既知キー。
const KNOWN_KEYS: &[&str] = &[
    "log_file",
    "log_format",
    "log_group",
    "priority_boost",
    "flush",
    "freq",
    "num_logs",
    "disp_qos",
    "dispatcher",
    "name_format",
    "name",
    "max_log_file",
    "max_log_file_action",
    "action_mail_acct",
    "space_left",
    "space_left_action",
    "verify_email",
    "admin_space_left",
    "admin_space_left_action",
    "disk_full_action",
    "disk_error_action",
    "use_libwrap",
    "wrap_nets",
    "tcp_listen_port",
    "tcp_listen_queue",
    "tcp_max_per_addr",
    "tcp_client_ports",
    "tcp_client_max_idle",
    "transport",
    "enable_krb5",
    "krb5_principal",
    "krb5_key_file",
    "distribute_network",
    "q_depth",
    "overflow_action",
    "overflow_triggered",
    "max_restarts",
    "plugin_dir",
    "end_of_event_timeout",
    "cpu_load_threshold",
    "startup",
    "ebq_backlog",
    "write_logs",
    "space_left_threshold",
    "disk_space_left",
    "disk_full_threshold",
    "audisp_remote_port",
    "network_failure_action",
];

/// `*_action` / `action_*` 系のアクション指定キー。
fn is_action_key(key: &str) -> bool {
    key.ends_with("_action") || key == "action_mail_acct"
}

/// auditd.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` エントリ数。
    pub entries: usize,
    /// 既知キーを持つエントリ数。
    pub known_keys: usize,
    /// `*_action`/`action_mail_acct` 系アクションキー数。
    pub action_keys: usize,
    /// 数値値を持つエントリ数。
    pub numeric_values: usize,
}

/// `b` が auditd.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.known_keys >= 3 || (c.entries >= 4 && c.known_keys >= 2 && c.action_keys >= 1)
}

/// auditd.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        entries: 0,
        known_keys: 0,
        action_keys: 0,
        numeric_values: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim().to_ascii_lowercase();
        let val = line[eq + 1..].trim();
        if key.is_empty() || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            continue;
        }
        counts.entries += 1;
        saw_any = true;
        if KNOWN_KEYS.contains(&key.as_str()) {
            counts.known_keys += 1;
        }
        if is_action_key(&key) {
            counts.action_keys += 1;
        }
        if !val.is_empty() && val.bytes().all(|c| c.is_ascii_digit()) {
            counts.numeric_values += 1;
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"# auditd.conf
log_file = /var/log/audit/audit.log
log_format = ENRICHED
priority_boost = 4
flush = incremental_async
freq = 50
num_logs = 5
name = myhost
max_log_file = 8
max_log_file_action = ROTATE
space_left_action = SYSLOG
admin_space_left_action = HALT
disk_full_action = SUSPEND
tcp_listen_queue = 5
use_libwrap = yes
"#;

    #[test]
    fn detects_auditd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 14);
        assert_eq!(c.known_keys, 14);
        assert_eq!(c.action_keys, 4);
        assert_eq!(c.numeric_values, 5);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"x = 1\ny = 2\n"));
    }
}
