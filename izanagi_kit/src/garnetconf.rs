//! Garnet `.conf`(`key value` / `key=value`)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::garnetconf::parse(b"bind 127.0.0.1\nport 6379\ncheckpointdir /data\nrecover yes\n").unwrap();
//! assert_eq!(c.options, 4);
//! assert_eq!(c.storage, 2);
//! ```

/// 接続・ネットワーク系キー。
const NET: &[&str] = &[
    "bind",
    "port",
    "admin-port",
    "latency-monitor",
    "metrics-sampling",
    "network-buffer-size",
    "lua-network-buffer-size-limit",
    "max-index-size",
    "tls",
    "cert-file-name",
    "cert-password",
    "server-certificate-subject-name",
    "issuer-certificate-filename",
    "issuer-certificate-password",
    "client-certificate-filename",
    "client-certificate-password",
    "acl-file",
    "password",
    "auth-token",
    "auth-ticket",
    "acl",
    "cluster-announce-endpoint",
    "cluster-announce-port",
    "cluster-tls",
    "cluster-id",
    "port-no",
    "network-simulation-tick",
    "mute-tick",
];
/// ストレージ・永続化系キー。
const STORAGE: &[&str] = &[
    "checkpointdir",
    "recover",
    "snapshot",
    "aof-size",
    "aof-max-size",
    "aof-sparse-size",
    "mutation-logs",
    "mutation-logs-benchmark",
    "enable-storage-command",
    "aof-memory-size",
    "log-dir",
    "page-size",
    "page-size-bits",
    "segment-size",
    "segment-size-bits",
    "index-size",
    "index-size-bits",
    "mutable-percent",
    "mutable-percent-cache",
    "reviv-bin-record-count",
    "reviv-in-chain",
    "reviv-fraction",
    "revivable-bin-count",
    "checkpoint-seconds",
    "checkpoint-throttle",
    "checkpoint-memory-limit",
    "checkpoint-max-write-size",
    "object-store-checkpointdir",
    "object-store-bucket",
    "object-store-endpoint",
    "object-store-access-key",
    "object-store-secret-key",
    "device-storage",
    "heap-size",
    "heap-size-bits",
    "memory",
    "memory-size",
];
/// 実行・管理系キー。
const RUN: &[&str] = &[
    "lua",
    "lua-transaction-mode",
    "lua-script-memory-limit",
    "lua-snippets",
    "lua-object-memory-limit",
    "lua-object-volatile-max-limit",
    "read-command-input",
    "disabled-command",
    "disable-update-via-lookup",
    "quiet-mode",
    "pubsub-page-size",
    "pubsub-page-size-bits",
    "subscriber-refresh-frequency-ms",
    "slot-runtime-maximum-bytes",
    "enable-objstore",
    "enable-debug-command",
    "enable-aof",
    "use-storage-for-indexes",
    "tls-port",
    "bind-to-localhost-only",
    "interactive-mode",
    "password-file",
    "lru-max-size",
    "expired-key-delete",
    "session-expiry-seconds",
    "collect-latency-histograms",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key value` オプション行の総数。
    pub options: usize,
    /// 接続・TLS・認証系オプション数。
    pub net: usize,
    /// ストレージ・永続化系オプション数。
    pub storage: usize,
    /// 実行・管理系オプション数。
    pub runtime: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Garnet 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.options >= 2 && c.misc == 0)
}

/// `b` を Garnet 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        net: 0,
        storage: 0,
        runtime: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let key = t.split([' ', '\t', '=']).next().unwrap_or("");
        if key.is_empty() || !key.chars().next().is_some_and(|x| x.is_ascii_alphabetic()) {
            c.misc += 1;
            continue;
        }
        if t.split_whitespace().nth(1).is_none() && !t.contains('=') {
            c.misc += 1;
            continue;
        }
        c.options += 1;
        if NET.contains(&key) {
            c.net += 1;
        } else if STORAGE.contains(&key) {
            c.storage += 1;
        } else if RUN.contains(&key) {
            c.runtime += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garnet() {
        let cfg = b"bind 127.0.0.1\nport 6379\ncheckpointdir /data\nrecover yes\naof-size 1g\nlua-transaction-mode yes\nenable-storage-command yes\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.options, 7);
        assert_eq!(c.net, 2);
        assert_eq!(c.storage, 4);
        assert_eq!(c.runtime, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_garnet() {
        assert!(!detect(b"random line\n"));
    }
}
