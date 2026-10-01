//! ldirectord (Linux Virtual Server 監視デーモン) `ldirectord.cf` の解析。
//!
//! `virtual=<ip>:<port>`/`virtual=N` セクションとその配下の
//! `real=<ip>:<port> <gate|masq|ipip> <weight>`/`fallback=`/`service=`/
//! `scheduler=`/`protocol=`/`checktype=`/`checkcommand=`/`checkport=`/
//! `request=`/`receive=`/`httpmethod=`/`login=`/`passwd=`/`database=`/
//! `virtualhost=`/`persistent=`/`netmask=`/`monitoraddr=`/`quiescent=`/
//! `emailalert=`/`failurecount=`/`negotiatetimeout=`/`connecttimeout=`/
//! `timeout=`/`connecttimeout=`/`checkinterval=`/`autoreload=`/`fork=`/
//! `cleanstart=`/`supervised=`/`maintenancedir=`/`logfile=`/`emailalertfreq=`/
//! `emailfrom=`/`smtp=`/`callback=`/`execute=`/`af=`/`binaryname=`/
//! `checkversion=`/`rss=`/`command=`/`connectstring=`/`hostname=`/
//! `ping_probe=`/`radius_probe=`/`sip_probe=`/`sslengine=`/`dns_probe=`/
//! `ldap_probe=`/`mysql_probe=`/`pgsql_probe=`/`pop_probe=`/`smtp_probe=`/
//! `submit_probe=`/`nntp_probe=`/`ftp_probe=`/`imap_probe=`/`ssync=`/
//! `socket=`/`app_probe=`/`http_proxy=`/`fwmark=` 等を検出し、
//! グローバル/仮想サーバ/リアルサーバ設定の数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::ldirectord;
//!
//! let text = b"virtual=192.168.1.100:80\n    real=192.168.1.11:80 gate 1\n    service=http\n";
//!
//! assert!(ldirectord::detect(text));
//! let c = ldirectord::parse(text).unwrap();
//! assert_eq!(c.virtuals, 1);
//! ```

/// ldirectord.cf の既知キー (`key=value` の左辺)。
const KNOWN_KEYS: &[&str] = &[
    "virtual",
    "real",
    "fallback",
    "service",
    "scheduler",
    "protocol",
    "checktype",
    "checkcommand",
    "checkport",
    "checktimeout",
    "negotiatetimeout",
    "connecttimeout",
    "timeout",
    "checkinterval",
    "autoreload",
    "quiescent",
    "logfile",
    "fallbackcommand",
    "failurecount",
    "fork",
    "cleanstart",
    "supervised",
    "maintenancedir",
    "request",
    "receive",
    "httpmethod",
    "login",
    "passwd",
    "password",
    "database",
    "virtualhost",
    "persistent",
    "netmask",
    "monitoraddr",
    "emailalert",
    "emailalertfreq",
    "emailfrom",
    "smtp",
    "emailfromname",
    "callback",
    "cleanstop",
    "execute",
    "af",
    "binaryname",
    "checkversion",
    "rss",
    "command",
    "connectstring",
    "hostname",
    "ping_probe",
    "radius_probe",
    "sip_probe",
    "sslengine",
    "dns_probe",
    "ldap_probe",
    "mysql_probe",
    "pgsql_probe",
    "pop_probe",
    "smtp_probe",
    "submit_probe",
    "nntp_probe",
    "ftp_probe",
    "imap_probe",
    "ssync",
    "socket",
    "app_probe",
    "http_proxy",
    "fwmark",
    "num_real",
    "num_fallback",
    "runs",
    "checktype_override",
    "proxyarp",
    "transparent_proxy",
    "persistent_granularity",
    "labeled",
];

/// ldirectord.cf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 全 `key=value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known: usize,
    /// `virtual=` 宣言数。
    pub virtuals: usize,
    /// `real=… gate|masq|ipip` リアルサーバ宣言数。
    pub reals: usize,
    /// `fallback=` 宣言数。
    pub fallbacks: usize,
    /// `service=`/`scheduler=`/`protocol=` コア設定数。
    pub core_settings: usize,
    /// `check*=`/`request=`/`receive=`/`httpmethod=`/`*timeout=`/`checkinterval=` 監視設定数。
    pub check_settings: usize,
    /// `emailalert*`/`smtp`/`callback`/`execute` 通知・アクション設定数。
    pub notify_settings: usize,
}

/// `b` が ldirectord.cf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.virtuals >= 1 && c.reals >= 1 && c.known >= 3
}

/// ldirectord.cf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        entries: 0,
        known: 0,
        virtuals: 0,
        reals: 0,
        fallbacks: 0,
        core_settings: 0,
        check_settings: 0,
        notify_settings: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        counts.entries += 1;
        if !KNOWN_KEYS.contains(&key.as_str()) {
            continue;
        }
        counts.known += 1;
        saw_any = true;
        match key.as_str() {
            "virtual" => counts.virtuals += 1,
            "real" => counts.reals += 1,
            "fallback" | "fallbackcommand" => counts.fallbacks += 1,
            "service" | "scheduler" | "protocol" => {
                counts.core_settings += 1;
            }
            "checktype" | "checkcommand" | "checkport" | "checktimeout" | "negotiatetimeout"
            | "connecttimeout" | "timeout" | "checkinterval" | "request" | "receive"
            | "httpmethod" | "failurecount" | "quiescent" => {
                counts.check_settings += 1;
            }
            "emailalert" | "emailalertfreq" | "emailfrom" | "emailfromname" | "smtp"
            | "callback" | "execute" => {
                counts.notify_settings += 1;
            }
            _ => {}
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

    const SAMPLE: &[u8] = br#"# ldirectord.cf
checktimeout=3
checkinterval=5
autoreload=yes
logfile=/var/log/ldirectord.log
quiescent=no

virtual=192.168.1.100:80
    real=192.168.1.11:80 gate 10
    real=192.168.1.12:80 gate 10
    fallback=127.0.0.1:80 gate
    service=http
    scheduler=wlc
    protocol=tcp
    checktype=negotiate
    checkport=80
    request="index.html"
    receive="Test Page"
    httpmethod=GET
    persistent=600
    netmask=255.255.255.0

virtual=192.168.1.100:443
    real=192.168.1.11:443 gate
    service=https
    scheduler=rr
    protocol=tcp
    checktype=connect
    emailalert=admin@example.com
"#;

    #[test]
    fn detects_ldirectord() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.virtuals, 2);
        assert_eq!(c.reals, 3);
        assert_eq!(c.fallbacks, 1);
        assert_eq!(c.core_settings, 6);
        assert_eq!(c.notify_settings, 1);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"[section]\nkey = value"));
    }
}
