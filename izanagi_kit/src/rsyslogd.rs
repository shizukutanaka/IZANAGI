//! rsyslog 設定ファイル (`rsyslog.conf` / `rsyslog.d/*.conf`) の解析。
//!
//! レガシーセレクタ (`auth,authpriv.* /var/log/…`)、`$` ディレクティブ
//! (`$ModLoad`/`$IncludeConfig`/`$ActionQueueType`)、RAInerscript ブロック
//! (`module(`/`template(`/`input(`/`action(`/`global(`/`main_queue(`/
//! `ruleset(`/`if … then`) を検出して整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::rsyslogd;
//!
//! let text = br#"$ModLoad imuxsock
//! $WorkDirectory /var/spool/rsyslog
//! auth,authpriv.* /var/log/auth.log
//! *.* @@logs.example.com:514
//! "#;
//!
//! assert!(rsyslogd::detect(text));
//! let c = rsyslogd::parse(text).unwrap();
//! assert_eq!(c.directives, 2);
//! assert_eq!(c.selectors, 1);
//! assert_eq!(c.forwarders, 1);
//! ```

/// `$<name>` ディレクティブの既知名。
const KNOWN_DIRECTIVES: &[&str] = &[
    "ModLoad",
    "IncludeConfig",
    "WorkDirectory",
    "ActionQueueType",
    "ActionQueueFileName",
    "ActionQueueMaxDiskSpace",
    "ActionQueueSaveOnShutdown",
    "ActionQueueDequeueBatchSize",
    "ActionQueueHighWaterMark",
    "ActionQueueLowWaterMark",
    "ActionResumeRetryCount",
    "ActionResumeInterval",
    "MainMsgQueueSize",
    "MainMsgQueueType",
    "MainMsgQueueFileName",
    "MainMsgQueueMaxDiskSpace",
    "MaxOpenFiles",
    "LocalHostName",
    "PreserveFQDN",
    "PrivDropToUser",
    "PrivDropToGroup",
    "Umask",
    "MaxMessageSize",
    "RepeatedMsgReduction",
    "DefaultNetstreamDriver",
    "DefaultNetstreamDriverCAFile",
    "DefaultNetstreamDriverCertFile",
    "DefaultNetstreamDriverKeyFile",
    "DirCreateMode",
    "FileCreateMode",
    "FileOwner",
    "FileGroup",
    "umask",
    "IncludeConfig",
    "OMFile",
    "SyslogFacility",
    "KLogPermitNonKernelFacility",
    "GSSForwardServiceName",
    "ReopenInterval",
    "LogRSyslogStatusMessages",
    "AbortOnUncleanConfig",
    "DropMsgsWithMaliciousDnsPtrRecords",
    "GenerateConfigGraph",
    "NonCancelableTermination",
    "ResetConfigVariables",
];

/// RAInerscript の既知オブジェクト名。
const KNOWN_OBJECTS: &[&str] = &[
    "module",
    "template",
    "input",
    "action",
    "global",
    "main_queue",
    "ruleset",
    "property",
    "constant",
    "lookup_table",
    "dyn_stats",
    "parser",
    "timezone",
];

/// rsyslog.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `$<name>` レガシーディレクティブ数。
    pub directives: usize,
    /// `object(`/`object(...)` RAInerscript オブジェクト宣言数。
    pub objects: usize,
    /// レガシーセレクタ行 (`facil[,facil]…  action`) 数。
    pub selectors: usize,
    /// リモート転送アクション (`@@`/`@`) 数。
    pub forwarders: usize,
    /// `if … then` 条件行数。
    pub conditionals: usize,
    /// `*.*`/`*.=`/`!prog`/`~`/`stop`/`&` 系特殊アクション行数。
    pub special_actions: usize,
}

/// `b` が rsyslog.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.directives + c.objects) >= 2 || (c.selectors >= 2 && c.forwarders >= 1) || c.selectors >= 3
}

