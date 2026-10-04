//! IBM MQ スタンザ設定(`qm.ini`/`mqs.ini`/`mqclient.ini`)の検出と構造カウント。
//!
//! `QueueManager:`/`Service:`/`TCP:`/`Log:`/`CHANNELS:`/`ExitPath:`/
//! `AllQueueManagers:`/`DefaultQueueManager:`/`ClientExitPath:`/
//! `ApiExitLocal:`/`TuningParameters:`/`SSL:`/`Security:` スタンザ行と、
//! `Name=`/`Directory=`/`Prefix=`/`Port=`/`Service=`/`Module=`/`Function=`/
//! `LogPrimaryFiles=` 等の `Key=Value` 行を識別する。
//!
//! ```
//! let c = izanagi_kit::ibmmq::parse(
//!     b"QueueManager:\n   Name=QM1\n   Prefix=/var/mqm\n   Directory=QM1\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::ibmmq::detect(b"TCP:\n   Port=1414\n   IPAddress=0.0.0.0\n"));
//! ```

/// 既知スタンザ名(行が `Name:` で終わる)。
const STANZAS: &[&str] = &[
    "AllQueueManagers",
    "ApiExitCommon",
    "ApiExitLocal",
    "ApiExitTemplate",
    "CHANNELS",
    "ClientExitPath",
    "ConnectionFactoryParms",
    "DefaultQueueManager",
    "ExitPath",
    "ExitProperties",
    "FileSystem",
    "Installations",
    "Log",
    "LU62",
    "NETBIOS",
    "QueueManager",
    "RestrictedMode",
    "Security",
    "Service",
    "ServiceComponent",
    "SPX",
    "SSL",
    "TCP",
    "TuningParameters",
    "XAResourceManager",
];

/// 既知キー。
const KEYS: &[&str] = &[
    "AccessMode",
    "AdoptNewMCA",
    "AdoptNewMCACheck",
    "AdoptNewMCATimeout",
    "API",
    "ApplicationName",
    "AuthenticationInformation",
    "AutostartService",
    "Channel",
    "ChlauthEarlyAdopt",
    "ClientChannelTable",
    "ClntSndBuffSize",
    "ClntRcvBuffSize",
    "ClusterName",
    "Clusters",
    "ComponentData",
    "Connect_Timeout",
    "ConnectionMode",
    "ConsoleMode",
    "Data",
    "DefaultBindQ",
    "DefaultQueueManagerName",
    "DevideFileSystemNames",
    "Directory",
    "DSAParameters",
    "FileSize",
    "Function",
    "Group",
    "IPAddress",
    "KeepAlive",
    "LogBufferPages",
    "LogFilePages",
    "LogPath",
    "LogPrimaryFiles",
    "LogSecondaryFiles",
    "LogType",
    "MaxActiveChannels",
    "MaxChannels",
    "MaxPropertiesSize",
    "Module",
    "MQIBindType",
    "Name",
    "NetbiosName",
    "Password",
    "Policy",
    "Port",
    "Prefix",
    "Protocol",
    "QMLogFilePageSize",
    "QueueManagerName",
    "RequiredComponent",
    "SecurityPolicy",
    "SeqNumber",
    "Sequence",
    "ServerConnectionChannels",
    "Service",
    "ServiceComponent",
    "StartArgs",
    "StartCmd",
    "StopArgs",
    "StopCmd",
    "StanzaType",
    "Status",
    "TCPKeepAlive",
    "TpName",
    "UseMQIAuthentication",
    "Version",
    "WorkstationServer",
];

/// ibmmq 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知スタンザ行。
    pub sections: usize,
    /// 既知 `Key=Value` 行。
    pub options: usize,
    /// `;`/`#`/`*` コメント行。
    pub comments: usize,
    /// 分類不能行(未知キー・その他)。
    pub misc: usize,
}

/// `Key=` 前のキー名。
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

/// b が qm.ini/mqs.ini かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') || t.starts_with('*') {
            continue;
        }
        let stanza = t
            .strip_suffix(':')
            .filter(|s| s.chars().all(|c| c.is_ascii_alphanumeric()));
        if stanza.is_some_and(|s| STANZAS.contains(&s))
            || kv_key(t).is_some_and(|k| KEYS.contains(&k))
        {
            hits += 1;
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with(';') || t.starts_with('#') || t.starts_with('*') {
            c.comments += 1;
            continue;
        }
        let stanza = t
            .strip_suffix(':')
            .filter(|s| s.chars().all(|c| c.is_ascii_alphanumeric()));
        if let Some(s) = stanza {
            if STANZAS.contains(&s) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if let Some(k) = kv_key(t) {
            if KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"* qm.ini\nQueueManager:\n   Name=QM1\n   Prefix=/var/mqm\n   Directory=QM1\n   FileSize=4096\nTCP:\n   Port=1414\n   IPAddress=0.0.0.0\nService:\n   Name=AuthorizationService\n   EntryPoints=14\n   SharedLocksUserSupplied=0\nLog:\n   LogPrimaryFiles=3\n   LogSecondaryFiles=2\n   LogFilePages=4096\n   LogType=CIRCULAR\n";

    #[test]
    fn ibmmq() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert!(c.options >= 9);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_ibmmq() {
        assert!(!detect(b"[section]\nkey=value\n"));
        assert!(!detect(b"Foo:\n   Name=x\n"));
    }
}
