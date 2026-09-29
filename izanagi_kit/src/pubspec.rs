//! Dart/Flutter `pubspec.yaml` — `name:`/`version:`/`environment:` +
//! `dependencies:`/`dev_dependencies:`/`flutter:` section census.
//!
//! ```
//! let d = b"name: demo_app\ndescription: A demo\nversion: 1\x2e0\x2e0\nenvironment:\n  sdk: '>=2\x2e19\x2e0 <4\x2e0\x2e0'\ndependencies:\n  http: ^1\x2e0\x2e0\n  collection: ^1\x2e18\x2e0\ndev_dependencies:\n  test: ^1\x2e24\x2e0\nflutter:\n  uses-material-design: true\n";
//! let p = izanagi_kit::pubspec::parse(d).unwrap();
//! assert_eq!(p.name, "demo_app");
//! assert_eq!(p.dependencies, 2);
//! assert!(p.flutter);
//! assert!(izanagi_kit::pubspec::detect(d));
//! ```

/// A parsed `pubspec.yaml` summary.
#[derive(Debug, Clone)]
pub struct Pubspec {
    /// `name:` value.
    pub name: String,
    /// `version:` value.
    pub version: String,
    /// `environment:` → `sdk:` constraint, e.g. `>=2.19.0 <4.0.0`.
    pub env_sdk: String,
    /// `environment:` → `flutter:` constraint.
    pub env_flutter: String,
    /// Entries under `dependencies:`.
    pub dependencies: usize,
    /// Entries under `dev_dependencies:`.
    pub dev_dependencies: usize,
    /// Entries under `dependency_overrides:`.
    pub overrides: usize,
    /// Entries under `executables:`.
    pub executables: usize,
    /// Top-level `flutter:` section present.
    pub flutter: bool,
    /// `publish_to:` value (`none` disables publishing).
    pub publish_to: String,
    /// Entries under `screenshots:`.
    pub screenshots: usize,
}

/// Top-level `key:` value (depth-0 lines only).
fn top_value<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for line in t.lines() {
        if line.starts_with(char::is_whitespace) || line.starts_with('#') {
            continue;
        }
        let l = line.trim_end();
        if let Some(rest) = l.strip_prefix(key) {
            let rest = rest.strip_prefix(':')?.trim();
            let v = rest.trim_matches('\'').trim_matches('"');
            return Some(v);
        }
    }
    None
}

/// True when a depth-0 `key:` line exists.
fn has_key(t: &str, key: &str) -> bool {
    top_value(t, key).is_some() || t.lines().any(|l| l.trim_end() == key)
}

/// Count direct children (`  child:`) of a depth-0 `section:` line —
/// only lines at the same indent as the section's first entry count
/// (grandchildren like `    sdk:` under `  flutter:` are excluded).
fn section_entries(t: &str, section: &str) -> usize {
    let mut n = 0;
    let mut in_sec = false;
    let mut child_indent = None;
    for line in t.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            if in_sec && !line.trim_start().starts_with('#') {
                let indent = line.len() - line.trim_start().len();
                if line.trim_start().contains(':') {
                    if child_indent.is_none() {
                        child_indent = Some(indent);
                    }
                    if child_indent == Some(indent) {
                        n += 1;
                    }
                }
            }
            continue;
        }
        in_sec = line.trim_end() == section;
        child_indent = None;
    }
    n
}

/// Value of a child key inside a section (`section:` → `  child: value`).
fn section_value<'a>(t: &'a str, section: &str, child: &str) -> Option<&'a str> {
    let mut in_sec = false;
    for line in t.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            if in_sec {
                let l = line.trim();
                if let Some(rest) = l.strip_prefix(child) {
                    if let Some(v) = rest.strip_prefix(':') {
                        return Some(v.trim().trim_matches('\'').trim_matches('"'));
                    }
                }
            }
            continue;
        }
        in_sec = line.trim_end() == section;
    }
    None
}

/// Detects a pubspec: `name:` plus `environment:`/`sdk:` or a `flutter:`/`dev_dependencies:` section.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    has_key(t, "name")
        && (section_value(t, "environment:", "sdk").is_some()
            || has_key(t, "flutter")
            || has_key(t, "dev_dependencies"))
}

/// Parses a `pubspec.yaml`; `None` without `name:` + environment/flutter markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Pubspec> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    Some(Pubspec {
        name: top_value(t, "name").unwrap_or("").to_string(),
        version: top_value(t, "version").unwrap_or("").to_string(),
        env_sdk: section_value(t, "environment:", "sdk")
            .unwrap_or("")
            .to_string(),
        env_flutter: section_value(t, "environment:", "flutter")
            .unwrap_or("")
            .to_string(),
        dependencies: section_entries(t, "dependencies:"),
        dev_dependencies: section_entries(t, "dev_dependencies:"),
        overrides: section_entries(t, "dependency_overrides:"),
        executables: section_entries(t, "executables:"),
        flutter: has_key(t, "flutter"),
        publish_to: top_value(t, "publish_to").unwrap_or("").to_string(),
        screenshots: section_entries(t, "screenshots:"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"name: demo_app\ndescription: A demo\nversion: 1\x2e0\x2e0\nhomepage: https://example\x2ecom\nenvironment:\n  sdk: '>=2\x2e19\x2e0 <4\x2e0\x2e0'\n  flutter: '>=3\x2e0\x2e0'\ndependencies:\n  http: ^1\x2e0\x2e0\n  collection: ^1\x2e18\x2e0\n  flutter:\n    sdk: flutter\ndev_dependencies:\n  test: ^1\x2e24\x2e0\ndependency_overrides:\n  intl: 0\x2e18\x2e0\nflutter:\n  uses-material-design: true\n";

    #[test]
    fn parses() {
        let p = parse(D).unwrap();
        assert_eq!(p.name, "demo_app");
        assert_eq!(p.version, "1\x2e0\x2e0");
        assert!(p.env_sdk.contains("2\x2e19"));
        assert_eq!(p.dependencies, 3);
        assert_eq!(p.dev_dependencies, 1);
        assert_eq!(p.overrides, 1);
        assert!(p.flutter);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"name: x\nenvironment:\n  sdk: '>=3\x2e0\x2e0'\n"));
        assert!(!detect(b"name: x\nversion: 1\x2e0\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}
