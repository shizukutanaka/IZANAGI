//! ZeekControl 設定ファイル (`node.cfg`, `zeekctl.cfg`, `control.cfg`) パーサ。
//!
//! `[manager]`/`[proxy-*]`/`[logger]`/`[worker-*]`/`[standalone]` セクションと
//! `type`/`host`/`interface`/`lb_method`/`pin_cpus` ノードキー、
//! `LogDir`/`SpoolDir`/`MailTo` 等 zeekctl.cfg キーを計数する。
//!
//! ```
//! use izanagi_kit::zeekctl;
//! let conf = b"[logger]\ntype = logger\nhost = localhost\n[manager]\ntype = manager\nhost = localhost\n";
//! assert!(zeekctl::detect(conf));
//! let c = zeekctl::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.entries, 4);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "type",
    "host",
    "interface",
    "lb_method",
    "lb_procs",
    "pin_cpus",
    "aux_scripts",
    "env_vars",
    "ethernet_link_speed",
    "ipv6_first_addr",
    "LogDir",
    "SpoolDir",
    "User",
    "MinDiskSpace",
    "MailTo",
    "MailSendmail",
    "TimeFmt",
    "LBMethod",
    "PinCommand",
    "BinDir",
    "CfgDir",
    "ScriptsDir",
    "HelpDir",
    "DocDir",
    "ShareDir",
    "TracesDir",
    "Version",
    "SiteName",
    "HostName",
    "CronCmd",
    "MakeArchiveName",
    "FileRotation",
    "DebugLevel",
    "NiceLevel",
    "BroUser",
    "BroGroup",
    "PostProcessorSuffix",
    "LogRotationInterval",
    "LogExpireInterval",
    "RemoteCommand",
    "SSHBinary",
    "CompressLogs",
    "CompressCmd",
    "CompressExtension",
    "MailSubjectPrefix",
    "SendMail",
    "MailReplyTo",
    "PrivateAddressSpace",
    "NetConfigWarnDelete",
    "PF_RINGClusterID",
    "PF_RINGClusterType",
    "AfPacketBufferSize",
    "AfPacketFanoutID",
    "AfPacketFanoutMode",
];

fn known_section(name: &str) -> bool {
    matches!(name, "manager" | "logger" | "standalone")
        || name.starts_with("proxy")
        || name.starts_with("worker")
}

/// 簡易判定 (既知セクション or 既知キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.known_sections >= 1 && c.entries >= 2) || c.known_keys >= 3
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    let mut found = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            c.sections += 1;
            if known_section(t[1..t.len() - 1].trim()) {
                c.known_sections += 1;
            }
            found = true;
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim();
        if key.is_empty() || key.bytes().any(|b| b == b' ') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            c.known_keys += 1;
        }
        found = true;
    }
    found.then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"[manager]\ntype = manager\nhost = manager.local\n[proxy-1]\ntype = proxy\nhost = proxy.local\n[worker-eth0]\ntype = worker\nhost = eth0.local\ninterface = eth0\nlb_method = pf_ring\npin_cpus = 4,5\n";

    #[test]
    fn detects_zeekctl() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 9);
        assert_eq!(c.known_keys, 9);
    }

    #[test]
    fn detects_zeekctl_cfg() {
        assert!(detect(b"LogDir = /usr/local/zeek/logs\nSpoolDir = /usr/local/zeek/spool\nMailTo = admin@example.com\n"));
    }

    #[test]
    fn rejects_generic_ini() {
        assert!(!detect(b"[server]\nhost = x\nport = 1\n"));
        assert!(!detect(b"[main]\nfoo = 1\nbar = 2\nbaz = 3\n"));
    }
}
