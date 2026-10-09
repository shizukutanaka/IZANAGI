//! `vcpkg.json` manifest format.
//!
//! vcpkg manifests carry `"name"`, one of `"version"`/
//! `"version-semver"`/`"version-date"`/`"version-string"`,
//! `"dependencies"` (bare strings or `{"name":…,"features":[]}`),
//! `"features"`, `"default-features"`, `"overrides"`,
//! `"builtin-baseline"`, `"supports"`, `"platform"`, `"license"`,
//! `"homepage"`, `"description"`, `"maintainers"`, `"documentation"`.
//!
//! ```
//! let b = br#"{
//!   "name": "proj",
//!   "version": "1",
//!   "dependencies": [
//!     "boost",
//!     { "name": "openssl", "features": ["tools"] }
//!   ],
//!   "overrides": [ { "name": "openssl", "version": "3" } ],
//!   "builtin-baseline": "abc",
//!   "supports": "windows"
//! }"#;
//! assert!(izanagi_kit::vcpkg::detect(b));
//! let c = izanagi_kit::vcpkg::Vcpkg::parse(b).unwrap();
//! assert_eq!(c.dependencies, 2);
//! assert_eq!(c.overrides, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed vcpkg.json summary.
#[derive(Debug, Clone)]
pub struct Vcpkg {
    /// `dependencies` entries (string or object).
    pub dependencies: usize,
    /// Object-form dependencies (with features/platform).
    pub objects: usize,
    /// `features`/`default-features` entries.
    pub features: usize,
    /// `overrides` entries.
    pub overrides: usize,
    /// Version-ish keys (`version`/`version-semver`/`version-date`/`version-string`/`port-version`/`builtin-baseline`).
    pub versions: usize,
    /// Metadata keys (`homepage`/`description`/`license`/`maintainers`/`documentation`/`supports`/`platform`).
    pub metadata: usize,
}

fn array_entries(t: &str, key: &str) -> (usize, usize) {
    // Elements of `"key": [ ... ]`: objects (`{` at depth 1) plus
    // bare strings (`"` pairs at depth 1). Nested arrays/objects are
    // skipped via depth tracking; the pair ends at the matching `]`.
    let Some(pos) = t.find(&format!("\"{key}\"")) else {
        return (0, 0);
    };
    let Some(open) = t[pos..].find('[').map(|o| pos + o) else {
        return (0, 0);
    };
    let mut depth = 0usize;
    let mut objs = 0usize;
    let mut in_str = false;
    let mut strings = 0usize;
    for ch in t[open..].chars() {
        if in_str {
            if ch == '"' {
                in_str = false;
                if depth == 1 {
                    strings += 1;
                }
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '[' | '{' => {
                depth += 1;
                if ch == '{' && depth == 2 {
                    objs += 1;
                }
            }
            ']' | '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && ch == ']' {
                    break;
                }
            }
            _ => {}
        }
    }
    (objs + strings, objs)
}

/// Whether the buffer looks like vcpkg.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.trim_start().starts_with('{')
        && t.contains("\"name\"")
        && (t.contains("\"dependencies\"")
            || t.contains("\"builtin-baseline\"")
            || t.contains("\"overrides\"")
            || t.contains("\"version\""))
}

impl Vcpkg {
    /// Parses a vcpkg.json summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let (deps, objs) = array_entries(t, "dependencies");
        let (overrides, _) = array_entries(t, "overrides");
        let (features, _) = array_entries(t, "features");
        let (def_features, _) = array_entries(t, "default-features");
        Some(Self {
            dependencies: deps.max(objs),
            objects: objs,
            features: features + def_features,
            overrides,
            versions: [
                "\"version\"",
                "\"version-semver\"",
                "\"version-date\"",
                "\"version-string\"",
                "\"port-version\"",
                "\"builtin-baseline\"",
            ]
            .iter()
            .map(|k| t.matches(k).count())
            .sum(),
            metadata: [
                "\"homepage\"",
                "\"description\"",
                "\"license\"",
                "\"maintainers\"",
                "\"documentation\"",
                "\"supports\"",
                "\"platform\"",
            ]
            .iter()
            .map(|k| t.matches(k).count())
            .sum(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vcpkg() {
        let b = br#"{
  "name": "proj",
  "version": "1",
  "description": "d",
  "license": "MIT",
  "supports": "windows",
  "dependencies": [
    "boost",
    "fmt",
    { "name": "openssl", "features": ["tools"] }
  ],
  "overrides": [
    { "name": "openssl", "version": "3" }
  ],
  "features": { "x": {} },
  "builtin-baseline": "abc",
  "homepage": "h"
}"#;
        assert!(detect(b));
        let c = Vcpkg::parse(b).unwrap();
        assert_eq!(c.dependencies, 3);
        assert_eq!(c.objects, 1);
        assert_eq!(c.overrides, 1);
        assert_eq!(c.versions, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{}"));
        assert!(Vcpkg::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
