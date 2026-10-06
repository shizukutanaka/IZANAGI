//! Carvel suite (`kapp`/`kapp-controller`) manifest census.
//!
//! Detects `apiVersion` under the `*.k14s.io` or `packaging.carvel.dev`
//! API groups together with a known `kind`:
//! `Config` (kapp.k14s.io), `App`, `PackageInstall`,
//! `PackageRepository`, `Package`, `PackageMetadata`.
//!
//! ```rust
//! let k = b"apiVersion: kappctrl.k14s.io/v1alpha1\nkind: App\nmetadata:\n  name: demo\nspec:\n  syncPeriod: 5m\n  fetch:\n  - git:\n      url: https://example.com/repo.git\n";
//! assert!(izanagi_kit::carvel::detect(k));
//! ```

/// Carvel manifest census.
#[derive(Debug, Clone)]
pub struct Carvel {
    /// `apiVersion`/`kind`/`metadata`/`spec` envelope lines.
    pub envelope: usize,
    /// `kind` value, when present.
    pub kind: Option<String>,
    /// `- ` list items (fetch/template/deploy steps).
    pub items: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const GROUPS: &[&str] = &[
    "kapp.k14s.io",
    "kappctrl.k14s.io",
    "packaging.carvel.dev",
    "data.packaging.carvel.dev",
    "install.package.carvel.dev",
    "kbld.k14s.io",
    "imgpkg.carvel.dev",
];

const KINDS: &[&str] = &[
    "Config",
    "App",
    "PackageInstall",
    "PackageRepository",
    "Package",
    "PackageMetadata",
    "Bundle",
    "Image",
    "ImagesConfig",
];

fn key_of(s: &str) -> &str {
    match s.find(':') {
        Some(i) => s[..i].trim(),
        None => s.trim(),
    }
}

fn value_of(s: &str) -> &str {
    match s.find(':') {
        Some(i) => s[i + 1..].trim().trim_matches('"').trim_matches('\''),
        None => "",
    }
}

fn group_ok(t: &str) -> bool {
    t.lines().any(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("apiVersion") {
            return false;
        }
        let v = value_of(s);
        GROUPS.iter().any(|g| v.starts_with(g))
    })
}

fn kind_val(t: &str) -> Option<String> {
    t.lines().find_map(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("kind") {
            return None;
        }
        let v = value_of(s);
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    })
}

/// Detect a Carvel-suite manifest.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if !group_ok(t) {
        return false;
    }
    match kind_val(t) {
        Some(k) => KINDS.contains(&k.as_str()),
        None => false,
    }
}

impl Carvel {
    /// Census a Carvel manifest buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            envelope: 0,
            kind: kind_val(t),
            items: 0,
            settings: 0,
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
            if s.starts_with("- ") || (s.starts_with('-') && s.len() > 1) {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                let k = key_of(s);
                if matches!(k, "apiVersion" | "kind" | "metadata" | "spec") {
                    c.envelope += 1;
                }
                if !k.is_empty() {
                    c.settings += 1;
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
    fn detects_app() {
        let b = b"apiVersion: kappctrl.k14s.io/v1alpha1\nkind: App\nmetadata:\n  name: d\nspec:\n  fetch:\n  - git:\n      url: u\n";
        assert!(detect(b));
        let c = Carvel::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("App"));
        assert_eq!(c.items, 1);
    }

    #[test]
    fn detects_kapp_config() {
        let b = b"apiVersion: kapp.k14s.io/v1alpha1\nkind: Config\nrebaseRules:\n- paths: [a]\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_kinds() {
        assert!(!detect(
            b"apiVersion: kappctrl.k14s.io/v1alpha1\nkind: Deployment\n"
        ));
        assert!(!detect(b"apiVersion: apps/v1\nkind: App\n"));
        // Comment-only mention must not detect.
        assert!(!detect(b"# apiVersion: kapp.k14s.io/v1alpha1\nkind: App\n"));
    }
}
