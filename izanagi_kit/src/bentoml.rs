//! BentoML service/bento file parser (`bentofile.yaml`, `bento.yaml`).
//!
//! Detects `service:` + `include:`/`runners:`/`creation_time:`/`bentoml`
//! build manifests and counts services, includes/excludes, models,
//! runners, APIs, labels and env entries.
//!
//! ```
//! let b = concat!(
//!     "service: \"svc.py:svc\"\n",
//!     "include:\n",
//!     "  - \"*.py\"\n",
//!     "exclude:\n",
//!     "  - \"tests\"\n",
//!     "python:\n",
//!     "  packages:\n",
//!     "    - numpy\n",
//!     "docker:\n",
//!     "  distro: debian\n"
//! ).as_bytes();
//! assert!(izanagi_kit::bentoml::detect(b));
//! let c = izanagi_kit::bentoml::Bentoml::parse(b).unwrap();
//! assert_eq!(c.services, 1);
//! assert_eq!(c.includes, 1);
//! ```

/// Parsed BentoML manifest summary.
#[derive(Debug, Clone)]
pub struct Bentoml {
    /// `service:` entries.
    pub services: usize,
    /// `include:` list items.
    pub includes: usize,
    /// `exclude:` list items.
    pub excludes: usize,
    /// `models:` list items.
    pub models: usize,
    /// `runners:` entries.
    pub runners: usize,
    /// `apis:`/`endpoints:` entries.
    pub apis: usize,
    /// `labels:`/`envs:` entries.
    pub labels_envs: usize,
    /// `python:`/`docker:`/`conda:` section keys.
    pub sections: usize,
}

/// Whether the buffer looks like a BentoML manifest.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("bentoml")
        || (t.contains("service:") && (t.contains("runners:") || t.contains("creation_time:")))
        || (t.contains("service:") && t.contains("include:") && t.contains("exclude:"))
}

impl Bentoml {
    /// Parses a BentoML manifest summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            services: 0,
            includes: 0,
            excludes: 0,
            models: 0,
            runners: 0,
            apis: 0,
            labels_envs: 0,
            sections: 0,
        };
        const LIST_KEYS: &[(&str, u8)] = &[
            ("include:", b'i'),
            ("exclude:", b'x'),
            ("models:", b'm'),
            ("runners:", b'r'),
            ("apis:", b'a'),
            ("labels:", b'l'),
            ("envs:", b'e'),
        ];
        let mut list: Option<u8> = None;
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("service:") {
                c.services += 1;
                list = None;
                continue;
            }
            if tr.starts_with("- ") {
                match list {
                    Some(b'i') => c.includes += 1,
                    Some(b'x') => c.excludes += 1,
                    Some(b'm') => c.models += 1,
                    Some(b'r') => c.runners += 1,
                    Some(b'a') => c.apis += 1,
                    Some(b'l' | b'e') => c.labels_envs += 1,
                    _ => {}
                }
                continue;
            }
            if tr.starts_with("python:") || tr.starts_with("docker:") || tr.starts_with("conda:") {
                c.sections += 1;
            }
            list = LIST_KEYS
                .iter()
                .find(|(k, _)| *k == tr)
                .map(|(_, tag)| *tag);
            if list.is_none() && !tr.is_empty() && !tr.ends_with(':') && !tr.starts_with('#') {
                list = None;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "service: \"svc.py:svc\"\n",
            "include:\n",
            "  - \"*.py\"\n",
            "  - \"data/\"\n",
            "exclude:\n",
            "  - \"tests/\"\n",
            "models:\n",
            "  - model:latest\n",
            "runners:\n",
            "  - runner1\n",
            "apis:\n",
            "  - predict\n",
            "labels:\n",
            "  - team: ml\n",
            "python:\n",
            "  packages:\n",
            "    - numpy\n",
            "docker:\n",
            "  distro: debian\n",
            "conda: {}\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Bentoml::parse(b).unwrap();
        assert_eq!(c.services, 1);
        assert_eq!(c.includes, 2);
        assert_eq!(c.excludes, 1);
        assert_eq!(c.models, 1);
        assert_eq!(c.runners, 1);
        assert_eq!(c.apis, 1);
        assert_eq!(c.labels_envs, 1);
        assert_eq!(c.sections, 3);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"service: x\n"));
        assert!(Bentoml::parse(b"x").is_none());
    }
}
