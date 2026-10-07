//! Minecraft `server.properties` 検出モジュール。
//!
//! 平坦な `key=value` 形式で、`motd`/`server-port`/`max-players`/
//! `difficulty`/`gamemode`/`online-mode`/`white-list`/`level-name`/
//! `level-seed`/`level-type`/`spawn-protection`/`spawn-monsters`/
//! `view-distance`/`simulation-distance`/`enable-command-block`/
//! `enable-rcon`/`enable-query`/`allow-flight`/`allow-nether`/
//! `pvp`/`hardcore`/`force-gamemode`/`generate-structures`/
//! `generator-settings`/`max-world-size`/`network-compression-threshold`/
//! `max-tick-time`/`use-native-transport`/`op-permission-level`/
//! `function-permission-level`/`player-idle-timeout`/
//! `prevent-proxy-connections`/`entity-broadcast-range-percentage`/
//! `text-filtering-config`/`rate-limit`/`hide-online-players`/
//! `snooper-enabled`/`broadcast-rcon-to-ops`/`broadcast-console-to-ops`/
//! `resource-pack`/`resource-pack-sha1`/`require-resource-pack`/
//! `sync-chunk-writes`/`server-ip`/`query.port`/`rcon.port`/
//! `rcon.password`/`enable-status`/`enable-jmx-monitoring`/`debug`/
//! `pause-when-empty-seconds`/`initial-enabled-packs`/
//! `initial-disabled-packs`/`bug-report-link`/`management-server-*`/
//! `status-heartbeat-interval` 等のキーで構成される。
//!
//! ```
//! let b = b"motd=A Minecraft Server\n\
//!           server-port=25565\n\
//!           max-players=20\n\
//!           difficulty=easy\n\
//!           gamemode=survival\n\
//!           online-mode=true\n\
//!           white-list=false\n";
//! let c = izanagi_kit::mcserverprops::parse(b);
//! assert!(izanagi_kit::mcserverprops::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEYS: &[&str] = &[
    "allow-flight",
    "allow-nether",
    "broadcast-console-to-ops",
    "broadcast-rcon-to-ops",
    "bug-report-link",
    "debug",
    "difficulty",
    "enable-command-block",
    "enable-jmx-monitoring",
    "enable-query",
    "enable-rcon",
    "enable-status",
    "enforce-secure-profile",
    "enforce-whitelist",
    "entity-broadcast-range-percentage",
    "force-gamemode",
    "function-permission-level",
    "gamemode",
    "generate-structures",
    "generator-settings",
    "hardcore",
    "hide-online-players",
    "initial-disabled-packs",
    "initial-enabled-packs",
    "level-name",
    "level-seed",
    "level-type",
    "log-ips",
    "management-server-allowed-origins",
    "management-server-enabled",
    "management-server-host",
    "management-server-port",
    "management-server-secret",
    "max-chained-neighbor-updates",
    "max-players",
    "max-tick-time",
    "max-world-size",
    "motd",
    "network-compression-threshold",
    "online-mode",
    "op-permission-level",
    "pause-when-empty-seconds",
    "player-idle-timeout",
    "prevent-proxy-connections",
    "pvp",
    "query.port",
    "rate-limit",
    "rcon.password",
    "rcon.port",
    "region-file-compression",
    "require-resource-pack",
    "resource-pack",
    "resource-pack-id",
    "resource-pack-prompt",
    "resource-pack-sha1",
    "server-ip",
    "server-port",
    "simulation-distance",
    "snooper-enabled",
    "spawn-animals",
    "spawn-monsters",
    "spawn-npcs",
    "spawn-protection",
    "status-heartbeat-interval",
    "sync-chunk-writes",
    "text-filtering-config",
    "use-native-transport",
    "view-distance",
    "white-list",
];

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が server.properties に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    keys >= 4
}

/// server.properties の統計。
#[derive(Debug, Default, Clone)]
pub struct McServerProps {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を server.properties として統計する。
pub fn parse(b: &[u8]) -> McServerProps {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = McServerProps::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
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
        let b = b"motd=x\nserver-port=25565\nmax-players=20\ndifficulty=easy\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"motd=x\nserver-port=25565\nmax-players=20\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\nquux2=x\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# motd=x\n# server-port=25565\n# max-players=20\n# difficulty=easy\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 4);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
