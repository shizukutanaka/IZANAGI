//! Minecraft `server.properties` census.
//!
//! `#` comment banner + `key=value` settings:
//! `motd`/`server-port`/`server-ip`/`max-players`/`difficulty`/`gamemode`/
//! `force-gamemode`/`hardcore`/`pvp`/`white-list`/`enforce-whitelist`/
//! `online-mode`/`view-distance`/`simulation-distance`/`level-name`/
//! `level-seed`/`level-type`/`generator-settings`/`generate-structures`/
//! `spawn-protection`/`spawn-monsters`/`spawn-animals`/`spawn-npcs`/
//! `allow-flight`/`allow-nether`/`enable-command-block`/`enable-query`/
//! `enable-rcon`/`rcon.port`/`rcon.password`/`query.port`/`snooper-enabled`/
//! `enable-jmx-monitoring`/`enable-status`/`hide-online-players`/
//! `entity-broadcast-range-percentage`/`function-permission-level`/
//! `op-permission-level`/`broadcast-console-to-ops`/`broadcast-rcon-to-ops`/
//! `enforce-secure-profile`/`prevent-proxy-connections`/`network-compression-threshold`/
//! `player-idle-timeout`/`max-tick-time`/`max-world-size`/`rate-limit`/
//! `resource-pack`/`resource-pack-id`/`resource-pack-prompt`/
//! `resource-pack-sha1`/`require-resource-pack`/`sync-chunk-writes`/
//! `use-native-transport`/`debug`/`text-filtering-config`/`initial-enabled-packs`/
//! `initial-disabled-packs`/`pause-when-empty-seconds`/`log-ips`/
//! `accepts-transfers`/`bug-report-link`/`max-chained-neighbor-updates`/
//! `monitor-level`/`region-file-compression`/`management-server-enabled`/
//! `management-server-host`/`management-server-port`/`management-server-secret`/
//! `status-heartbeat-interval`.
//!
//! ```rust
//! let p = "server-port=25565\nmax-players=20\ndifficulty=normal\npvp=true\n";
//! let c = izanagi_kit::serverprop::Serverprop::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.settings, 4);
//! assert_eq!(c.booleans, 1);
//! ```

/// server.properties census.
#[derive(Debug, Clone)]
pub struct Serverprop {
    /// `key=value` entries.
    pub settings: usize,
    /// Recognised server.properties key names.
    pub named: usize,
    /// Entries whose value is `true`/`false`.
    pub booleans: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "motd",
    "server-port",
    "server-ip",
    "max-players",
    "difficulty",
    "gamemode",
    "force-gamemode",
    "hardcore",
    "pvp",
    "white-list",
    "enforce-whitelist",
    "online-mode",
    "view-distance",
    "simulation-distance",
    "level-name",
    "level-seed",
    "level-type",
    "generator-settings",
    "generate-structures",
    "spawn-protection",
    "spawn-monsters",
    "spawn-animals",
    "spawn-npcs",
    "allow-flight",
    "allow-nether",
    "enable-command-block",
    "enable-query",
    "enable-rcon",
    "rcon.port",
    "rcon.password",
    "query.port",
    "snooper-enabled",
    "enable-jmx-monitoring",
    "enable-status",
    "hide-online-players",
    "entity-broadcast-range-percentage",
    "function-permission-level",
    "op-permission-level",
    "broadcast-console-to-ops",
    "broadcast-rcon-to-ops",
    "enforce-secure-profile",
    "prevent-proxy-connections",
    "network-compression-threshold",
    "player-idle-timeout",
    "max-tick-time",
    "max-world-size",
    "rate-limit",
    "resource-pack",
    "resource-pack-id",
    "resource-pack-prompt",
    "resource-pack-sha1",
    "require-resource-pack",
    "sync-chunk-writes",
    "use-native-transport",
    "debug",
    "text-filtering-config",
    "initial-enabled-packs",
    "initial-disabled-packs",
    "pause-when-empty-seconds",
    "log-ips",
    "accepts-transfers",
    "bug-report-link",
    "max-chained-neighbor-updates",
    "monitor-level",
    "region-file-compression",
    "management-server-enabled",
    "management-server-host",
    "management-server-port",
    "management-server-secret",
    "status-heartbeat-interval",
];

/// Whether the buffer looks like server.properties.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("server-port")
        || t.contains("max-players")
        || t.contains("view-distance")
        || t.contains("online-mode")
        || t.contains("motd=")
        || t.contains("enable-command-block")
        || t.contains("spawn-protection")
}

impl Serverprop {
    /// Parse a server.properties into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            named: 0,
            booleans: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            c.settings += 1;
            let key = s[..eq].trim();
            if KEYS.contains(&key) {
                c.named += 1;
            }
            if matches!(s[eq + 1..].trim(), "true" | "false") {
                c.booleans += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_props() {
        let b = concat!(
            "#Minecraft server properties\n",
            "#(File modify date)\n",
            "server-port=25565\n",
            "max-players=20\n",
            "motd=A Minecraft Server\n",
            "difficulty=normal\n",
            "gamemode=survival\n",
            "pvp=true\n",
            "online-mode=true\n",
            "view-distance=10\n",
            "enable-command-block=false\n",
            "spawn-protection=16\n",
            "white-list=false\n",
        );
        let c = Serverprop::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 11);
        assert_eq!(c.comments, 2);
        assert_eq!(c.named, 11);
        assert_eq!(c.booleans, 4);
    }

    #[test]
    fn rejects_other() {
        assert!(Serverprop::parse(b"foo=1").is_none());
    }
}
