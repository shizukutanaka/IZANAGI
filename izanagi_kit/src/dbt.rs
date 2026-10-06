//! dbt `dbt_project.yml` / `profiles.yml` detection and census.
//!
//! Detects dbt project markers (`model-paths:`, `require-dbt-version:`,
//! `seed-paths:`, `snapshot-paths:`) or a `name:`/`version:`/`profile:`
//! triple, then counts project keys, path keys, resource blocks
//! (`models:`/`seeds:`/`snapshots:`/`tests:`/`analyses:`/`macros:`/`docs:`/
//! `metrics:`/`semantic_models:`/`saved_queries:`/`exposures:`/`unit_tests:`/
//! `vars:`/`dispatch:`/`flags:`/`quoting:`/`on-run-start:`/`on-run-end:`),
//! `+`-prefixed config keys (`+materialized`/`+schema`/`+tags`/`+pre-hook`/
//! `+post-hook`/`+grants`/`+persist_docs`/`+meta`/`+docs`/`+group`/`+access`/
//! `+contract`/`+incremental_strategy`/`+on_schema_change`/`+unique_key`/
//! `+partition_by`/`+cluster_by`/`+full_refresh`/`+enabled`/`+severity`/
//! `+transient`/`+static`/`+batch_id`/`+event_time`/`+check_cols`/`+strategy`/
//! `+updated_at`/`+sql_header`/`+fail_calc`/`+where`/`+limit`/`+store_failures`/
//! `+store_failures_as`/`+quote_columns`/`+invalidate_hard_deletes`/
//! `+snapshot_meta_column_names`/`+target_schema`/`+target_database`),
//! resource-definition keys (`description:`/`columns:`/`config:`/`enabled:`/
//! `grants:`/`meta:`/`tests:`/`data_tests:`/`docs:`/`deprecation_date:`/
//! `versions:`/`latest_version:`/`access:`/`constraints:`/`contract:`/
//! `language:`/`salience:`/`tags:`), and `#` comment lines.
//!
//! ```
//! let b = b"name: my_project\nversion: 1\nprofile: my_profile\nmodel-paths: [\"models\"]\nseed-paths: [\"seeds\"]\nrequire-dbt-version: [\">=1\", \"<2\"]\nmodels:\n  my_project:\n    marts:\n      +materialized: table\nseeds:\n  +schema: seed_schema\n";
//! assert!(izanagi_kit::dbt::detect(b));
//! let c = izanagi_kit::dbt::Dbt::parse(b).unwrap();
//! assert_eq!(c.paths, 2);
//! assert_eq!(c.blocks, 2);
//! assert_eq!(c.config_keys, 2);
//! ```

