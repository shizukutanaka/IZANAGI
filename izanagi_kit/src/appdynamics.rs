//! Parser for AppDynamics agent configuration (`controller-info.xml` for
//! Java/.NET/machine agents, plus `app-agent-config.xml` shapes).
//!
//! Counts `<controller-info>` envelope attributes (`controller-host`,
//! `controller-port`, `controller-ssl-enabled`, `account-name`,
//! `account-access-key`, `application-name`, `tier-name`, `node-name`,
//! `agent-type`, `season`), `<property name="..." value="..."/>` extension
//! entries, `<application>`/`<tier>`/`<node>` blocks inside
//! `<application-info>`/`<machine-agent>` containers, and `<?xml` header.
//!
//! ```
//! let b = b"<controller-info controller-host=\"h\"><property name=\"x\" value=\"y\"/></controller-info>";
//! assert!(izanagi_kit::appdynamics::detect(b));
//! let c = izanagi_kit::appdynamics::Appdynamics::parse(b).unwrap();
//! assert_eq!(c.properties, 1);
//! ```

/// Parsed controller-info.xml summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appdynamics {
    /// `name="..."` + `value="..."` / key-attribute pairs inside `controller-info` (all `attr="value"` attributes on the root element, minus xmlns).
    pub attributes: usize,
    /// `<property name="x" value="y"/>` extension entries.
    pub properties: usize,
    /// `<application>`/`<tier>`/`<node>`/`<container>` element occurrences.
    pub containers: usize,
    /// Identity element names seen (`account-name`/`account-access-key`/`application-name`/`tier-name`/`node-name`/`controller-host`/`agent-type`/`machine-path`/`sim-enabled`/`orchestration-type`).
    pub identity_keys: usize,
    /// `<!-- -->` comment count.
    pub comments: usize,
}

const IDENTITY_ATTRS: &[&str] = &[
    "controller-host",
    "controller-port",
    "controller-ssl-enabled",
    "account-name",
    "account-access-key",
    "application-name",
    "tier-name",
    "node-name",
    "agent-type",
    "agent-runtime-dir",
    "machine-path",
    "sim-enabled",
    "orchestration-type",
    "season",
    "use-simple-hostname",
    "enable-orchestration",
    "unique-host-id",
    "docker-enabled",
];

/// Detects controller-info.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("<controller-info")
        || (t.contains("controller-host") && t.contains("<property "))
        || (t.contains("appdynamics") && t.contains('<'))
}

fn count_attrs(s: &str) -> usize {
    s.matches("=\"").count()
}

impl Appdynamics {
    /// Parse controller-info.xml; `None` when nothing counted.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let Ok(t) = core::str::from_utf8(b) else {
            return None;
        };
        let mut c = Self {
            attributes: 0,
            properties: 0,
            containers: 0,
            identity_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if s.contains("<controller-info") {
                c.containers += 1;
                c.attributes += count_attrs(s).saturating_sub(s.matches("xmlns").count());
            }
            if s.contains("<property ") && s.contains("name=\"") {
                c.properties += 1;
            }
            if s.contains("<application>")
                || s.contains("<tier>")
                || s.contains("<node>")
                || s.contains("<application-info")
                || s.contains("<machine-agent")
                || s.contains("<container")
            {
                c.containers += 1;
            }
            for a in IDENTITY_ATTRS {
                if s.contains(a) {
                    c.identity_keys += 1;
                }
            }
        }
        (c.attributes + c.properties + c.containers > 0).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_appd() {
        assert!(detect(b"<controller-info controller-host=\"h\"/>"));
        assert!(!detect(b"<root/>"));
    }

    #[test]
    fn counts() {
        let c = Appdynamics::parse(
            b"<?xml version=\"1.0\"?>\n<controller-info controller-host=\"h\" controller-port=\"443\" account-name=\"acme\" application-name=\"app\" tier-name=\"t\" node-name=\"n\">\n<property name=\"reuseNodeName\" value=\"true\"/>\n<application-info>\n<application/>\n</application-info>\n</controller-info>\n",
        )
        .unwrap();
        assert!(c.attributes >= 6);
        assert_eq!(c.properties, 1);
        assert!(c.containers >= 2);
        assert!(c.identity_keys >= 6);
    }

    #[test]
    fn rejects() {
        assert!(Appdynamics::parse(b"plain").is_none());
    }
}
