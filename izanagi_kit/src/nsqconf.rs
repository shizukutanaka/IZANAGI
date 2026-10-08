//! NSQ `nsqd.cfg`/`nsqlookupd.cfg`(TOML 風 `key = value`)の検出と構造カウント。
//!
//! `data-path`/`mem-queue-size`/`max-bytes-per-file`/`sync-every`/
//! `broadcast-address`/`lookupd-tcp-address`/`tls-*`/`statsd-*`/
//! `e2e-processing-latency-*`/`auth-http-address` 等の既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::nsqconf::parse(
//!     b"data-path = \"/tmp/nsq\"\nmem-queue-size = 10000\nlookupd-tcp-address = \"127.0.0.1:4160\"\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::nsqconf::detect(b"data-path = \"/x\"\nmem-queue-size = 1\nbroadcast-address = \"h\"\n"));
//! ```

/// 既知キー。
const KEYS: &[&str] = &[
    "auth-http-address",
    "auth-remote-addresses",
    "broadcast-address",
    "broadcast-http-port",
    "broadcast-tcp-port",
    "data-path",
    "deflate-level",
    "e2e-processing-latency-percentiles",
    "e2e-processing-latency-window-time",
    "gzip-level",
    "http-address",
    "http-client-connect-timeout",
    "http-client-request-timeout",
    "https-address",
    "https-client-auth-cacert",
    "https-client-auth-cert",
    "https-client-auth-key",
    "https-client-auth-policy",
    "inactive-channel-timeout",
    "lookupd-tcp-address",
    "log-level",
    "log-prefix",
    "max-body-size",
    "max-bytes-per-file",
    "max-deflate-level",
    "max-heartbeat-interval",
    "max-msg-size",
    "max-msg-timeout",
    "max-output-buffer-size",
    "max-output-buffer-timeout",
    "max-rdy-count",
    "max-req-timeout",
    "mem-queue-size",
    "min-output-buffer-timeout",
    "msg-timeout",
    "snappy-enabled",
    "statsd-address",
    "statsd-interval",
    "statsd-mem-stats",
    "statsd-prefix",
    "sync-every",
    "sync-timeout",
    "tcp-address",
    "tls-address",
    "tls-cert",
    "tls-client-auth-cacert",
    "tls-client-auth-cert",
    "tls-client-auth-key",
    "tls-client-auth-policy",
    "tls-key",
    "tls-min-version",
    "tls-required",
    "tls-root-ca-file",
    "verbose",
];

/// nsqconf 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知プロパティ行。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 分類不能行(未知キー・その他)。
    pub misc: usize,
}

/// `key =` 前のキー名。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find('=')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が nsqd.cfg 系かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with('#')
                && !t.starts_with(';')
                && kv_key(t).is_some_and(|k| KEYS.contains(&k))
        })
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if kv_key(t).is_some_and(|k| KEYS.contains(&k)) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# nsqd\ndata-path = \"/var/lib/nsqd\"\nmem-queue-size = 10000\nmax-bytes-per-file = 104857600\nsync-every = 2500\nbroadcast-address = \"node1\"\ntcp-address = \"0.0.0.0:4150\"\nhttp-address = \"0.0.0.0:4151\"\nlookupd-tcp-address = [\"127.0.0.1:4160\"]\nstatsd-address = \"127.0.0.1:8125\"\nstatsd-prefix = \"nsq.%s\"\n";

    #[test]
    fn nsqconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 10);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_nsq() {
        assert!(!detect(b"key=value\nother=thing\n"));
        assert!(!detect(b"data-path = \"/x\"\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
