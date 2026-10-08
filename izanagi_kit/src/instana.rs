//! Parser for Instana agent configuration (`configuration.yaml` with the
//! `com.instana.plugin.*` plugin blocks, `maven-releases/instana` bundle
//! layout) and `instana-agent.properties`/`agent.properties`.
//!
//! Counts `key: value`/`key=value` entries plus the plugin-block idiom
//! `com.instana.plugin.<name>:` (one section per monitored technology —
//! e.g. `com.instana.plugin.db2`, `com.instana.plugin.kafka`,
//! `com.instana.plugin.openshift`, `com.instana.plugin.action`),
//! `secrets` / `configuration file` references, `- item` list entries
//! (interfaces/processes/disk/users), `enabled:` toggles, and `#` comments.
//!
//! ```
//! let b = b"com.instana.plugin.db2:\n  user: u\ncom.instana.plugin.kafka:\n  enabled: true\n";
//! assert!(izanagi_kit::instana::detect(b));
//! let c = izanagi_kit::instana::Instana::parse(b).unwrap();
//! assert_eq!(c.plugin_sections, 2);
//! ```

/// Parsed instana conf summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instana {
    /// Total `key:`/`key=value` entries.
    pub keys: usize,
    /// `com.instana.plugin.<name>:` section headers.
    pub plugin_sections: usize,
    /// Other `key:` blocks (non-plugin `key:` ending lines — e.g. `secrets:`, `configuration file` groups).
    pub sections: usize,
    /// `enabled:`/`mode:`/`poll_rate:` toggle-ish keys.
    pub toggles: usize,
    /// `- item` list entries.
    pub list_items: usize,
    /// `#` comment lines.
    pub comments: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detects instana configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("com.instana.plugin") {
            hits += 2;
        }
        if s.contains("instana") {
            hits += 1;
        }
        if s.starts_with("agent_key") || s.starts_with("zone.name") || s.starts_with("host.agent") {
            hits += 1;
        }
    }
    hits >= 2
}

fn key_of(s: &str) -> &str {
    match s.find(':') {
        Some(c) => s[..c]
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-'))
            .next()
            .unwrap_or(""),
        None => "",
    }
}

impl Instana {
    /// Parse instana conf; `None` when nothing counted.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let Ok(t) = core::str::from_utf8(b) else {
            return None;
        };
        let t = strip_bom(t);
        let mut c = Self {
            keys: 0,
            plugin_sections: 0,
            sections: 0,
            toggles: 0,
            list_items: 0,
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
            if s.starts_with("- ") || s == "-" {
                c.list_items += 1;
                continue;
            }
            if s.starts_with("com.instana.plugin") && l.trim_end().ends_with(':') {
                c.plugin_sections += 1;
                c.keys += 1;
                continue;
            }
            let key = if s.contains('=') {
                s[..s.find('=').unwrap_or(0)].trim()
            } else {
                key_of(s)
            };
            if key.is_empty() {
                continue;
            }
            c.keys += 1;
            if l.trim_end().ends_with(':') {
                c.sections += 1;
            }
            if key == "enabled" || key == "mode" || key == "poll_rate" || key == "metric_level" {
                c.toggles += 1;
            }
        }
        (c.keys > 0).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_instana() {
        assert!(detect(b"com.instana.plugin.db2:\n  user: u\n"));
        assert!(detect(b"agent_key=k\nzone.name=z\n"));
        assert!(!detect(b"plugin:\n  a: b\n"));
    }

    #[test]
    fn counts_plugins() {
        let c = Instana::parse(
            b"com.instana.plugin.db2:\n  user: u\n  password: p\ncom.instana.plugin.kafka:\n  enabled: true\nsecrets:\n  - k1\n",
        )
        .unwrap();
        assert_eq!(c.plugin_sections, 2);
        assert_eq!(c.sections, 1);
        assert_eq!(c.toggles, 1);
        assert_eq!(c.list_items, 1);
        assert!(c.keys >= 6);
    }

    #[test]
    fn rejects() {
        assert!(Instana::parse(b"nada\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
