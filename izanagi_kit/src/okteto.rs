//! Okteto `okteto.yaml` manifest census.
//!
//! Okteto manifests declare `deploy:`/`destroy:`/`test:`/`build:`/
//! `dependencies:`/`manifests:`/`context:`/`namespace:`/`icon:`/
//! `external:` sections, plus `dev:` containers with `sync:`/
//! `forward:`/`remote:`/`autocreate:`/`imagePullPolicy`/
//! `persistentVolume:` sub-keys.
//!
//! ```rust
//! let k = b"deploy:\n  - helm upgrade --install app chart\ndev:\n  api:\n    image: api:dev\n    sync:\n      - .:/src\n    forward:\n      - 8080:8080\n";
//! assert!(izanagi_kit::okteto::detect(k));
//! ```

/// okteto.yaml census.
#[derive(Debug, Clone)]
pub struct Okteto {
    /// `dev:` container entries.
    pub devs: usize,
    /// `sync:`/`forward:`/`volumes:`/`persistentVolume` entries.
    pub mounts: usize,
    /// `deploy:`/`destroy:`/`test:` command items.
    pub commands: usize,
    /// Recognised top-level keys present.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "deploy",
    "destroy",
    "test",
    "build",
    "dev",
    "dependencies",
    "manifests",
    "external",
    "context",
    "namespace",
    "icon",
    "variables",
];

const DEV_HINTS: &[&str] = &[
    "sync",
    "forward",
    "remote",
    "autocreate",
    "imagePullPolicy",
    "persistentVolume",
    "reverse",
    "services",
    "replicas",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect okteto.yaml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut tops = 0usize;
    let mut dev_hints = 0usize;
    let mut in_dev = false;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            in_dev = k == "dev";
            if TOP_KEYS.contains(&k) {
                tops += 1;
            }
            continue;
        }
        if in_dev {
            let s = line.trim();
            if s.starts_with('#') || s.starts_with('-') {
                continue;
            }
            if let Some(i) = s.find(':') {
                if DEV_HINTS.contains(&s[..i].trim()) {
                    dev_hints += 1;
                }
            }
        }
    }
    // `dev:` plus an okteto-only sub-key, or several okteto top keys.
    (in_dev || tops >= 1) && dev_hints >= 1 || tops >= 3
}

impl Okteto {
    /// Census an okteto.yaml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            devs: 0,
            mounts: 0,
            commands: 0,
            sections: 0,
            settings: 0,
            comments: 0,
        };
        let mut section = "";
        let mut dev_child = "";
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = top_key(line) {
                section = k;
                dev_child = "";
                if TOP_KEYS.contains(&k) {
                    c.sections += 1;
                }
                c.settings += 1;
                continue;
            }
            if s.starts_with("- ") {
                match section {
                    "deploy" | "destroy" | "test" | "manifests" => c.commands += 1,
                    _ => {
                        if DEV_HINTS.contains(&dev_child) || dev_child == "forward" {
                            c.mounts += 1;
                        }
                    }
                }
                continue;
            }
            if s.contains(':') {
                let k = s[..s.find(':').unwrap_or(s.len())].trim();
                if section == "dev" && line.starts_with("  ") && !line.starts_with("   ") {
                    c.devs += 1;
                }
                if DEV_HINTS.contains(&k) {
                    dev_child = k;
                    if k == "sync" || k == "forward" || k == "volumes" || k == "persistentVolume" {
                        c.mounts += 1;
                    }
                }
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_manifest() {
        let b = b"deploy:\n  - helm upgrade --install app chart\ndev:\n  api:\n    image: api:dev\n    sync:\n      - .:/src\n    forward:\n      - 8080:8080\n";
        assert!(detect(b));
        let c = Okteto::parse(b).unwrap();
        assert_eq!(c.devs, 1);
        assert_eq!(c.commands, 1);
    }

    #[test]
    fn detects_dev_only() {
        let b = b"dev:\n  api:\n    sync:\n      - .:/src\n    autocreate: true\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"version: '3'\nservices:\n  web: {}\n"));
        assert!(!detect(b"dev: true\nkey: v\n"));
    }
}
