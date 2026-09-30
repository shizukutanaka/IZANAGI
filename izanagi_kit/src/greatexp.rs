//! Census of a `great_expectations.yml` file.
//!
//! `config_version:` header plus `datasources:`/`fluent_datasources:`/
//! `stores:`/`expectations_store`/`validations_store`/
//! `evaluation_parameter_store`/`checkpoint_store`/`profiler_store`/
//! `data_docs_sites`/`anonymous_usage_statistics`/`notebooks`/
//! `concurrency`/`progress_bars`. Store entries are `<name>:` blocks
//! under `stores:` with `class_name:`/`module_name:`/`store_backend:`.
//! Counts stores, docs sites, datasource entries and comments.
//!
//! ```rust
//! let c = izanagi_kit::greatexp::GreatExp::parse(
//!     b"config_version: 4.0\nstores:\n  expectations_store:\n    class_name: ExpectationsStore\n\
//!       data_docs_sites:\n  local_site:\n    class_name: SiteBuilder\n",
//! ).unwrap();
//! assert_eq!(c.stores, 1);
//! assert_eq!(c.docs_sites, 1);
//! ```
#![forbid(unsafe_code)]

/// great_expectations.yml census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GreatExp {
    /// Named store blocks under `stores:`.
    pub stores: usize,
    /// Named site blocks under `data_docs_sites:`.
    pub docs_sites: usize,
    /// `class_name:`/`module_name:`/`store_backend:`/`file_path:` field lines.
    pub fields: usize,
    /// `fluent_datasources:`/`datasources:` named entries.
    pub datasources: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Field keys inside blocks.
const FIELD_KEYS: &[&str] = &[
    "class_name",
    "module_name",
    "store_backend",
    "file_path",
    "base_directory",
    "data_context_id",
    "encryption_key",
    "suppress_database_store_backend_id",
];

/// True if `b` looks like great_expectations.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("data_docs_sites")
        || (t.contains("stores:") && t.contains("class_name:"))
        || (t.contains("config_version:") && t.contains("store_name"))
        || t.contains("expectations_store")
}

impl GreatExp {
    /// Parse great_expectations.yml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            stores: 0,
            docs_sites: 0,
            fields: 0,
            datasources: 0,
            comments: 0,
        };
        let mut section: Option<&str> = None;
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
            if indent == 0 {
                section = if l == "stores:" {
                    Some("stores")
                } else if l == "data_docs_sites:" {
                    Some("sites")
                } else if l == "datasources:" || l == "fluent_datasources:" {
                    Some("datasources")
                } else {
                    None
                };
                continue;
            }
            if l.starts_with('-') {
                if section == Some("datasources") {
                    c.datasources += 1;
                }
                continue;
            }
            let key = l.split(':').next().unwrap_or("").trim();
            if FIELD_KEYS.contains(&key) {
                c.fields += 1;
                continue;
            }
            if l.ends_with(':') {
                match section {
                    Some("stores") => c.stores += 1,
                    Some("sites") => c.docs_sites += 1,
                    Some("datasources") => c.datasources += 1,
                    _ => {}
                }
            }
        }
        if c.stores == 0 && c.docs_sites == 0 && c.fields == 0 {
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
            "config_version: 4.0\n",
            "stores:\n",
            "  expectations_store:\n",
            "    class_name: ExpectationsStore\n",
            "    store_backend:\n",
            "      class_name: TupleFilesystemStoreBackend\n",
            "data_docs_sites:\n",
            "  local_site:\n",
            "    class_name: SiteBuilder\n",
            "anonymous_usage_statistics:\n",
            "  enabled: true\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = GreatExp::parse(b.as_bytes()).unwrap();
        assert_eq!(c.stores, 1);
        assert_eq!(c.docs_sites, 1);
        assert_eq!(c.fields, 4);
        assert_eq!(c.comments, 0);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(GreatExp::parse(b"# none\n").is_none());
    }
}
