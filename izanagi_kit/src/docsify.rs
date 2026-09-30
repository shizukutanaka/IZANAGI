//! Docsify `index.html`/`window.$docsify` config census.
//!
//! Docsify config lives inside `window.$docsify = { … }` (or
//! `window.$docsify=`) in `index.html`: `name`, `repo`,
//! `el`, `themeColor`, `homepage`, `basePath`, `coverpage`,
//! `loadNavbar`, `loadSidebar`, `subMaxLevel`, `maxLevel`,
//! `auto2top`, `relativePath`, `executeScript`, `noEmoji`,
//! `mergeNavbar`, `formatUpdated`, `externalLinkTarget`,
//! `cornerExternalLinkTarget`, `routerMode`, `search`,
//! `alias`, `plugins`, `fallbackLanguages`, `notFoundPage`,
//! `onlyCover`, `requestHeaders`, `ext`, `markdown`, `vue`,
//! `nameLink`, `hideSidebar`, `catchPluginErrors`,
//! `crossOriginLinks`, `themeable` (`readyPlugins`, `toolbar`,
//! `responsiveTables`), `topMargin`, `logo`, `copyCode`.
//!
//! ```rust
//! let c = izanagi_kit::docsify::Docsify::parse(
//!     b"window.$docsify = {\n  name: 'd',\n  repo: 'x/y',\n  loadSidebar: true,\n  search: 'auto',\n};\n").unwrap();
//! assert_eq!(c.keys, 5);
//! ```

/// `window.$docsify` config census.
#[derive(Debug, Clone)]
pub struct Docsify {
    /// `key:`/`key =` entries at any depth.
    pub keys: usize,
    /// Docsify-known keys.
    pub known: usize,
    /// `plugins:`/`alias:`/`search:` block entries.
    pub plugin_entries: usize,
    /// `//`/`<!--` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "name",
    "repo",
    "el",
    "themecolor",
    "homepage",
    "basepath",
    "coverpage",
    "loadnavbar",
    "loadsidebar",
    "submaxlevel",
    "maxlevel",
    "auto2top",
    "relativepath",
    "executescript",
    "noemoji",
    "mergenavbar",
    "formatupdated",
    "externallinktarget",
    "cornerexternallinktarget",
    "routermode",
    "search",
    "alias",
    "plugins",
    "fallbacklanguages",
    "notfoundpage",
    "onlycover",
    "requestheaders",
    "ext",
    "markdown",
    "vue",
    "namelink",
    "hidesidebar",
    "catchpluginerrors",
    "crossoriginlinks",
    "themeable",
    "readyplugins",
    "toolbar",
    "responsivetables",
    "topmargin",
    "logo",
    "copycode",
];

/// Whether the buffer contains a `window.$docsify` block.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("$docsify")
}

impl Docsify {
    /// Parse a Docsify config block into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            known: 0,
            plugin_entries: 0,
            comments: 0,
        };
        let mut depth = 0usize;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with("<!--") || s.starts_with('*') {
                c.comments += 1;
                continue;
            }
            if !(s.contains(':') || s.contains('=')) {
                continue;
            }
            c.keys += 1;
            let head = s
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_ascii_lowercase();
            if KEYS.contains(&head.as_str()) {
                c.known += 1;
            }
            if depth > 0 && s.contains(':') {
                c.plugin_entries += 1;
            }
            if (head == "plugins" || head == "alias" || head == "search") && s.contains('{') {
                depth += 1;
            }
            depth = depth.saturating_sub(s.matches('}').count());
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_docsify() {
        let b = concat!(
            "// docsify\n",
            "window.$docsify = {\n",
            "  name: 'Docs',\n",
            "  repo: 'x/y',\n",
            "  loadSidebar: true,\n",
            "  subMaxLevel: 2,\n",
            "  auto2top: true,\n",
            "  homepage: 'README.md',\n",
            "  search: 'auto',\n",
            "  coverpage: true,\n",
            "  themeColor: '#42b983',\n",
            "};\n",
        );
        let c = Docsify::parse(b.as_bytes()).unwrap();
        assert_eq!(c.known, 9);
        assert_eq!(c.keys, 10);
        assert_eq!(c.plugin_entries, 0);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Docsify::parse(b"window.x = 1;\n").is_none());
    }
}
