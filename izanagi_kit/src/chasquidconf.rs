//! Chasquid SMTP サーバ `chasquid.conf` (textproto 風 `key: "value"`) の検出と構造カウント。
//!
//! `hostname:`/`max_data_size_mb:`/`smtp_address:`/`submission_address:`/
//! `monitoring_address:`/`mail_delivery_agent_*`/`data_dir:`/`suffix_separators:`/
//! `drop_characters:`/`mail_log_path:`/`dovecot_*`/`haproxy_incoming` 等の
//! 既知キーを `key: "value"` 形式で識別する。
//!
//! ```
//! let c = izanagi_kit::chasquidconf::parse(
//!     b"hostname: \"mx.example.com\"\nsmtp_address: \":25\"\ndata_dir: \"/var/lib/chasquid\"\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::chasquidconf::detect(b"hostname: \"mx\"\nmax_data_size_mb: 50\n"));
//! ```

/// 既知キー(chasquid.conf 5)。
const KEYS: &[&str] = &[
    "certificate_request_evaluate",
    "data_dir",
    "dovecot_auth",
    "dovecot_client_enabled",
    "dovecot_path",
    "drop_characters",
    "haproxy_incoming",
    "hostname",
    "mail_delivery_agent_args",
    "mail_delivery_agent_binary",
    "mail_log_path",
    "max_data_size_mb",
    "monitoring_address",
    "smtp_address",
    "smtp_ssl_cert_file",
    "smtp_ssl_key_file",
    "submission_address",
    "submission_over_tls_address",
    "suffix_separators",
];

/// Chasquid 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key: "value"` 既知代入。
    pub options: usize,
    /// `#`/`//` コメント行。
    pub comments: usize,
    /// `key:` で始まる未知代入。
    pub unknown: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行が `key: value` パターンか(キー名を返す)。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        None
    } else {
        Some(k)
    }
}

/// b が chasquid.conf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| kv_key(l.trim()).is_some_and(|k| KEYS.contains(&k)))
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        comments: 0,
        unknown: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if let Some(k) = kv_key(t) {
            if KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.unknown += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# chasquid\nhostname: \"mx.example.com\"\nmax_data_size_mb: 50\nsmtp_address: \":25\"\nsubmission_address: \":587\"\nsubmission_over_tls_address: \":465\"\nmonitoring_address: \":1099\"\ndata_dir: \"/var/lib/chasquid\"\nmail_log_path: \"<stdout>\"\n";

    #[test]
    fn chasquidconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 8);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_chasquid() {
        assert!(!detect(b"key: value\n"));
        assert!(!detect(b"hello\n"));
    }
}
