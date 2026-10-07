//! `xinetd.conf` 検出モジュール。
//!
//! xinetd の設定は `defaults { ... }` グローバルブロックと
//! `service <name> { ... }` サービスブロックで構成される。
//! ブロック内は `disable`/`socket_type`/`protocol`/`wait`/`user`/
//! `group`/`server`/`server_args`/`port`/`only_from`/`no_access`/
//! `access_times`/`log_type`/`log_on_success`/`log_on_failure`/
//! `instances`/`per_source`/`nice`/`redirect`/`bind`/`banner`/
//! `flags`/`type`/`rpc_version`/`rpc_number`/`env`/`passenv`/
//! `groups`/`enabled`/`include`/`includedir`/`umask`/`cps`/
//! `max_load`/`v6only`/`deny_time`/`mdns`/`libwrap`/`sensors`/`sensor`
//! 等の `key = value` 行(+ `+=`/`-=` 演算子)で構成される。
//!
//! ```
//! let b = b"service telnet\n\
//!           {\n\
//!           disable = no\n\
//!           socket_type = stream\n\
//!           protocol = tcp\n\
//!           wait = no\n\
//!           user = root\n\
//!           server = /usr/sbin/in.telnetd\n\
//!           }\n";
//! let c = izanagi_kit::xinetdconf::parse(b);
//! assert!(izanagi_kit::xinetdconf::detect(b));
//! assert_eq!(c.services, 1);
//! ```

const KEYS: &[&str] = &[
    "access_times",
    "banner",
    "banner_fail",
    "banner_success",
    "bind",
    "cps",
    "deny_time",
    "disable",
    "enabled",
    "env",
    "flags",
    "group",
    "groups",
    "id",
    "include",
    "includedir",
    "instances",
    "interface",
    "libwrap",
    "log_on_failure",
    "log_on_success",
    "log_type",
    "max_load",
    "mdns",
    "nice",
    "no_access",
    "only_from",
    "passenv",
    "per_source",
    "port",
    "protocol",
    "redirect",
    "rlimit_as",
    "rlimit_cpu",
    "rlimit_data",
    "rlimit_rss",
    "rlimit_stack",
    "rpc_number",
    "rpc_version",
    "sensor",
    "server",
    "server_args",
    "service_attribution",
    "socket_type",
    "type",
    "umask",
    "user",
    "v6only",
    "wait",
];

fn is_service_head(t: &str) -> bool {
    t == "defaults"
        || t == "defaults {"
        || (t.starts_with("service ") && t.split_whitespace().count() <= 3)
}

fn is_kv(t: &str) -> bool {
    for op in ["+=", "-=", "="] {
        if let Some((k, _)) = t.split_once(op) {
            return KEYS.contains(&k.trim());
        }
    }
    false
}

/// `b` が xinetd.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut heads = 0usize;
    let mut kvs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_service_head(tr) {
            heads += 1;
        } else if is_kv(tr) {
            kvs += 1;
        }
    }
    heads >= 1 && kvs >= 2
}

/// xinetd.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct XinetdConf {
    /// service/defaults ブロック数。
    pub services: usize,
    /// 既知 `key = value` 行数。
    pub kvs: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を xinetd.conf として統計する。
pub fn parse(b: &[u8]) -> XinetdConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = XinetdConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_service_head(tr) {
            c.services += 1;
        } else if is_kv(tr) {
            c.kvs += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"service tftp\n{\ndisable = yes\nsocket_type = dgram\n}\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.services, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"service x\n{\n}\n"));
        assert!(!detect(b"service tftp\ndisable = yes\n"));
        assert!(!detect(
            b"disable = yes\nsocket_type = stream\nuser = root\n"
        ));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# service x\n# disable = yes\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.services, 0);
    }
}
