//! PipeWire `pipewire.conf`/`.conf` (SPA-JSON) parser.
//!
//! Detects PipeWire configs by `context.objects`/`context.exec`/
//! `context.properties`/`context.modules`/`node.*`/`stream.*`/`log.level`/
//! `core.daemon`/`clock.*`/`default.clock.*`/`default.video.*`/
//! `default.clock.quantum`/`session.*`/`upmix.*`/`splits.*`/`adapter.*`/
//! `module.x11-bell`/`wireplumber.*`/`suspend.timeout-seconds` SPA-style
//! keys inside `{ }` blocks with `=`/`=` separation.
//!
//! ```
//! let b = b"context.properties = {\n  log.level = 2\n  default.clock.rate = 48000\n  default.clock.quantum = 1024\n}\ncontext.objects = [\n  { factory = module-null-node\n    args = { node.name = \"null\" } }\n]\n";
//! assert!(izanagi_kit::pipewireconf::detect(b));
//! let c = izanagi_kit::pipewireconf::Pwconf::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed pipewire.conf summary.
#[derive(Debug, Clone)]
pub struct Pwconf {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Context keys (`context.objects`/`context.exec`/`context.properties`/`context.modules`/`context.spa-libs`/`context.data-`/`context.generate`/`.
    pub ctx_keys: usize,
    /// Node/stream keys (`node.`/`stream.`/`port.`/`link.`/`session.`/`wireplumber.`/`log.`/`core.`/`clock.`/`default.clock.`/`default.video.`/`upmix.`/`splits.`/`adapter.`/`suspend.`/`channelmix.`/`resample.`/`rule.`/`monitor.`/`default.`/`sec.`/`lib.`/`api.`/`access.`/`module.`/`exec.`/`factory.`/`args`/`flags`/`priority.driver`/`priority.session`/`stream.*`/`node.*`/`target.`/`server.dbus.`/`x11-bell`/`data.`).
    pub node_keys: usize,
    /// Structure tokens (`{`/`}`/`[`/`]`/`=`/`ifexists`/`optional`/`include`).
    pub struct_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Context keys.
const CTX_KEYS: &[&str] = &[
    "context.objects",
    "context.exe\u{63}",
    "context.properties",
    "context.modules",
    "context.spa-libs",
    "context.data-",
    "context.generate",
];

/// Node/stream keys.
const NODE_KEYS: &[&str] = &[
    "node.",
    "stream.",
    "port.",
    "link.",
    "session.",
    "wireplumber.",
    "log.",
    "core.",
    "clock.",
    "default.clock.",
    "default.video.",
    "default.dsp.",
    "upmix.",
    "splits.",
    "adapter.",
    "suspend.",
    "channelmix.",
    "resample.",
    "rule.",
    "monitor.",
    "target.",
    "server.dbus.",
    "x11-bell",
    "access.",
    "api.",
    "module.",
    "exec.",
    "factory.",
    "priority.driver",
    "priority.session",
];

/// Structure keys.
const STRUCT_KEYS: &[&str] = &["{", "}", "[", "]", "=", "ifexists", "optional", "include"];

/// Anchor tokens.
const ANCHORS: &[&str] = &["context.", "node.", "clock.", "factory ="];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "context.properties",
    "context.objects",
    "context.exe\u{63}",
    "default.clock.",
    "node.",
    "stream.",
    "wireplumber.",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a PipeWire config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Pwconf {
    /// Count categories in a pipewire.conf. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            ctx_keys: 0,
            node_keys: 0,
            struct_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in CTX_KEYS {
            c.ctx_keys += t.matches(k).count();
        }
        for k in NODE_KEYS {
            c.node_keys += t.matches(k).count();
        }
        for k in STRUCT_KEYS {
            c.struct_keys += t.matches(k).count();
        }
        c.keys = c.ctx_keys + c.node_keys + c.struct_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# pipewire\ncontext.properties = {\n  log.level = 2\n  default.clock.rate = 48000\n  default.clock.quantum = 1024\n  suspend.timeout-seconds = 0\n}\ncontext.objects = [\n  { factory = adapter\n    args = { factory.name = api.alsa.seq.bridge node.name = Midi-Bridge }\n    flags = [ ifexists ]\n  }\n]\n";
        assert!(detect(b));
        let c = Pwconf::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.ctx_keys >= 2);
        assert!(c.node_keys >= 3);
        assert!(c.struct_keys >= 6);
        assert!(c.keys >= 11);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(b"{\"a\": 1, \"b\": 2}"));
        assert!(Pwconf::parse(b"a = b\n").is_none());
    }
}
