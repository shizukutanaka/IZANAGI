//! `luigi.cfg` / `client.cfg` 検出モジュール。
//!
//! Luigi の設定は INI 形式で、`[core]`/`[scheduler]`/
//! `[worker]`/`[resources]`/`[hadoop]`/`[task_history]`/
//! `[retcode]`/`[email]`/`[batch_notifier]` セクションが特徴。
//!
//! ```
//! let b = br#"[core]
//! default-scheduler-host = localhost
//! default-scheduler-port = 8082
//! logging_conf_file = /etc/luigi/logging.cfg
//! [scheduler]
//! record_task_history = True
//! state_path = /var/lib/luigi-state/state.pickle
//! [worker]
//! no_install_shutdown_handler = False
//! "#;
//! let c = izanagi_kit::luigi::parse(b);
//! assert!(izanagi_kit::luigi::detect(b));
//! assert_eq!(c.sections, 3);
//! ```

const SECTIONS: &[&str] = &[
    "[batch_notifier]",
    "[core]",
    "[email]",
    "[error]",
    "[execution_summary]",
    "[hdfs]",
    "[hadoop]",
    "[kubernetes]",
    "[mysql]",
    "[resources]",
    "[retcode]",
    "[scheduler]",
    "[scalding]",
    "[spark]",
    "[task_history]",
    "[worker]",
];

const KEYS: &[&str] = &[
    "default-scheduler-host",
    "default-scheduler-port",
    "default-scheduler-url",
    "logging_conf_file",
    "record_task_history",
    "state_path",
    "no_install_shutdown_handler",
    "rpc_connect_attempts",
    "max-reschedules",
    "disable_hard_timeout",
    "disable_window",
    "count_uniques",
    "worker-timeout",
    "remove-mark-success-rate",
    "retry-external-tasks",
    "send_messages",
    "smtp_ssl",
    "smtp_login",
    "smtp_port",
    "type",
];

fn key_present(t: &str) -> bool {
    t.split('=')
        .next()
        .is_some_and(|k| KEYS.contains(&k.trim()))
}

/// `b` が luigi.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if SECTIONS.contains(&tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 1) || keys >= 2 || secs >= 2
}

/// luigi.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct LuigiConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を luigi.cfg として統計する。
pub fn parse(b: &[u8]) -> LuigiConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = LuigiConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if SECTIONS.contains(&tr) {
            c.sections += 1;
        } else if key_present(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"[core]
default-scheduler-host = localhost
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_keys() {
        let b = br#"default-scheduler-host = localhost
default-scheduler-port = 8082
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[core]\n"));
        assert!(!detect(b"[section]\nkey = val\nfoo = bar\n"));
        assert!(!detect(b"default-scheduler-host = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
