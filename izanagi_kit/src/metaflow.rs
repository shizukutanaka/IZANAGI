//! Census of a Metaflow `config.json` (`~/.metaflowconfig/config.json`).
//!
//! A flat JSON object of `METAFLOW_*` environment-variable overrides:
//! `METAFLOW_DEFAULT_METADATA`/`METAFLOW_DATASTORE_SYSROOT_*`/
//! `METAFLOW_BATCH_*`/`METAFLOW_KUBERNETES_*`/`METAFLOW_SERVICE_*`/
//! `METAFLOW_SFN_*`/`METAFLOW_CARD_*`/`METAFLOW_ARTIFACT_*`/
//! `METAFLOW_CLIENT_*`/`METAFLOW_PLUGIN_*`. Counts keys, grouped by
//! prefix family.
//!
//! ```rust
//! let c = izanagi_kit::metaflow::Metaflow::parse(
//!     b"{\"METAFLOW_DEFAULT_METADATA\":\"local\",\"METAFLOW_BATCH_JOB_QUEUE\":\"q\",\
//!       \"METAFLOW_DATASTORE_SYSROOT_S3\":\"s3://b\"}",
//! ).unwrap();
//! assert_eq!(c.keys, 3);
//! ```
#![forbid(unsafe_code)]

/// metaflow config.json census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metaflow {
    /// `METAFLOW_*` keys.
    pub keys: usize,
    /// `METAFLOW_BATCH_*` keys.
    pub batch: usize,
    /// `METAFLOW_KUBERNETES_*` keys.
    pub kubernetes: usize,
    /// `METAFLOW_SERVICE_*`/`METAFLOW_SFN_*` keys.
    pub service: usize,
    /// `METAFLOW_DATASTORE_*`/`METAFLOW_ARTIFACT_*` keys.
    pub datastore: usize,
}

/// True if `b` looks like metaflowconfig/config.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.trim_start().starts_with('{') && t.contains("\"METAFLOW_")
}

impl Metaflow {
    /// Parse a metaflow config.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !t.trim_start().starts_with('{') {
            return None;
        }
        let mut c = Self {
            keys: 0,
            batch: 0,
            kubernetes: 0,
            service: 0,
            datastore: 0,
        };
        let mut rest = t;
        while let Some(q) = rest.find("\"METAFLOW_") {
            let after = &rest[q + 1..];
            let Some(end) = after.find('"') else {
                break;
            };
            let key = &after[..end];
            // Must be followed by `:` (a key, not a value).
            let nxt = after[end + 1..].trim_start();
            if nxt.starts_with(':') {
                c.keys += 1;
                if key.starts_with("METAFLOW_BATCH_") {
                    c.batch += 1;
                } else if key.starts_with("METAFLOW_KUBERNETES_") {
                    c.kubernetes += 1;
                } else if key.starts_with("METAFLOW_SERVICE_") || key.starts_with("METAFLOW_SFN_") {
                    c.service += 1;
                } else if key.starts_with("METAFLOW_DATASTORE_")
                    || key.starts_with("METAFLOW_ARTIFACT_")
                {
                    c.datastore += 1;
                }
            }
            rest = &after[end..];
        }
        if c.keys == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        concat!(
            "{\"METAFLOW_DEFAULT_METADATA\":\"local\",",
            "\"METAFLOW_BATCH_JOB_QUEUE\":\"q\",",
            "\"METAFLOW_KUBERNETES_NAMESPACE\":\"mf\",",
            "\"METAFLOW_SERVICE_URL\":\"https://mf\",",
            "\"METAFLOW_DATASTORE_SYSROOT_S3\":\"s3://b\"}",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Metaflow::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 5);
        assert_eq!(c.batch, 1);
        assert_eq!(c.kubernetes, 1);
        assert_eq!(c.service, 1);
        assert_eq!(c.datastore, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"name\":\"x\"}"));
        assert!(Metaflow::parse(b"x").is_none());
    }
}
