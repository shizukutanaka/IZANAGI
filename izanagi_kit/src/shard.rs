//! Crystal `shard.yml` — `name:`/`version:`/`crystal:` +
//! `targets:`/`executables:`/`dependencies:`/`development_dependencies:`.
//!
//! ```
//! let d = b"name: demo\nversion: 0\x2e1\x2e0\nauthors:\n  - Jane <j@example\x2ecom>\ncrystal: '>= 1\x2e0\x2e0'\ntargets:\n  demo:\n    main: src/demo\x2ecr\nexecutables:\n  - demo\ndependencies:\n  kemal:\n    github: kemalcr/kemal\ndevelopment_dependencies:\n  spec-kemal:\n    github: kemalcr/spec-kemal\nlicense: MIT\n";
//! let s = izanagi_kit::shard::parse(d).unwrap();
//! assert_eq!(s.name, "demo");
//! assert_eq!(s.targets, 2);
//! assert_eq!(s.dependencies, 2);
//! assert!(izanagi_kit::shard::detect(d));
//! ```

/// A parsed `shard.yml` summary.
#[derive(Debug, Clone)]
pub struct Shard {
    /// `name:` value.
    pub name: String,
    /// `version:` value.
    pub version: String,
    /// `crystal:` version constraint.
    pub crystal: String,
    /// `license:` value.
    pub license: String,
    /// Entries under `authors:`.
    pub authors: usize,
    /// Entries under `targets:`.
    pub targets: usize,
    /// Entries under `executables:`.
    pub executables: usize,
    /// Entries under `dependencies:`.
    pub dependencies: usize,
    /// Entries under `development_dependencies:`.
    pub dev_dependencies: usize,
    /// `documentation:`/`homepage:`/`repository:`/`description:` keys present.
    pub metadata_keys: usize,
    /// `libraries:` entries (C library bindings).
    pub libraries: usize,
}

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

fn has_key(t: &str, key: &str) -> bool {
    top_value(t, key).is_some() || t.lines().any(|l| l.trim_end() == key)
}

fn section_entries(t: &str, section: &str) -> usize {
    let mut n = 0;
    let mut in_sec = false;
    for line in t.lines() {
        if line.starts_with(char::is_whitespace) {
            let l = line.trim_start();
            if in_sec && !l.starts_with('#') && (l.contains(':') || l.starts_with('-')) {
                n += 1;
            }
            continue;
        }
        in_sec = line.trim_end() == section;
    }
    n
}

/// List items under a `key:` section written as `- item` lines.
fn dash_entries(t: &str, section: &str) -> usize {
    let mut n = 0;
    let mut in_sec = false;
    for line in t.lines() {
        if line.starts_with(char::is_whitespace) {
            if in_sec && line.trim_start().starts_with('-') {
                n += 1;
            }
            continue;
        }
        in_sec = line.trim_end() == section;
    }
    n
}

/// Detects shard.yml: `name:` plus a Crystal-specific key (`crystal:`/`targets:`/`development_dependencies:`/`executables:`).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    has_key(t, "name")
        && (has_key(t, "crystal")
            || has_key(t, "targets")
            || has_key(t, "development_dependencies")
            || has_key(t, "executables"))
}

/// Parses a `shard.yml`; `None` without `name:` + Crystal section markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Shard> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut metadata_keys = 0;
    for k in ["description", "documentation", "homepage", "repository"] {
        if has_key(t, k) {
            metadata_keys += 1;
        }
    }
    Some(Shard {
        name: top_value(t, "name").unwrap_or("").to_string(),
        version: top_value(t, "version").unwrap_or("").to_string(),
        crystal: top_value(t, "crystal").unwrap_or("").to_string(),
        license: top_value(t, "license").unwrap_or("").to_string(),
        authors: dash_entries(t, "authors:"),
        targets: section_entries(t, "targets:"),
        executables: dash_entries(t, "executables:"),
        dependencies: section_entries(t, "dependencies:"),
        dev_dependencies: section_entries(t, "development_dependencies:"),
        metadata_keys,
        libraries: section_entries(t, "libraries:"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"name: demo\nversion: 0\x2e1\x2e0\ndescription: demo shard\nauthors:\n  - Jane <j@example\x2ecom>\n  - John\ncrystal: '>= 1\x2e0\x2e0'\ntargets:\n  demo:\n    main: src/demo\x2ecr\nexecutables:\n  - demo\ndependencies:\n  kemal:\n    github: kemalcr/kemal\n    version: ~> 1\x2e0\ndevelopment_dependencies:\n  spec-kemal:\n    github: kemalcr/spec-kemal\nlicense: MIT\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.name, "demo");
        assert_eq!(s.version, "0\x2e1\x2e0");
        assert!(s.crystal.contains("1\x2e0"));
        assert_eq!(s.license, "MIT");
        assert_eq!(s.authors, 2);
        assert_eq!(s.targets, 2); // `demo:` + `main:` children
        assert_eq!(s.executables, 1);
        assert_eq!(s.dependencies, 3); // kemal + github + version children
        assert_eq!(s.dev_dependencies, 2);
        assert_eq!(s.metadata_keys, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"name: x\ncrystal: '>= 1\x2e0'\n"));
        assert!(!detect(b"name: x\nversion: 1\x2e0\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}
