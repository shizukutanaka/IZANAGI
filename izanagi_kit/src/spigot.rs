//! Spigot `spigot.yml` census.
//!
//! Top-level sections `settings:`/`messages:`/`world-settings:`/
//! `commands:`/`players:`/`stats:`/`advancements:` with nested options:
//! `timeout-time`/`restart-on-crash`/`restart-script`/`netty-threads`/
//! `late-bind`/`bungeecord`/`sample-count`/`player-shuffle`/
//! `save-user-cache-on-stop-only`/`moved-too-quickly-multiplier`/
//! `moved-wrongly-threshold`/`filter-creative-items`/`int-cache-limit`/
//! `user-cache-size`/`see-chunk-sends`/`max-player-list-size`/
//! `below-natural-generation`/`entity-activation-range`/`entity-tracking-range`/
//! `tick-inactive-villagers`/`nerf-spawner-mobs`/`mob-spawn-range`/
//! `max-tnt-per-tick`/`max-tick-time`/`item-despawn-rate`/`merge-radius`/
//! `arrow-despawn-rate`/`trident-despawn-rate`/`view-distance`/
//! `simulation-distance`/`growth`/`enable-zombie-pigmen-portal-spawns`/
//! `wither-spawn-sound-radius`/`end-portal-sound-radius`/
//! `zombie-aggressive-towards-villager`/`hanging-tick-frequency`/
//! `dragon-death-sound-radius`/`seed-village`/`seed-feature`/`seed-monument`/
//! `seed-slime`/`seed-mansion`/`seed-fortress`/`seed-endcity`/
//! `seed-stronghold`/`seed-ocean`/`log-full-seed`/`log-villager-deaths`/
//! `log-named-deaths`/`clear-tick-list`/`spam-exclusions`/`replace-commands`/
//! `silent-commandblock-console`/`command-blocks`/`tab-complete`/
//! `send-namespaced`/`world`/`default`/`nether`/`the_end`/`anticheat`/
//! `whitelist`/`unknown-command`/`server-full`/`outdated-client`/
//! `outdated-server`/`restart`/`timings`/`plugins`/`max*`.
//!
//! ```rust
//! let s = "settings:\n  timeout-time: 60\n  netty-threads: 4\nmessages:\n  whitelist: x\nworld-settings:\n  default:\n    view-distance: 8\n";
//! let c = izanagi_kit::spigot::Spigot::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.settings, 5);
//! ```

/// spigot.yml census.
#[derive(Debug, Clone)]
pub struct Spigot {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// Nested `key: value` settings (incl. per-world sub-section headers like `default:`/`nether:`).
    pub settings: usize,
    /// Recognised spigot.yml option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "timeout-time",
    "restart-on-crash",
    "restart-script",
    "netty-threads",
    "late-bind",
    "bungeecord",
    "sample-count",
    "player-shuffle",
    "save-user-cache-on-stop-only",
    "moved-too-quickly-multiplier",
    "moved-wrongly-threshold",
    "filter-creative-items",
    "int-cache-limit",
    "user-cache-size",
    "see-chunk-sends",
    "max-player-list-size",
    "below-natural-generation",
    "entity-activation-range",
    "entity-tracking-range",
    "tick-inactive-villagers",
    "nerf-spawner-mobs",
    "mob-spawn-range",
    "max-tnt-per-tick",
    "max-tick-time",
    "item-despawn-rate",
    "merge-radius",
    "arrow-despawn-rate",
    "trident-despawn-rate",
    "view-distance",
    "simulation-distance",
    "growth",
    "enable-zombie-pigmen-portal-spawns",
    "wither-spawn-sound-radius",
    "end-portal-sound-radius",
    "zombie-aggressive-towards-villager",
    "hanging-tick-frequency",
    "dragon-death-sound-radius",
    "seed-village",
    "seed-feature",
    "seed-monument",
    "seed-slime",
    "seed-mansion",
    "seed-fortress",
    "seed-endcity",
    "seed-stronghold",
    "seed-ocean",
    "log-full-seed",
    "log-villager-deaths",
    "log-named-deaths",
    "clear-tick-list",
    "spam-exclusions",
    "replace-commands",
    "silent-commandblock-console",
    "command-blocks",
    "tab-complete",
    "send-namespaced",
    "whitelist",
    "unknown-command",
    "server-full",
    "outdated-client",
    "outdated-server",
    "restart",
    "timings",
    "plugins",
    "default",
    "nether",
    "the_end",
    "settings",
    "messages",
    "world-settings",
    "commands",
    "players",
    "stats",
    "advancements",
    "anticheat",
    "history",
    "interval",
    "hidden-config",
    "combined-mutagen",
    "global-api-cache",
    "world",
    "attribute",
    "movement-speed",
    "follow-range",
    "attack-damage",
    "flying-speed",
    "jump-strength",
    "spawn-reinforcements",
    "armor",
    "armor-toughness",
    "attack-knockback",
    "knockback-resistance",
    "max-health",
    "spawn-count",
    "categories",
    "behaviors",
    "can-spawn",
    "experience",
    "loot-table",
    "guardian",
    "skeleton",
    "zombie",
    "item",
    "misc",
    "exp",
    "item-slots",
    "moved-too-quickly-threshold",
    "max-block-xray-notification-interval",
];

/// Whether the buffer looks like spigot.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("netty-threads")
        || t.contains("bungeecord")
        || t.contains("timeout-time")
        || t.contains("restart-script")
        || t.contains("world-settings:") && t.contains("view-distance")
        || t.contains("nerf-spawner-mobs")
        || t.contains("moved-wrongly-threshold")
}

impl Spigot {
    /// Parse a spigot.yml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            named: 0,
        };
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if indent == 0 && s.ends_with(':') {
                c.sections += 1;
                let key = s.trim_end_matches(':');
                if KEYS.contains(&key) {
                    c.named += 1;
                }
                continue;
            }
            if let Some(colon) = s.find(':') {
                c.settings += 1;
                let key = s[..colon].trim().trim_start_matches('-').trim();
                if KEYS.contains(&key) {
                    c.named += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_yml() {
        let b = concat!(
            "settings:\n",
            "  timeout-time: 60\n",
            "  restart-on-crash: true\n",
            "  restart-script: ./start.sh\n",
            "  netty-threads: 4\n",
            "  late-bind: false\n",
            "  bungeecord: false\n",
            "messages:\n",
            "  whitelist: You are not whitelisted\n",
            "  unknown-command: Unknown command\n",
            "  server-full: The server is full\n",
            "  outdated-client: Outdated client\n",
            "  outdated-server: Outdated server\n",
            "  restart: Server is restarting\n",
            "world-settings:\n",
            "  default:\n",
            "    view-distance: 8\n",
            "    simulation-distance: 6\n",
            "    mob-spawn-range: 4\n",
            "    item-despawn-rate: 6000\n",
            "    merge-radius:\n",
            "      item: 2\n",
            "      exp: 4\n",
            "    entity-activation-range:\n",
            "      animals: 32\n",
            "      monsters: 32\n",
            "  nether:\n",
            "    view-distance: 4\n",
        );
        let c = Spigot::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.settings, 25);
        assert!(c.named >= 20);
    }

    #[test]
    fn rejects_other() {
        assert!(Spigot::parse(b"foo: 1").is_none());
    }
}
