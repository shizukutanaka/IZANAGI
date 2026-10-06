//! `nzbget.conf` 検出モジュール。
//!
//! NZBGet の設定は平坦な `Key=Value` 形式で、`MainDir`/`DestDir`/
//! `InterDir`/`NzbDir`/`QueueDir`/`WebDir`/`ScriptDir`/`LockFile`/
//! `LogFile`/`ConfigTemplate`/`ControlIP`/`ControlPort`/
//! `ControlPassword`/`DaemonUserName`/`OutputMode`/`AppendCategoryDir`/
//! `ContinuePartial`/`DirectWrite`/`DupeCheck`/`ParCheck`/`ParRepair`/
//! `ParScan`/`ParTimeLimit`/`PostStrategy`/`SevenZipCmd`/`UnrarCmd`/
//! `ScriptPauseQueue`/`ShellOverride` 等のキー、および
//! `Server<N>.<opt>`/`Category<N>.<opt>`/`Task<N>.<opt>`/
//! `Feed<N>.<opt>`/`Unpack<N>.<opt>` 番号付きグループで構成される。
//!
//! ```
//! let b = b"MainDir=~/downloads\n\
//!           DestDir=${MainDir}/dst\n\
//!           NzbDir=${MainDir}/nzb\n\
//!           ControlPort=6789\n\
//!           Server1.Host=news.example.com\n\
//!           Server1.Port=563\n";
//! let c = izanagi_kit::nzbget::parse(b);
//! assert!(izanagi_kit::nzbget::detect(b));
//! assert_eq!(c.keys, 4);
//! assert_eq!(c.groups, 2);
//! ```

const KEYS: &[&str] = &[
    "AddUrl",
    "AppendCategoryDir",
    "AuthorizedIP",
    "CertCheck",
    "CertStore",
    "ConfigTemplate",
    "ContinuePartial",
    "ControlIP",
    "ControlPassword",
    "ControlPort",
    "CursesGroup",
    "CursesNzbName",
    "DaemonUserName",
    "DebugTarget",
    "Decode",
    "DeleteQueue",
    "DestDir",
    "DirectRename",
    "DirectUnpack",
    "DirectWrite",
    "DiskSpace",
    "DownloadRate",
    "DropPassword",
    "DropUsername",
    "DupeCheck",
    "ExtCleanupDisk",
    "FeedHistory",
    "FeedInterval",
    "FileLog",
    "FormAuth",
    "HealthCheck",
    "InfoTarget",
    "InterDir",
    "KeepHistory",
    "LogBuffer",
    "LogFile",
    "MainDir",
    "MonthlyQuota",
    "NzbDir",
    "NzbDirAge",
    "NzbDirFileAge",
    "NzbDirInterval",
    "OutputMode",
    "ParBuffer",
    "ParCheck",
    "ParCleanupQueue",
    "ParQuick",
    "ParRename",
    "ParRepair",
    "ParScan",
    "ParThreads",
    "ParTimeLimit",
    "PostStrategy",
    "PropagateErrors",
    "QuotaStartDay",
    "RawArticle",
    "ReloadQueue",
    "RequiredDir",
    "RetryOnCrcError",
    "SaveQueue",
    "ScanScript",
    "ScriptDir",
    "ScriptOrder",
    "ScriptPauseQueue",
    "SecureCert",
    "SecureControl",
    "SecurePort",
    "SevenZipCmd",
    "ShellOverride",
    "TempDir",
    "UMask",
    "UnrarCmd",
    "UnpackPauseQueue",
    "UnpackSimClean",
    "UpdateInterval",
    "UrlConnections",
    "UrlForce",
    "UserName",
    "WarningTarget",
    "WebDir",
    "WriteBuffer",
];

fn is_group(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    for p in ["Server", "Category", "Task", "Feed", "Unpack", "Extension"] {
        if let Some(rest) = k.strip_prefix(p) {
            let mut it = rest.splitn(2, '.');
            if it
                .next()
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
                && it.next().is_some_and(|s| !s.is_empty())
            {
                return true;
            }
        }
    }
    false
}

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が nzbget.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut groups = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if !tr.contains('=') {
            continue;
        }
        if is_group(tr) {
            groups += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (keys >= 1 && groups >= 1) || keys >= 3
}

/// nzbget.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct Nzbget {
    /// 既知キー行数。
    pub keys: usize,
    /// `Server<N>.*`/`Category<N>.*` 等番号付きグループ行数。
    pub groups: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を nzbget.conf として統計する。
pub fn parse(b: &[u8]) -> Nzbget {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Nzbget::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if !tr.contains('=') {
            continue;
        }
        if is_group(tr) {
            c.groups += 1;
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
        let b = b"MainDir=~/dl\nDestDir=${MainDir}/dst\nControlPort=6789\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_group() {
        let b = b"MainDir=~/dl\nServer1.Host=n.x\nServer1.Port=563\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.groups, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"MainDir=~/dl\n"));
        assert!(!detect(b"key = value\nfoo = bar\nbaz = quux\n"));
        assert!(!detect(b"Server1.Host=x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
