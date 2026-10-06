//! Rancher Fleet `fleet.yaml` GitOps bundle census.
//!
//! Fleet bundle top-level keys: `name`, `chart`, `repo`, `branch`,
//! `paths`, `helm`, `kustomize`, `targetNamespace`, `targets`,
//! `clusterSelector`, `rolloutStrategy`, `dependsOn`, `imageScans`,
//! `diff`, `helmReleaseName`, `clientID`, `namespaceLabels`,
//! `serviceAccount`, `resources`, `yaml`, `ignore`.
//!
//! ```rust
//! let k = b"name: demo\nrepo: https://github.com/x/charts\nchart: app\ntargets:\n- clusterSelector:\n    matchLabels:\n      env: prod\n";
//! assert!(izanagi_kit::fleet::detect(k));
//! ```

/// Fleet bundle census.
#[derive(Debug, Clone)]
pub struct Fleet {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "targetNamespace",
    "defaultNamespace",
    "targets",
    "clusterSelector",
    "clusterGroup",
    "clusterGroupSelector",
    "rolloutStrategy",
    "dependsOn",
    "imageScans",
    "diff",
    "helmReleaseName",
    "clientID",
    "namespaceLabels",
];

const WEAK: &[&str] = &[
    "name",
    "chart",
    "repo",
    "branch",
    "paths",
    "helm",
    "kustomize",
    "namespace",
    "yaml",
    "ignore",
    "serviceAccount",
    "resources",
    "repoName",
    "tag",
    "commit",
    "forceSyncGeneration",
    "keepResources",
    "deleteNamespace",
    "correctDrift",
    "paused",
    "privateRepoURL",
    "helmRepoURLRegex",
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

/// Detect a Fleet `fleet.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Fleet {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
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
    fn detects() {
        let b = b"name: demo\nrepo: https://github.com/x/charts\nchart: app\ntargets:\n- clusterSelector:\n    matchLabels:\n      env: prod\n";
        assert!(detect(b));
        let c = Fleet::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"name: demo\nchart: app\n"));
        assert!(!detect(b"targets:\n- x\n"));
        assert!(!detect(b"# targets:\n# rolloutStrategy: all\n"));
    }
}
