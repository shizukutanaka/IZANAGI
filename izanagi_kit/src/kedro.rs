//! Census of a Kedro `catalog.yml`/`settings.yml`/`parameters.yml` file.
//!
//! `catalog.yml` entries: `<name>:` blocks with `type:`/`filepath:`/
//! `dataset:`/`save_args:`/`load_args:`/`credentials:`/`fs_args:`/
//! `versioned:`/`layer:`/`metadata:`. `settings.yml` keys:
//! `HOOKS`/`SESSION_STORE_ARGS`/`CONFIG_LOADER_ARGS`/`DATA_CATALOG`/
//! `CONFIG_LOADER_CLASS`/`CONTEXT_CLASS`/`DYNAMIC_PIPELINES`. Counts
//! catalog entries, typed datasets, filepaths and comments.
//!
//! ```rust
//! let c = izanagi_kit::kedro::Kedro::parse(
//!     b"companies:\n  type: pandas.CSVDataset\n  filepath: data/01_raw/companies.csv\n\
//!       reviews:\n  type: pandas.CSVDataset\n  filepath: data/01_raw/reviews.csv\n",
//! ).unwrap();
//! assert_eq!(c.catalog_entries, 2);
//! assert_eq!(c.typed_datasets, 2);
//! ```
#![forbid(unsafe_code)]

/// kedro config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kedro {
    /// Top-level `<name>:` catalog/parameter entries.
    pub catalog_entries: usize,
    /// `type:` dataset class references.
    pub typed_datasets: usize,
    /// `filepath:`/`path:` entries.
    pub filepaths: usize,
    /// `settings.yml`-style keys (`HOOKS`/`SESSION_STORE_ARGS`/…).
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// settings.yml keys.
const SETTINGS_KEYS: &[&str] = &[
    "HOOKS",
    "DISABLE_HOOKS_FOR_PLUGINS",
    "SESSION_STORE_ARGS",
    "CONFIG_LOADER_ARGS",
    "DATA_CATALOG",
    "CONFIG_LOADER_CLASS",
    "CONTEXT_CLASS",
    "DYNAMIC_PIPELINES",
    "CONF_SOURCE",
    "KEDRO_TELEMETRY",
];

/// True if `b` looks like a kedro config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("type:") && t.contains("filepath:"))
        || SETTINGS_KEYS.iter().any(|k| t.contains(k))
        || t.contains("pandas.CSVDataset")
        || t.contains("Dataset:")
}

impl Kedro {
    /// Parse a kedro config file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            catalog_entries: 0,
            typed_datasets: 0,
            filepaths: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if l.starts_with("type:") {
                c.typed_datasets += 1;
            } else if l.starts_with("filepath:") || l.starts_with("path:") {
                c.filepaths += 1;
            } else if indent == 0 && l.ends_with(':') {
                let key = l.trim_end_matches(':');
                if SETTINGS_KEYS.contains(&key) {
                    c.settings += 1;
                } else {
                    c.catalog_entries += 1;
                }
            }
        }
        if c.catalog_entries == 0 && c.settings == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# catalog\n",
            "companies:\n",
            "  type: pandas.CSVDataset\n",
            "  filepath: data/01_raw/companies.csv\n",
            "reviews:\n",
            "  type: pandas.CSVDataset\n",
            "  filepath: data/01_raw/reviews.csv\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Kedro::parse(b.as_bytes()).unwrap();
        assert_eq!(c.catalog_entries, 2);
        assert_eq!(c.typed_datasets, 2);
        assert_eq!(c.filepaths, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Kedro::parse(b"# none\n").is_none());
    }
}
