//! Linux-HA Heartbeat `ha.cf` の解析。
//!
//! 平坦な `key value` 行: `logfile`/`keepalive`/`deadtime`/`warntime`/
//! `initdead`/`udpport`/`bcast`/`mcast`/`ucast`/`serial`/`baud`/`ping`/
//! `ping_group`/`node <name>`/`respawn`/`apiauth`/`crm`/`stonith`/
//! `auto_failback`/`deadping`/`watchdog`/`hbaping`/`ipfail`/`hbgenmethod`/
//! `use_logd`/`compression`/`traditional_compression`/`enable_logd`/
//! `conn_logd_time`/`compression_threshold`/`realtime`/`debug`/`debugfile`/
//! `logfacility`/`nocheck`/`rtosd` 等を検出し、設定種別の数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::hacf;
//!
//! let text = b"keepalive 2\ndeadtime 30\nnode ha1\nnode ha2\n";
//!
//! assert!(hacf::detect(text));
//! let c = hacf::parse(text).unwrap();
//! assert_eq!(c.nodes, 2);
//! ```

/// ha.cf の既知キー。
const KNOWN_KEYS: &[&str] = &[
    "logfile",
    "debugfile",
    "logfacility",
    "keepalive",
    "deadtime",
    "warntime",
    "initdead",
    "deadping",
    "udpport",
    "ping",
    "ping_group",
    "ping_node",
    "bcast",
    "mcast",
    "ucast",
    "serial",
    "baud",
    "hbmedia",
    "auto_failback",
    "node",
    "stonith",
    "stonith_host",
    "respawn",
    "apiauth",
    "crm",
    "use_logd",
    "enable_logd",
    "conn_logd_time",
    "compression",
    "traditional_compression",
    "compression_threshold",
    "coredumps",
    "watchdog",
    "hbaping",
    "ipfail",
    "hbgenmethod",
    "realtime",
    "debug",
    "nocheck",
    "rtosd",
    "msgfmt",
    "netstring",
    "uuidfrom",
    "autojoin",
    "mcast6",
    "ucast6",
    "bcast6",
    "v2check",
    "memdbg",
    "core",
    "auto_mcast",
    "cl_product",
    "cluster",
    "hb_standby",
];

/// ha.cf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 全キー=値行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known: usize,
    /// `node <name>` 宣言数。
    pub nodes: usize,
    /// `bcast`/`mcast`/`ucast`/`serial`/`mcast6`/`ucast6`/`bcast6` 通信媒体行数。
    pub media: usize,
    /// `ping`/`ping_group`/`ping_node`/`hbaping` 外部 ping 設定行数。
    pub pings: usize,
    /// `respawn`/`apiauth`/`stonith`/`crm`/`watchdog`/`hbaping`/`ipfail` 実行系行数。
    pub daemons: usize,
}

/// `b` が ha.cf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.known >= 2 && c.entries >= 3
}

/// ha.cf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        entries: 0,
        known: 0,
        nodes: 0,
        media: 0,
        pings: 0,
        daemons: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut words = line.split_whitespace();
        let Some(key) = words.next() else {
            continue;
        };
        // `key value` (空白区切り) または `key=value`。
        let key = key.trim_end_matches(':');
        if key.is_empty() {
            continue;
        }
        counts.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            counts.known += 1;
            saw_any = true;
            match key {
                "node" => counts.nodes += 1,
                "bcast" | "mcast" | "ucast" | "serial" | "hbmedia" | "mcast6" | "ucast6"
                | "bcast6" | "auto_mcast" => counts.media += 1,
                "ping" | "ping_group" | "ping_node" | "hbaping" => {
                    counts.pings += 1;
                }
                "respawn" | "apiauth" | "stonith" | "stonith_host" | "crm" | "watchdog"
                | "ipfail" => {
                    counts.daemons += 1;
                }
                _ => {}
            }
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

    const SAMPLE: &[u8] = br#"# ha.cf
logfile /var/log/ha-log
logfacility local0
keepalive 2
deadtime 30
warntime 10
initdead 120
udpport 694
bcast eth0
mcast eth0 225.0.0.1 694 1 0
ucast eth0 192.168.1.2
serial /dev/ttyS0
baud 19200
ping 192.168.1.254
ping_group group1 192.168.1.1 192.168.1.2
auto_failback on
node ha1
node ha2
respawn hacluster /usr/lib/heartbeat/ipfail
apiauth ipfail gid=haclient uid=hacluster
crm yes
stonith_host * external/ssh ha1 22
"#;

    #[test]
    fn detects_hacf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 21);
        assert_eq!(c.nodes, 2);
        assert_eq!(c.media, 4);
        assert_eq!(c.pings, 2);
        assert_eq!(c.daemons, 4);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"[section]\nkey = value"));
    }
}