/// Parsed dbt project summary.
#[derive(Debug, Clone)]
pub struct Dbt {
    /// top-level project keys (`name`/`version`/`profile`/`require-dbt-version`/…).
    pub project_keys: usize,
    /// path keys (`model-paths`/`seed-paths`/`clean-targets`/…).
    pub paths: usize,
    /// resource/config blocks (`models:`/`seeds:`/`vars:`/`quoting:`/…).
    pub blocks: usize,
    /// `+`-prefixed materialization config keys.
    pub config_keys: usize,
    /// resource-definition keys (`description:`/`columns:`/`config:`/…).
    pub model_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const PROJECT_KEYS: &[&str] = &[
    "name:",
    "version:",
    "profile:",
    "config-version:",
    "require-dbt-version:",
    "dbt-cloud:",
    "install-deps:",
    "query-comment:",
    "write-json:",
    "warn-error:",
    "warn-error-options:",
    "deprecation_date:",
];

const PATH_KEYS: &[&str] = &[
    "model-paths:",
    "seed-paths:",
    "test-paths:",
    "analysis-paths:",
    "macro-paths:",
    "snapshot-paths:",
    "data-paths:",
    "docs-paths:",
    "asset-paths:",
    "clean-targets:",
    "target-path:",
    "log-path:",
    "packages-install-path:",
    "dbt-plugins-path:",
];

const BLOCK_KEYS: &[&str] = &[
    "models:",
    "seeds:",
    "snapshots:",
    "tests:",
    "analyses:",
    "macros:",
    "docs:",
    "metrics:",
    "semantic_models:",
    "saved_queries:",
    "exposures:",
    "unit_tests:",
    "vars:",
    "dispatch:",
    "flags:",
    "quoting:",
    "on-run-start:",
    "on-run-end:",
    "restrict-access:",
];

const CONFIG_KEYS: &[&str] = &[
    "+materialized",
    "+schema",
    "+alias",
    "+tags",
    "+pre-hook",
    "+post-hook",
    "+grants",
    "+persist_docs",
    "+meta",
    "+docs",
    "+group",
    "+access",
    "+contract",
    "+incremental_strategy",
    "+on_schema_change",
    "+unique_key",
    "+partition_by",
    "+cluster_by",
    "+full_refresh",
    "+enabled",
    "+severity",
    "+transient",
    "+static",
    "+batch_id",
    "+event_time",
    "+check_cols",
    "+strategy",
    "+updated_at",
    "+sql_header",
    "+fail_calc",
    "+where",
    "+limit",
    "+store_failures",
    "+store_failures_as",
    "+quote_columns",
    "+invalidate_hard_deletes",
    "+snapshot_meta_column_names",
    "+target_schema",
    "+target_database",
    "+error_if",
    "+warn_if",
    "+database",
    "+indexes",
    "+column_types",
    "+labels",
    "+concurrent_batches",
    "+skip",
    "+merge_exclude_columns",
    "+merge_update_columns",
    "+predicates",
];

const MODEL_KEYS: &[&str] = &[
    "description:",
    "columns:",
    "config:",
    "enabled:",
    "grants:",
    "meta:",
    "tests:",
    "data_tests:",
    "docs:",
    "deprecation_date:",
    "versions:",
    "latest_version:",
    "access:",
    "constraints:",
    "contract:",
    "language:",
    "salience:",
    "tags:",
];

fn key_is(line: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| line.starts_with(k))
}
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim_start(), key))
}

/// Detects dbt project YAML files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    let has_marker = [
        "model-paths",
        "require-dbt-version",
        "seed-paths",
        "snapshot-paths",
    ]
    .iter()
    .any(|m| has_key(&t, m));
    let triple = has_key(&t, "name") && has_key(&t, "version") && has_key(&t, "profile");
    has_marker || triple
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

impl Dbt {
    /// Parses a dbt project file, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            project_keys: 0,
            paths: 0,
            blocks: 0,
            config_keys: 0,
            model_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim_end();
            if tr.trim_start().starts_with('#') {
                c.comments += 1;
                continue;
            }
            let tr = tr.trim_start();
            if key_is(tr, PROJECT_KEYS) {
                c.project_keys += 1;
            }
            if key_is(tr, PATH_KEYS) {
                c.paths += 1;
            }
            if indent(l) == 0 && key_is(tr, BLOCK_KEYS) {
                c.blocks += 1;
            }
            if key_is(tr, MODEL_KEYS) {
                c.model_keys += 1;
            }
        }
        c.config_keys = CONFIG_KEYS.iter().map(|k| t.matches(k).count()).sum();
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# name: x\n# version: 1\n# profile: p\n"));
        assert!(!detect(b"# model-paths: [\"models\"]\n"));
    }

    #[test]
    fn detects_and_counts() {
        let b = b"name: my_project\nversion: 1\nprofile: my_profile\nmodel-paths: [\"models\"]\nseed-paths: [\"seeds\"]\nclean-targets: [\"target\"]\nrequire-dbt-version: [\">=1\", \"<2\"]\nmodels:\n  my_project:\n    marts:\n      +materialized: table\n      +schema: marts\nseeds:\n  +schema: seed_schema\nvars:\n  region: us\n";
        let c = Dbt::parse(b).unwrap();
        assert!(c.project_keys >= 4);
        assert_eq!(c.paths, 3);
        assert_eq!(c.blocks, 3);
        assert_eq!(c.config_keys, 3);
    }

    #[test]
    fn detects_profiles() {
        let b = b"name: my_project\nversion: 1\nprofile: my_profile\nmodel-paths: [\"models\"]\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Dbt::parse(b"key: value\nother: x\n").is_none());
        assert!(!detect(b"services:\n  app:\n    image: x\n"));
    }
}
