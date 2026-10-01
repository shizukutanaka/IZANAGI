//! Bukkit `bukkit.yml` census.
//!
//! Top-level sections `settings:`/`spawn-limits:`/`chunk-gc:`/
//! `ticks-per:`/`world-settings:`/`worlds:`/`aliases:`/`database:`
//! with nested `key: value` options:
//! `allow-end`/`warn-on-overload`/`permissions-file`/`update-folder`/
//! `ping-packet-limit`/`use-exact-login-location`/`world-container`/
//! `plugin-profiling`/`connection-throttle`/`query-plugins`/
//! `deprecated-verbose`/`shutdown-message`/`minimum-api`/
//! `use-imagemap-map-cache`/`item-dirty-ticks`/`check-updates`/
//! `monsters`/`animals`/`water-animals`/`water-ambient`/`water-underground-creature`/
//! `axolotls`/`ambient`/`period-in-ticks`/`load-threshold`/
//! `autosave`/`animal-spawns`/`monster-spawns`/`water-spawns`/`ambient-spawns`/
//! `chunk-gc`/`network-compression-threshold`/`spawn-radius`/`growth`/`entity-activation-range`/
//! `ticks-per-animal-spawns`/`ticks-per-monster-spawns`/etc.
//!
//! ```rust
//! let b = "settings:\n  allow-end: true\n  connection-throttle: 4000\nspawn-limits:\n  monsters: 70\n";
//! let c = izanagi_kit::bukkit::Bukkit::parse(b.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.settings, 3);
//! ```

/// bukkit.yml census.
#[derive(Debug, Clone)]
pub struct Bukkit {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// Nested `key: value` settings.
    pub settings: usize,
    /// Recognised bukkit.yml option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "allow-end",
    "warn-on-overload",
    "permissions-file",
    "update-folder",
    "ping-packet-limit",
    "use-exact-login-location",
    "world-container",
    "plugin-profiling",
    "connection-throttle",
    "query-plugins",
    "deprecated-verbose",
    "shutdown-message",
    "minimum-api",
    "use-imagemap-map-cache",
    "item-dirty-ticks",
    "check-updates",
    "monsters",
    "animals",
    "water-animals",
    "water-ambient",
    "water-underground-creature",
    "axolotls",
    "ambient",
    "period-in-ticks",
    "load-threshold",
    "autosave",
    "animal-spawns",
    "monster-spawns",
    "water-spawns",
    "ambient-spawns",
    "network-compression-threshold",
    "spawn-radius",
    "chunk-gc",
    "ticks-per",
    "spawn-limits",
    "world-settings",
    "worlds",
    "aliases",
    "database",
    "settings",
    "growth",
    "entity-activation-range",
    "view-distance",
    "simulation-distance",
    "ticks-per-animal-spawns",
    "ticks-per-monster-spawns",
    "ticks-per-water-spawns",
    "ticks-per-water-ambient-spawns",
    "ticks-per-ambient-spawns",
    "ticks-per-axolotl-spawns",
    "ticks-per-water-underground-creature-spawns",
    "ticks-per-autosave",
    "per-player-mob-spawns",
    "max-entity-collisions",
    "merge-radius",
    "mob-spawn-range",
    "entity-tracking-range",
    "tick-inactive-villagers",
    "ignore-unnamed-pets",
    "enable-zombie-pigmen-portal-spawns",
    "wither-spawn-sound-radius",
    "end-portal-sound-radius",
    "zombie-aggressive-towards-villager",
    "hanging-tick-frequency",
    "arrow-despawn-rate",
    "trident-despawn-rate",
    "item-merge-radius",
    "exp-merge-max-value",
    "ticks-per-item-despawn",
    "keep-spawn-loaded",
    "keep-spawn-loaded-range",
    "skip-entity-tick",
    "sync-chunk-writes",
    "max-tnt-per-tick",
    "max-tick-time",
    "legacy-max-player-count",
];

/// Whether the buffer looks like bukkit.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("connection-throttle")
        || t.contains("spawn-limits:")
        || t.contains("chunk-gc:")
        || t.contains("ticks-per:")
        || t.contains("allow-end")
        || t.contains("world-settings:")
}

impl Bukkit {
    /// Parse a bukkit.yml into census counts.
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
            "  allow-end: true\n",
            "  warn-on-overload: true\n",
            "  connection-throttle: 4000\n",
            "  plugin-profiling: false\n",
            "spawn-limits:\n",
            "  monsters: 70\n",
            "  animals: 10\n",
            "  water-animals: 5\n",
            "  ambient: 15\n",
            "chunk-gc:\n",
            "  period-in-ticks: 600\n",
            "  load-threshold: 0\n",
            "ticks-per:\n",
            "  animal-spawns: 400\n",
            "  monster-spawns: 1\n",
            "  autosave: 6000\n",
        );
        let c = Bukkit::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.settings, 13);
        assert_eq!(c.named, 17);
    }

    #[test]
    fn rejects_other() {
        assert!(Bukkit::parse(b"foo: 1").is_none());
    }
}
