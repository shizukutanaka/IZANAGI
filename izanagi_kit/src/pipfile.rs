//! `Pipfile` (TOML) census.
//!
//! `[[source]]` blocks (`url`/`verify_ssl`/`name`), `[packages]` +
//! `[dev-packages]` spec tables (`name = "==1.0"`/`"*"`/
//! `{version = ">=1", extras = ["x"]}`/`{ref = "…"}`/`{git = "…"}`/
//! `{file = "…"}`/`{path = "…"}`/`{editable = true}`), `[requires]`
//! (`python_version`/`python_full_version`), `[pipenv]` (`allow_prereleases`/
//! `keep_outdated`/`install_search_all_sources`).
//!
//! ```rust
//! let p = "[[source]]\nurl = \"https://pypi.org/simple\"\n[packages]\nrequests = \"==2\"\n[dev-packages]\npytest = \"*\"\n[requires]\npython_version = \"3\"\n";
//! let c = izanagi_kit::pipfile::Pipfile::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.specs, 2);
//! ```

/// Pipfile census.
#[derive(Debug, Clone)]
pub struct Pipfile {
    /// `[[source]]` blocks.
    pub sources: usize,
    /// `[packages]`/`[dev-packages]`/`[requires]`/`[pipenv]`/`[scripts]` sections.
    pub sections: usize,
    /// `name = "…"` spec lines inside packages/dev-packages.
    pub specs: usize,
    /// Inline-table specs (`{version = …}`/`{ref = …}`/`{git = …}`).
    pub tables: usize,
    /// Recognised key names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "url",
    "verify_ssl",
    "name",
    "packages",
    "dev-packages",
    "requires",
    "pipenv",
    "scripts",
    "source",
    "python_version",
    "python_full_version",
    "allow_prereleases",
    "keep_outdated",
    "install_search_all_sources",
    "version",
    "ref",
    "git",
    "file",
    "path",
    "editable",
    "extras",
    "markers",
    "index",
    "hashes",
    "branch",
    "tag",
    "subdirectory",
    "skip_lock",
];

/// Whether the buffer looks like a Pipfile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[packages]")
        || t.contains("[dev-packages]")
        || t.contains("[[source]]")
        || t.contains("[requires]")
        || t.contains("[pipenv]")
}

impl Pipfile {
    /// Parse a Pipfile into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sources: 0,
            sections: 0,
            specs: 0,
            tables: 0,
            named: 0,
        };
        let mut ctx = "";
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s == "[[source]]" {
                c.sources += 1;
                ctx = "source";
                continue;
            }
            if s.starts_with('[') {
                c.sections += 1;
                ctx = s.trim_matches(['[', ']']);
                if KEYS.contains(&ctx) {
                    c.named += 1;
                }
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            let key = s[..eq].trim().trim_matches('"');
            if KEYS.contains(&key) {
                c.named += 1;
            }
            if matches!(ctx, "packages" | "dev-packages") {
                c.specs += 1;
                if s.contains('{') {
                    c.tables += 1;
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
    fn parses_pipfile() {
        let b = concat!(
            "[[source]]\n",
            "url = \"https://pypi.org/simple\"\n",
            "verify_ssl = true\n",
            "name = \"pypi\"\n",
            "[[source]]\n",
            "url = \"https://private/simple\"\n",
            "verify_ssl = false\n",
            "name = \"internal\"\n",
            "[packages]\n",
            "requests = \"==2.31\"\n",
            "flask = \">=3\"\n",
            "mypkg = {git = \"https://github.com/x/y\", ref = \"main\"}\n",
            "local = {path = \"./lib\", editable = true}\n",
            "extras = {version = \"*\", extras = [\"all\"]}\n",
            "[dev-packages]\n",
            "pytest = \"*\"\n",
            "black = \"==24\"\n",
            "[requires]\n",
            "python_version = \"3.12\"\n",
            "[pipenv]\n",
            "allow_prereleases = true\n",
        );
        let c = Pipfile::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sources, 2);
        assert_eq!(c.sections, 4);
        assert_eq!(c.specs, 7);
        assert_eq!(c.tables, 3);
        assert!(c.named >= 10);
    }

    #[test]
    fn rejects_other() {
        assert!(Pipfile::parse(b"[foo]\na = 1").is_none());
    }
}
