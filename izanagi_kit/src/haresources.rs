//! Linux-HA Heartbeat `haresources` ファイルの解析。
//!
//! `<primary-node> <resource1> <resource2> …` 形式の行を検出し、
//! `IPaddr::`/`Filesystem::`/`ldirectord::`/`mailTo::`/`Db2::` 等の
//! リソースエージェント、サービス名、行継続 (`\`) を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::haresources;
//!
//! let text = b"ha1 IPaddr::192.168.1.100/24/eth0 httpd\n";
//!
//! assert!(haresources::detect(text));
//! let c = haresources::parse(text).unwrap();
//! assert_eq!(c.entries, 1);
//! ```

/// haresources の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 論理エントリ数 (継続行をまとめた行単位)。
    pub entries: usize,
    /// 総リソース語数。
    pub resources: usize,
    /// `Agent::arg1::arg2` 形式のリソース数。
    pub colon_agents: usize,
    /// `IPaddr*`/`SendArp`/`IPsrcaddr` IP 系リソース数。
    pub ip_resources: usize,
    /// `Filesystem`/`drbddisk`/`LVM`/`RAID1`/`OCFS2` 等ストレージ系数。
    pub storage_resources: usize,
    /// `mailTo`/`VirtualDomain`/`WinPopup`/`AudibleAlarm` 通知系数。
    pub notify_resources: usize,
    /// `\` で終わる継続行数。
    pub continuations: usize,
    /// プレーンスクリプト名リソース数 (httpd, sshd 等)。
    pub script_resources: usize,
}

/// `b` が haresources らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    // Real haresources entries carry at least one `Agent::params` resource
    // (IPaddr::, Filesystem::, DRBD::, …); requiring a colon agent keeps
    // arbitrary multi-word text lines from being claimed.
    c.entries >= 1 && c.resources >= 2 && c.colon_agents >= 1
}

fn agent_name(word: &str) -> &str {
    word.split("::").next().unwrap_or(word)
}

fn is_storage(agent: &str) -> bool {
    matches!(
        agent,
        "Filesystem"
            | "drbddisk"
            | "LVM"
            | "RAID1"
            | "OCFS2"
            | "EVMS"
            | "LinuxSCSI"
            | "raid1"
            | "iSCSILogicalUnit"
            | "iSCSITarget"
            | "scsitarget"
            | "scsi2reservation"
            | "ManageRAID"
            | "ServeRAID"
            | "drbd"
            | "vgchange"
            | "fio"
    )
}

fn is_notify(agent: &str) -> bool {
    matches!(
        agent,
        "mailTo" | "VirtualDomain" | "WinPopup" | "AudibleAlarm" | "sjohns"
    )
}

/// haresources を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        entries: 0,
        resources: 0,
        colon_agents: 0,
        ip_resources: 0,
        storage_resources: 0,
        notify_resources: 0,
        continuations: 0,
        script_resources: 0,
    };
    let mut saw_any = false;
    let mut continuing = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.as_bytes().last() == Some(&92) {
            counts.continuations += 1;
        }
        let mut words = line.split_whitespace();
        if continuing {
            // 継続行: 先頭はリソース名 (ノード名なし)。
            continuing = line.as_bytes().last() == Some(&92);
            for w in words {
                classify(w, &mut counts);
            }
            continue;
        }
        let Some(_node) = words.next() else {
            continue;
        };
        counts.entries += 1;
        saw_any = true;
        continuing = line.as_bytes().last() == Some(&92);
        for w in words {
            classify(w, &mut counts);
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

fn classify(word: &str, counts: &mut Counts) {
    if word == "\\" {
        return;
    }
    counts.resources += 1;
    let agent = agent_name(word);
    if word.contains("::") {
        counts.colon_agents += 1;
    } else {
        counts.script_resources += 1;
    }
    if agent.starts_with("IP") || agent == "SendArp" {
        counts.ip_resources += 1;
    }
    if is_storage(agent) {
        counts.storage_resources += 1;
    }
    if is_notify(agent) {
        counts.notify_resources += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"# haresources
ha1 IPaddr::192.168.1.100/24/eth0 Filesystem::/dev/drbd0::/data::ext3 httpd
ha2 IPaddr2::192.168.1.101/24/eth1 Filesystem::/dev/sdb1::/backup::xfs \
    mysqld mailTo::admin@example.com::FAILURE
ha1 IPsrcaddr::192.168.1.100 eth0::dhcp
"#;

    #[test]
    fn detects_haresources() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 3);
        assert_eq!(c.resources, 9);
        assert_eq!(c.colon_agents, 7);
        assert_eq!(c.ip_resources, 3);
        assert_eq!(c.continuations, 1);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"node alpha beta gamma\n"));
        assert!(!detect(b"# comment only\n"));
    }
}
