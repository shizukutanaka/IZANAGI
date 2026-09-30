//! Census of a container `registries.conf` file.
//!
//! TOML registries configuration: `unqualified-search-registries`,
//! `[[registry]]` array entries with `prefix`/`location`/`insecure`/
//! `blocked`, plus nested `[[registry.mirror]]` blocks. Also accepts
//! the newer `registries.yaml` v2 shape (`prefix:`/`mirrors:`/`location:`).
//! Counts registries, mirrors, option keys and comments.
//!
//! ```rust
//! let c = izanagi_kit::registriesconf::RegistriesConf::parse(
//!     b"unqualified-search-registries = [\"docker.io\"]\n\
//!       [[registry]]\nprefix = \"quay.io\"\nlocation = \"quay.io\"\n",
//! ).unwrap();
//! assert_eq!(c.registries, 1);
//! ```
#![forbid(unsafe_code)]

/// registries.conf census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistriesConf {
    /// `[[registry]]` blocks.
    pub registries: usize,
    /// `[[registry.mirror]]`/`[[mirror]]` blocks.
    pub mirrors: usize,
    /// `key = value` / `key: value` option lines.
    pub options: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Option keys recognised for `detect`.
const KEYS: &[&str] = &[
    "unqualified-search-registries",
    "short-name-mode",
    "prefix",
    "location",
    "insecure",
    "blocked",
    "mirror-by-digest-only",
];

/// True if `b` looks like registries.conf/registries.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("unqualified-search-registries")
        || t.contains("[[registry]]")
        || t.contains("mirror-by-digest-only")
        || t.contains("short-name-mode"))
        || (t.contains("mirrors:")
            && t.contains("location:")
            && KEYS.iter().filter(|k| t.contains(**k)).count() >= 2)
}

impl RegistriesConf {
    /// Parse a registries config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut registries = 0usize;
        let mut mirrors = 0usize;
        let mut options = 0usize;
        let mut comments = 0usize;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                comments += 1;
            } else if l.starts_with("[[registry.mirror]]") || l.starts_with("[[mirror]]") {
                mirrors += 1;
            } else if l.starts_with("[[registry]]") {
                registries += 1;
            } else if l.contains('=') || l.contains(':') {
                options += 1;
            }
        }
        if options == 0 {
            return None;
        }
        Some(Self {
            registries,
            mirrors,
            options,
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# registries\n",
            "unqualified-search-registries = [\"docker.io\"]\n",
            "short-name-mode = \"enforcing\"\n",
            "[[registry]]\n",
            "prefix = \"quay.io\"\n",
            "location = \"quay.io\"\n",
            "insecure = false\n",
            "[[registry.mirror]]\n",
            "location = \"mirror.local\"\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = RegistriesConf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.registries, 1);
        assert_eq!(c.mirrors, 1);
        assert_eq!(c.options, 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(RegistriesConf::parse(b"# only\n").is_none());
    }
}
