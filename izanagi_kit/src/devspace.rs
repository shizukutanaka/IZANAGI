//! DevSpace `devspace.yaml` census.
//!
//! `version:` + DevSpace top-level keys `pipelines`, `deployments`,
//! `dev`, `images`, `vars`, `profiles`, `hooks`, `commands`,
//! `pullSecrets`, `imports`, `localRegistry`, `dependencies`,
//! `require`, `merge`, `patches`.
//!
//! ```rust
//! let k = b"version: v2beta1\ndeployments:\n  app:\n    helm:\n      chart:\n        name: app\ndev:\n  app:\n    imageSelector: app\n";
//! assert!(izanagi_kit::devspace::detect(k));
//! ```

/// devspace.yaml census.
#[derive(Debug, Clone)]
pub struct Devspace {
    /// `deployments:` entries.
    pub deployments: usize,
    /// `dev:` entries.
    pub dev: usize,
    /// `pipelines:` stage entries.
    pub pipelines: usize,
    /// Recognised top-level keys present.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "pipelines",
    "deployments",
    "dev",
    "images",
    "vars",
    "profiles",
    "hooks",
    "commands",
    "pullSecrets",
    "imports",
    "localRegistry",
    "dependencies",
    "require",
    "merge",
    "patches",
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

/// Detect devspace.yaml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut has_version = false;
    let mut keys = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if k == "version" {
                has_version = true;
            } else if TOP_KEYS.contains(&k) {
                keys += 1;
            }
        }
    }
    has_version && keys >= 1 || keys >= 3
}

impl Devspace {
    /// Census a devspace.yaml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            deployments: 0,
            dev: 0,
            pipelines: 0,
            sections: 0,
            settings: 0,
            comments: 0,
        };
        let mut section = "";
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
                if TOP_KEYS.contains(&k) || k == "version" {
                    c.sections += 1;
                }
                c.settings += 1;
                continue;
            }
            if s.contains(':') && line.starts_with("  ") && !line.starts_with("   ") {
                match section {
                    "deployments" => c.deployments += 1,
                    "dev" => c.dev += 1,
                    "pipelines" => c.pipelines += 1,
                    _ => {}
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
    fn detects_devspace() {
        let b = b"version: v2beta1\ndeployments:\n  app:\n    helm:\n      chart:\n        name: app\n  api:\n    kubectl:\n      manifests: [api.yaml]\ndev:\n  app:\n    imageSelector: app\n";
        assert!(detect(b));
        let c = Devspace::parse(b).unwrap();
        assert_eq!(c.deployments, 2);
        assert_eq!(c.dev, 1);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"version: '3'\nservices:\n  web: {}\n"));
        assert!(!detect(b"deployments:\n  x: y\n"));
    }
}
