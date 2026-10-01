//! MLflow `MLmodel` metadata file parser.
//!
//! Detects `flavors:` + `mlflow_version:`/`model_uuid:` metadata files
//! and counts flavor entries, signature blocks, loader keys and
//! timestamps.
//!
//! ```
//! let b = concat!(
//!     "artifact_path: model\n",
//!     "flavors:\n",
//!     "  python_function:\n",
//!     "    loader_module: mlflow.pyfunc\n",
//!     "    env: conda.yaml\n",
//!     "  sklearn:\n",
//!     "    pickled_model: model.pkl\n",
//!     "signature:\n",
//!     "  inputs: '[{\"type\":\"long\"}]'\n",
//!     "model_uuid: abc\n",
//!     "mlflow_version: 2\n"
//! ).as_bytes();
//! assert!(izanagi_kit::mlflow::detect(b));
//! let c = izanagi_kit::mlflow::Mlflow::parse(b).unwrap();
//! assert_eq!(c.flavors, 2);
//! ```

/// Parsed MLmodel file summary.
#[derive(Debug, Clone)]
pub struct Mlflow {
    /// Flavor entries (one-indent keys under `flavors:`).
    pub flavors: usize,
    /// `python_function` flavor present flag counted with others.
    pub loader_keys: usize,
    /// `signature` section present (`inputs:`/`outputs:`/`params:`).
    pub signature_fields: usize,
    /// `model_uuid`/`run_id`/`utc_time_created`/`mlflow_version`
    /// metadata keys.
    pub metadata: usize,
    /// `saved_input_example_info` entries.
    pub input_examples: usize,
}

/// Whether the buffer looks like an MLflow `MLmodel` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("flavors:")
        && (t.contains("mlflow_version:")
            || t.contains("model_uuid:")
            || t.contains("python_function:")
            || t.contains("utc_time_created:"))
}

impl Mlflow {
    /// Parses an MLmodel file summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            flavors: 0,
            loader_keys: 0,
            signature_fields: 0,
            metadata: 0,
            input_examples: 0,
        };
        let mut in_flavors = false;
        for l in t.lines() {
            if l.starts_with("flavors:") {
                in_flavors = true;
                continue;
            }
            if in_flavors {
                if l.starts_with("  ") && !l.starts_with("    ") {
                    c.flavors += 1;
                } else if !l.starts_with(' ') {
                    in_flavors = false;
                }
            }
            let tr = l.trim();
            for k in [
                "loader_module:",
                "cloudpickle_version:",
                "pickled_model:",
                "model_path:",
                "env:",
                "data:",
            ] {
                if tr.starts_with(k) {
                    c.loader_keys += 1;
                }
            }
            for k in ["inputs:", "outputs:", "params:", "signature:"] {
                if tr.starts_with(k) {
                    c.signature_fields += 1;
                }
            }
            for k in [
                "model_uuid:",
                "run_id:",
                "utc_time_created:",
                "mlflow_version:",
            ] {
                if tr.starts_with(k) {
                    c.metadata += 1;
                }
            }
            if tr.starts_with("saved_input_example_info:") || tr.starts_with("example_") {
                c.input_examples += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "artifact_path: model\n",
            "flavors:\n",
            "  python_function:\n",
            "    loader_module: mlflow.pyfunc\n",
            "    env: conda.yaml\n",
            "    cloudpickle_version: 2\n",
            "  sklearn:\n",
            "    pickled_model: model.pkl\n",
            "    sklearn_version: 1\n",
            "signature:\n",
            "  inputs: '[{\"type\":\"long\"}]'\n",
            "  outputs: '[{\"type\":\"tensor\"}]'\n",
            "saved_input_example_info:\n",
            "  artifact_path: input_example.json\n",
            "model_uuid: abc\n",
            "run_id: r1\n",
            "utc_time_created: '2024-01-01'\n",
            "mlflow_version: 2\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Mlflow::parse(b).unwrap();
        assert_eq!(c.flavors, 2);
        assert_eq!(c.loader_keys, 4);
        assert_eq!(c.signature_fields, 3);
        assert_eq!(c.metadata, 4);
        assert_eq!(c.input_examples, 1);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"flavors: x\n"));
        assert!(!detect(b"name: x\n"));
        assert!(Mlflow::parse(b"x").is_none());
    }
}
