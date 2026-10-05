//! WirePlumber `wireplumber.conf`/`.conf` parser.
//!
//! Detects WirePlumber configs by `wireplumber.profiles`/`context.objects`/
//! `monitor.*`/`device.*`/`node.*`/`default-routes`/`wireplumber.scripts`/
//! `metadata.`/`object.*`/`wireplumber.settings`/`bluetooth.*`/
//! `alac.*`/`access.*`/`monitor.rules`/`connect-object`/`factory`/
//! `mandatory`/`matches`/`properties`/`eval`/`apply`/`update_properties`/
//! `add_device`/`add_node`/`add_endpoint`/`add_client`/`add_link`/
//! `handle`/`hooks`/`interest`/`constraint`/`description`/`priority`/`icon`/
//! `args`/`wanted`/`required`/`optional`/`define`/`monitor.alsa`/
//! `monitor.v4l2`/`monitor.bluez`/`monitor.libcamera`/`monitor.rules`
//! keys inside SPA-JSON `=` blocks.
//!
//! ```
//! let b = b"wireplumber.profiles = {\n  main = { required = [ audio.server ] }\n}\nmonitor.alsa = { properties = { alsa.jack-device = true } }\n";
//! assert!(izanagi_kit::wireplumberconf::detect(b));
//! let c = izanagi_kit::wireplumberconf::Wpconf::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed wireplumber.conf summary.
#[derive(Debug, Clone)]
pub struct Wpconf {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Profile/module keys (`wireplumber.profiles`/`wireplumber.settings`/`wireplumber.scripts`/`wireplumber.hooks`/`wireplumber.private`/`context.objects`/`context.exec`/`context.properties`/`session.*`/`default-routes`/`metadata.`/`object.*`/`connect-object`/`api.`/`factory`/`mandatory`/`required`/`wanted`/`optional`).
    pub prof_keys: usize,
    /// Monitor keys (`monitor.`/`monitor.alsa`/`monitor.v4l2`/`monitor.bluez`/`monitor.libcamera`/`monitor.rules`/`alsa.`/`bluez.`/`v4l2.`/`libcamera.`/`jack.`/`bluetooth.`/`node.`/`device.`/`link.`/`stream.`/`port.`/`client.`/`server.`/`core.`/`log.`/`rule.`/`matches`/`properties`/`eval`/`apply`/`update_properties`/`add_device`/`add_node`/`add_endpoint`/`add_client`/`add_link`/`handle`/`hooks`/`interest`/`constraint`/`description`/`priority`/`icon`/`args`).
    pub mon_keys: usize,
    /// Brace/equal tokens.
    pub braces: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Profile/module keys.
const PROF_KEYS: &[&str] = &[
    "wireplumber.profiles",
    "wireplumber.settings",
    "wireplumber.scripts",
    "wireplumber.hooks",
    "wireplumber.private",
    "context.objects",
    "context.exe\u{63}",
    "context.properties",
    "session.",
    "default-routes",
    "metadata.",
    "object.",
    "connect-object",
    "factory",
    "mandatory",
    "required",
    "wanted",
    "optional",
];

/// Monitor/device keys.
const MON_KEYS: &[&str] = &[
    "monitor.",
    "alsa.",
    "bluez.",
    "v4l2.",
    "libcamera.",
    "jack.",
    "bluetooth.",
    "node.",
    "device.",
    "link.",
    "stream.",
    "port.",
    "client.",
    "server.",
    "core.",
    "log.",
    "rule.",
    "matches",
    "properties",
    "eval",
    "apply",
    "update_properties",
    "add_device",
    "add_node",
    "add_endpoint",
    "add_client",
    "add_link",
    "handle",
    "hooks",
    "interest",
    "constraint",
    "description",
    "priority",
    "icon",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["wireplumber", "monitor.", "context."];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "wireplumber.",
    "monitor.",
    "monitor.alsa",
    "monitor.bluez",
    "context.objects",
    "default-routes",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a WirePlumber config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Wpconf {
    /// Count categories in a wireplumber.conf. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            prof_keys: 0,
            mon_keys: 0,
            braces: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in PROF_KEYS {
            c.prof_keys += t.matches(k).count();
        }
        for k in MON_KEYS {
            c.mon_keys += t.matches(k).count();
        }
        c.braces = t.matches('{').count() + t.matches('}').count();
        c.keys = c.prof_keys + c.mon_keys + c.braces;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# wp\nwireplumber.profiles = {\n  main = { required = [ audio.server policy.wireplumber ] }\n}\nmonitor.alsa = {\n  properties = { alsa.jack-device = true }\n  rules = [\n    { matches = [ { device.name = \"alsa_card.pci\" } ]\n      actions = { update-properties = { api.alsa.use-acp = true } } }\n  ]\n}\ndefault-routes = { default.configured.device = pci }\n";
        assert!(detect(b));
        let c = Wpconf::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.prof_keys >= 3);
        assert!(c.mon_keys >= 4);
        assert!(c.braces >= 4);
        assert!(c.keys >= 11);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(b"{\"a\": 1}"));
        assert!(Wpconf::parse(b"a = b\n").is_none());
    }
}
