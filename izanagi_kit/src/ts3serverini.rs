//! `ts3server.ini` (TeamSpeak 3 server) 検出モジュール。
//!
//! TS3 サーバ設定は平坦な `key=value` 形式で、`machine_id=`/
//! `default_voice_port=`/`voice_ip=`/`filetransfer_port=`/
//! `filetransfer_ip=`/`query_port=`/`query_ip=`/`query_ip_whitelist=`/
//! `query_ip_blacklist=`/`dbplugin=`/`dbpluginparameter=`/`dbsqlpath=`/
//! `dbsqlcreatepath=`/`dbconnections=`/`dblogkeepdays=`/`logpath=`/
//! `logquerycommands=`/`logappend=`/`serverquerydocs_path=`/
//! `query_skipbruteforcecheck=`/`query_buffer_mb=`/`http_proxy=`/
//! `licensepath=`/`query_protocols=`/`query_ssh_ip=`/`query_ssh_port=`/
//! `query_ssh_rsa_host_key_file=`/`query_timeout=`/
//! `ssh_encryption_min_key_size=`/`query_ssh_max_login_attempts=`/
//! `hint_*`/`virtualserver_metadata_*`/`crashdumps_path=`/
//! `serverquerydocs_path=`/`admin_password*`/`license_accepted=` 等の
//! キーで構成される。
//!
//! ```
//! let b = b"machine_id=\n\
//!           default_voice_port=9987\n\
//!           voice_ip=0.0.0.0\n\
//!           filetransfer_port=30033\n\
//!           query_port=10011\n\
//!           dbplugin=ts3db_sqlite3\n";
//! let c = izanagi_kit::ts3serverini::parse(b);
//! assert!(izanagi_kit::ts3serverini::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "admin_password",
    "admin_password_file",
    "crashdumps_path",
    "dbclientkeepdays",
    "dbconnections",
    "dblogkeepdays",
    "dbplugin",
    "dbpluginparameter",
    "dbsqlcreatepath",
    "dbsqlpath",
    "default_voice_port",
    "filetransfer_ip",
    "filetransfer_port",
    "hints_enabled",
    "http_proxy",
    "license_accepted",
    "licensepath",
    "lipsis_api_key",
    "logappend",
    "logpath",
    "logquerycommands",
    "machine_id",
    "query_buffer_mb",
    "query_ip",
    "query_ip_blacklist",
    "query_ip_whitelist",
    "query_port",
    "query_protocols",
    "query_skipbruteforcecheck",
    "query_ssh_ip",
    "query_ssh_max_login_attempts",
    "query_ssh_port",
    "query_ssh_rsa_host_key_file",
    "query_timeout",
    "serverquerydocs_path",
    "ssh_encryption_min_key_size",
    "voice_ip",
];

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k) || k.starts_with("virtualserver_metadata_") || k.starts_with("hint_")
}

/// `b` が ts3server.ini に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// ts3server.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct Ts3ServerIni {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を ts3server.ini として統計する。
pub fn parse(b: &[u8]) -> Ts3ServerIni {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Ts3ServerIni::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.contains('=') && is_key(tr) {
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
        let b = b"default_voice_port=9987\nquery_port=10011\ndbplugin=x\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"default_voice_port=9987\nquery_port=10011\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# default_voice_port=1\n# query_port=2\n# dbplugin=x\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 3);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
