//! `composer.lock` (PHP) — `{ "packages": [...], "packages-dev": [...],
//! "platform": {...}, "plugin-api-version": "…", "content-hash": "…" }`.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of `composer.lock`.
pub struct Composerlock {
    /// `"packages"` entries (counted by `"name"` inside the section).
    pub packages: usize,
    /// `"packages-dev"` entries.
    pub packages_dev: usize,
    /// `"version"` fields.
    pub versions: usize,
    /// `"require"` maps.
    pub require: usize,
    /// `"require-dev"` maps.
    pub require_dev: usize,
    /// `"suggest"`/`"conflict"`/`"replace"` maps.
    pub relations: usize,
    /// `"source"` objects.
    pub sources: usize,
    /// `"type": "git"` sources.
    pub git_sources: usize,
    /// `"dist"` objects.
    pub dists: usize,
    /// `"autoload"` maps.
    pub autoload: usize,
    /// `"license"` arrays.
    pub license: usize,
    /// `"platform"`/`"platform-dev"` maps.
    pub platform: usize,
    /// `"plugin-api-version"`.
    pub plugin_api: String,
    /// `"content-hash"`.
    pub content_hash: usize,
    /// `"aliases"`/`"minimum-stability"` fields.
    pub meta: usize,
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

fn section_entries(t: &str, key: &str) -> usize {
    let pat = ["\"", key, "\""].concat();
    let Some(i) = t.find(&pat) else {
        return 0;
    };
    let rest = &t[i..];
    let end = rest[1..].find("\n  \"").map_or(rest.len(), |i| i + 1);
    rest[..end].matches("\"name\"").count()
}

/// `true` when the text looks like composer.lock.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    t.contains("\"packages\"")
        && (t.contains("\"platform\"")
            || t.contains("\"plugin-api-version\"")
            || t.contains("\"packages-dev\""))
}

impl Composerlock {
    #[must_use]
    /// Parses `b` into `Composerlock`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Self {
            packages: section_entries(t, "packages"),
            packages_dev: section_entries(t, "packages-dev"),
            versions: t.matches("\"version\"").count(),
            require: t.matches("\"require\"").count(),
            require_dev: t.matches("\"require-dev\"").count(),
            relations: t.matches("\"suggest\"").count()
                + t.matches("\"conflict\"").count()
                + t.matches("\"replace\"").count()
                + t.matches("\"provide\"").count(),
            sources: t.matches("\"source\"").count(),
            git_sources: t.matches("\"type\": \"git\"").count(),
            dists: t.matches("\"dist\"").count(),
            autoload: t.matches("\"autoload\"").count(),
            license: t.matches("\"license\"").count(),
            platform: t.matches("\"platform\"").count() + t.matches("\"platform-dev\"").count(),
            plugin_api: jstr(t, "plugin-api-version").unwrap_or("").to_string(),
            content_hash: t.matches("\"content-hash\"").count(),
            meta: t.matches("\"aliases\"").count()
                + t.matches("\"minimum-stability\"").count()
                + t.matches("\"prefer-stable\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = br#"{
  "packages": [
    { "name": "psr/log", "version": "3", "source": { "type": "git", "url": "https://x" }, "dist": {}, "require": { "php": ">=8" }, "autoload": {}, "license": ["MIT"] }
  ],
  "packages-dev": [
    { "name": "phpunit/phpunit", "version": "11", "require-dev": {}, "suggest": {} }
  ],
  "platform": { "php": ">=81" },
  "plugin-api-version": "26",
  "content-hash": "abc"
}"#;

    #[test]
    fn detects_composerlock() {
        assert!(detect(FIX));
        assert!(!detect(b"{}"));
    }

    #[test]
    fn parses_composerlock() {
        let c = Composerlock::parse(FIX).unwrap();
        assert_eq!(c.packages, 1);
        assert_eq!(c.packages_dev, 1);
        assert_eq!(c.sources, 1);
        assert_eq!(c.git_sources, 1);
        assert_eq!(c.dists, 1);
        assert_eq!(c.autoload, 1);
        assert_eq!(c.license, 1);
        assert_eq!(c.platform, 1);
        assert_eq!(c.plugin_api, "26");
        assert_eq!(c.content_hash, 1);
        assert!(Composerlock::parse(b"").is_none());
    }
}