/// syslog facilities (RFC 5424 names plus `local0`-`local7` and `*`).
const FACILITIES: &[&str] = &[
    "auth", "authpriv", "console", "cron", "daemon", "ftp", "kern", "lpr", "mail", "mark", "news",
    "security", "syslog", "user", "uucp", "local0", "local1", "local2", "local3", "local4",
    "local5", "local6", "local7",
];

fn looks_like_selector(line: &str) -> bool {
    // legacy selector: `facility.priority[,facility.priority]…  action`
    // (groups may also be `;`-separated). A bare word is not a selector.
    let Some(tok) = line.split_whitespace().next() else {
        return false;
    };
    if !tok.contains(['.', '*']) {
        return false;
    }
    tok.split(';').all(|grp| {
        !grp.is_empty()
            && grp.split(',').all(|piece| {
                let (fac, pri) = match piece.split_once('.') {
                    Some((f, p)) => (f, p),
                    None => (piece, ""),
                };
                (fac == "*" || FACILITIES.contains(&fac))
                    && (pri.is_empty()
                        || pri.bytes().all(|c| {
                            c.is_ascii_alphanumeric()
                                || matches!(c, b'=' | b'!' | b'<' | b'>' | b'*')
                        }))
            })
    })
}

/// rsyslog.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        directives: 0,
        objects: 0,
        selectors: 0,
        forwarders: 0,
        conditionals: 0,
        special_actions: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('$') {
            let name = rest.split([' ', '\t', '=']).next().unwrap_or("");
            if KNOWN_DIRECTIVES.contains(&name) {
                counts.directives += 1;
                saw_any = true;
            } else if !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphabetic() || c == b'_')
            {
                // 未認識の `$X` もディレクティブとして数える。
                counts.directives += 1;
                saw_any = true;
            }
            continue;
        }
        if let Some(open) = line.find('(') {
            let head = line[..open].trim();
            if KNOWN_OBJECTS.contains(&head) && line.ends_with(')') {
                counts.objects += 1;
                saw_any = true;
                continue;
            }
        }
        if (line.starts_with("if ") || line.starts_with("if("))
            && (line.contains("then") || line.ends_with('{'))
        {
            counts.conditionals += 1;
            saw_any = true;
            continue;
        }
        if line == "~" || line == "stop" || line.starts_with("& ") || line == "&" {
            counts.special_actions += 1;
            saw_any = true;
            continue;
        }
        if line.starts_with('*') || line.starts_with('!') || line.starts_with(':') {
            counts.special_actions += 1;
            saw_any = true;
            if line.contains('@') {
                counts.forwarders += 1;
            }
            continue;
        }
        if looks_like_selector(line) {
            counts.selectors += 1;
            saw_any = true;
            if line
                .split_whitespace()
                .nth(1)
                .is_some_and(|a| a.starts_with('@'))
            {
                counts.forwarders += 1;
            }
            continue;
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

    const SAMPLE: &[u8] = br#"# rsyslog.conf
$ModLoad imuxsock
$ModLoad imklog
$WorkDirectory /var/spool/rsyslog
$IncludeConfig /etc/rsyslog.d/*.conf
module(load="imudp")
input(type="imudp" port="514")
template(name="TraditionalFormat" type="string"
  string="%timegenerated% %HOSTNAME% %syslogtag%%msg%\n")
auth,authpriv.* /var/log/auth.log
*.*;auth,authpriv.none -/var/log/syslog
kern.* -/var/log/kern.log
mail.* -/var/log/mail.log
*.* @@logs.example.com:514
if $programname == 'sshd' then /var/log/sshd.log
& stop
"#;

    #[test]
    fn detects_rsyslog() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.directives, 4);
        assert_eq!(c.objects, 2);
        assert_eq!(c.selectors, 3);
        assert_eq!(c.forwarders, 1);
        assert_eq!(c.conditionals, 1);
        assert_eq!(c.special_actions, 3);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"key = value"));
    }
}
