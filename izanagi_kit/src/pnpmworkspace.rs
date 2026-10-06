//! `pnpm-workspace.yaml` census.
//!
//! Top-level `packages:` list of glob paths plus optional pnpm
//! workspace keys: `catalog`, `catalogs`, `overrides`,
//! `patchedDependencies`, `onlyBuiltDependencies`,
//! `ignoredBuiltDependencies`, `updateConfig`, `lockfileVersion`.
//!
//! ```rust
//! let k = b"packages:\n  - 'packages/*'\n  - '!**/test/**'\ncatalog:\n  react: ^18.0.0\n";
//! assert!(izanagi_kit::pnpmworkspace::detect(k));
//! ```

/// pnpm-workspace.yaml census.
#[derive(Debug, Clone)]
pub struct PnpmWorkspace {
    /// `packages:` glob entries.
    pub packages: usize,
    /// `catalog`/`catalogs` dependency pin lines.
    pub catalog_entries: usize,
    /// Other recognised top-level keys present.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const EXTRA_KEYS: &[&str] = &[
    "catalog",
    "catalogs",
    "overrides",
    "patchedDependencies",
    "onlyBuiltDependencies",
    "ignoredBuiltDependencies",
    "updateConfig",
    "lockfileVersion",
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

fn glob_like(s: &str) -> bool {
    let v = s
        .trim_start_matches('-')
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    v.contains('*') || v.contains('/') || v.starts_with('!')
}

/// Detect pnpm-workspace.yaml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut in_packages = false;
    let mut has_packages = false;
    let mut glob_items = 0usize;
    let mut extras = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            in_packages = k == "packages";
            if in_packages {
                has_packages = true;
            } else if EXTRA_KEYS.contains(&k) {
                extras += 1;
            }
            continue;
        }
        if in_packages && glob_like(line.trim()) {
            glob_items += 1;
        }
    }
    // `packages:` with glob entries, or `packages:` plus a pnpm-only key.
    (has_packages && glob_items >= 1) || (has_packages && extras >= 1)
}

impl PnpmWorkspace {
    /// Census a pnpm-workspace.yaml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            packages: 0,
            catalog_entries: 0,
            sections: 0,
            settings: 0,
            comments: 0,
        };
        let mut in_packages = false;
        let mut in_catalog = false;
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
                in_packages = k == "packages";
                in_catalog = k == "catalog" || k == "catalogs";
                if k == "packages" || EXTRA_KEYS.contains(&k) {
                    c.sections += 1;
                }
                c.settings += 1;
                continue;
            }
            if s.starts_with('-') && in_packages && glob_like(s) {
                c.packages += 1;
                continue;
            }
            if in_catalog && s.contains(':') {
                c.catalog_entries += 1;
            }
            if s.contains(':') {
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
    fn detects_workspace() {
        let b = b"packages:\n  - 'packages/*'\n  - 'apps/*'\n  - '!**/test/**'\ncatalog:\n  react: ^18.0.0\n";
        assert!(detect(b));
        let c = PnpmWorkspace::parse(b).unwrap();
        assert_eq!(c.packages, 3);
        assert_eq!(c.catalog_entries, 1);
    }

    #[test]
    fn detects_with_catalog_only() {
        let b = b"packages:\ncatalogs:\n  react18:\n    react: ^18.0.0\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_yaml() {
        // A docker-compose-ish file has no packages key.
        assert!(!detect(b"version: '3'\nservices:\n  web:\n    image: x\n"));
        // `packages:` without globs or pnpm keys is not a workspace file.
        assert!(!detect(b"packages:\n  - foo\n"));
    }
}
