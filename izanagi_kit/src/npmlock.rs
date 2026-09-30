//! npm `package-lock.json` — `{ "name", "version", "lockfileVersion",
//! "packages": { "node_modules/x": {...} } }` key census.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of `package-lock.json`.
pub struct Npmlock {
    /// `lockfileVersion` (1|2|3).
    pub lockfile_version: usize,
    /// Root `name`.
    pub name: String,
    /// Root `version`.
    pub version: String,
    /// `"node_modules/…"` package keys.
    pub packages: usize,
    /// `"version"` fields.
    pub versions: usize,
    /// `"resolved"` URLs.
    pub resolved: usize,
    /// `"integrity"` hashes.
    pub integrity: usize,
    /// `"dev": true` flags.
    pub dev: usize,
    /// `"dependencies"` objects.
    pub dependency_blocks: usize,
    /// `"engines"` objects.
    pub engines: usize,
    /// `"bin"` entries.
    pub bins: usize,
    /// `"peerDependencies"`/`"peer": true` markers.
    pub peers: usize,
    /// `"optional": true` flags.
    pub optional: usize,
    /// `"bundled"`/`"inBundle"` flags.
    pub bundled: usize,
    /// `"license"` fields.
    pub license: usize,
}

fn jstr<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let pat = ["\"", key, "\""].concat();
    let i = t.find(&pat)?;
    let r = &t[i + pat.len()..];
    let r = r.trim_start_matches(|c: char| c.is_whitespace() || c == ':');
    let r = r.strip_prefix('"')?;
    let end = r.find('"')?;
    Some(&r[..end])
}

fn jnum(t: &str, key: &str) -> Option<usize> {
    let pat = ["\"", key, "\""].concat();
    let i = t.find(&pat)?;
    t[i + pat.len()..]
        .trim_start_matches(|c: char| c.is_whitespace() || c == ':')
        .split(|c: char| c == ',' || c == '}' || c.is_whitespace())
        .next()
        .and_then(|v| v.parse().ok())
}

/// `true` when the text looks like package-lock.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    t.contains("\"lockfileVersion\"") && t.contains("\"packages\"")
}

impl Npmlock {
    #[must_use]
    /// Parses `b` into `Npmlock`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Self {
            lockfile_version: jnum(t, "lockfileVersion").unwrap_or(0),
            name: jstr(t, "name").unwrap_or("").to_string(),
            version: jstr(t, "version").unwrap_or("").to_string(),
            packages: t.matches("\"node_modules/").count(),
            versions: t.matches("\"version\"").count(),
            resolved: t.matches("\"resolved\"").count(),
            integrity: t.matches("\"integrity\"").count(),
            dev: t.matches("\"dev\": true").count(),
            dependency_blocks: t.matches("\"dependencies\"").count(),
            engines: t.matches("\"engines\"").count(),
            bins: t.matches("\"bin\"").count(),
            peers: t.matches("\"peerDependencies\"").count() + t.matches("\"peer\": true").count(),
            optional: t.matches("\"optional\": true").count(),
            bundled: t.matches("\"bundled\"").count() + t.matches("\"inBundle\"").count(),
            license: t.matches("\"license\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = br#"{
  "name": "app",
  "version": "1",
  "lockfileVersion": 3,
  "packages": {
    "": { "name": "app", "version": "1", "license": "MIT" },
    "node_modules/left-pad": { "version": "1", "resolved": "https://r/p.tgz", "integrity": "sha512-x", "dev": true },
    "node_modules/tiny": { "version": "2", "resolved": "https://r/t.tgz", "integrity": "sha512-y", "engines": {}, "bin": {} }
  }
}"#;

    #[test]
    fn detects_npmlock() {
        assert!(detect(FIX));
        assert!(!detect(b"{}"));
    }

    #[test]
    fn parses_npmlock() {
        let n = Npmlock::parse(FIX).unwrap();
        assert_eq!(n.lockfile_version, 3);
        assert_eq!(n.name, "app");
        assert_eq!(n.packages, 2);
        assert_eq!(n.versions, 4);
        assert_eq!(n.resolved, 2);
        assert_eq!(n.integrity, 2);
        assert_eq!(n.dev, 1);
        assert_eq!(n.engines, 1);
        assert_eq!(n.bins, 1);
        assert_eq!(n.license, 1);
        assert!(Npmlock::parse(b"").is_none());
    }
}
