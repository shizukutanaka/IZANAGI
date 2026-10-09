//! Parser for Apache SkyWalking agent configuration (`agent.config`,
//! `apm-agent-core.config`, `agent/<name>.config`).
//!
//! Java-properties style `key=value` (or `key: value`) lines grouped by
//! dotted prefix: `agent.*` (service_name/namespace/cluster/instance_name/
//! authentication), `collector.*` (backend_service/grpc channel),
//! `plugin.*` (toolkit/instrumentation exclusions/jdbc/mongodb/kafka),
//! `profile.*`/`meter.*`/`osinfo.*`/`log.*`/`logging.*`/`statuscheck.*`,
//! `correlation.*`/`SW_*/sw_*` env-style keys, plus `#` comments.
//!
//! ```
//! let b = b"agent.service_name=svc\ncollector.backend_service=127.0.0.1:11800\n";
//! assert!(izanagi_kit::skywalking::detect(b));
//! let c = izanagi_kit::skywalking::Skywalking::parse(b).unwrap();
//! assert_eq!(c.settings, 2);
//! assert_eq!(c.named_groups, 2);
//! ```

use crate::textutil::strip_bom;
/// Parsed skywalking agent.config summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skywalking {
    /// Total `key=value`/`key: value` settings.
    pub settings: usize,
    /// Distinct dotted-prefix groups (`agent`, `collector`, `plugin`, `profile`, `meter`, `osinfo`, `log`, `logging`, `statuscheck`, `correlation`, `jvm`, `buffer`, `tracing`, `SW_`, `sw_`).
    pub named_groups: usize,
    /// `agent.*` keys.
    pub agent_keys: usize,
    /// `plugin.*` keys (incl. exclusions like `plugin.activator.*`, `plugin.instrumentation.*`, `plugin.jdbc.*`, `plugin.exclude_plugins`).
    pub plugin_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const GROUPS: &[&str] = &[
    "agent",
    "collector",
    "plugin",
    "profile",
    "meter",
    "osinfo",
    "log",
    "logging",
    "statuscheck",
    "correlation",
    "jvm",
    "buffer",
    "tracing",
    "SW",
    "sw",
];

/// Detects skywalking-style properties.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("agent.service_name") || s.starts_with("collector.backend_service") {
            hits += 2;
        }
        if s.starts_with("agent.") || s.starts_with("plugin.") || s.starts_with("collector.") {
            hits += 1;
        }
        if s.contains("skywalking") || s.contains("SW_") {
            hits += 1;
        }
    }
    hits >= 3
}

impl Skywalking {
    /// Parse a skywalking config; `None` when no settings found.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let Ok(t) = core::str::from_utf8(b) else {
            return None;
        };
        let t = strip_bom(t);
        let mut c = Self {
            settings: 0,
            named_groups: 0,
            agent_keys: 0,
            plugin_keys: 0,
            comments: 0,
        };
        let mut seen = [0usize; GROUPS.len()];
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with('!') {
                c.comments += 1;
                continue;
            }
            let Some(sep) = s.find('=').or_else(|| s.find(": ")) else {
                continue;
            };
            let key = s[..sep].trim_end();
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|x| x.is_ascii_alphanumeric() || x == b'.' || x == b'_' || x == b'$')
            {
                continue;
            }
            c.settings += 1;
            let group = key.split('.').next().unwrap_or("");
            if let Some(gi) = GROUPS.iter().position(|g| *g == group) {
                if seen[gi] == 0 {
                    seen[gi] = 1;
                    c.named_groups += 1;
                }
            }
            if key.starts_with("agent.") {
                c.agent_keys += 1;
            }
            if key.starts_with("plugin.") {
                c.plugin_keys += 1;
            }
        }
        (c.settings > 0).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sw() {
        assert!(detect(
            b"agent.service_name=x\nagent.namespace=prod\ncollector.backend_service=h:11800\n"
        ));
        assert!(!detect(b"foo=bar\n"));
    }

    #[test]
    fn groups_counted() {
        let c = Skywalking::parse(
            b"agent.service_name=x\nagent.namespace=p\ncollector.backend_service=h\nplugin.toolkit.log.grpc.reporter.server_host=h\nprofile.active=true\n",
        )
        .unwrap();
        assert_eq!(c.settings, 5);
        assert_eq!(c.named_groups, 4);
        assert_eq!(c.agent_keys, 2);
        assert_eq!(c.plugin_keys, 1);
    }

    #[test]
    fn rejects() {
        assert!(Skywalking::parse(b"no kv\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
