//! Dagster `dagster.yaml` / `workspace.yaml` detection and census.
//!
//! Detects instance-config keys (`run_launcher:`/`run_storage:`/
//! `event_log_storage:`/`schedule_storage:`/`compute_logs:`/
//! `local_artifact_storage:`/`instance_concurrency_limits:`) and
//! workspace `load_from:` entries (`python_file`/`python_module`/
//! `python_package`/`grpc_server`), then counts storage keys
//! (`run_storage`/`event_log_storage`/`schedule_storage`/
//! `local_artifact_storage`/`compute_logs`/`run_monitoring`/`run_retries`/
//! `secrets`/`retention`/`execution`/`telemetry`/`nux`/`code_servers`/
//! `remote_instance`/`feature_flags`/`concurrency`/`auto_materialize`/
//! `daemon`), launcher/coordinator keys (`run_launcher`/`run_coordinator`/
//! `run_queue`/`max_concurrent_runs`/`tag_concurrency_limits`),
//! `load_from:` locations (`- python_file:`/`- python_module:`/
//! `- python_package:`/`- grpc_server:`/`- module:`/`- package:`),
//! location keys (`location_name`/`code_source`/`attribute`/
//! `executable_path`/`working_directory`/`python_file`/`module_name`/
//! `package_name`/`file_name`/`grpc_server`/`host`/`port`/`socket`/
//! `server_name`/`ssl`/`image_name`/`container_image`/`container_context`/
//! `pull_policy`), and `#` comment lines.
//!
//! ```
//! let b = b"run_launcher:\n  module: dagster.core.launcher\n  class: DefaultRunLauncher\nrun_storage:\n  module: dagster.core.storage.runs\n  class: SqliteRunStorage\nevent_log_storage:\n  module: dagster.core.storage.event_log\n  class: SqliteEventLogStorage\nlocal_artifact_storage:\n  module: dagster.core.storage.root\n  class: LocalArtifactStorage\ncompute_logs:\n  module: dagster.core.storage.local_compute_log_manager\n  class: LocalComputeLogManager\n  config:\n    base_dir: /tmp/logs\n";
//! assert!(izanagi_kit::dagster::detect(b));
//! let c = izanagi_kit::dagster::Dagster::parse(b).unwrap();
//! assert_eq!(c.storage_keys, 4);
//! assert!(c.module_keys >= 10);
//! ```

/// Parsed Dagster config/workspace summary.
#[derive(Debug, Clone)]
pub struct Dagster {
    /// instance storage/runtime keys (`run_storage`/`compute_logs`/…).
    pub storage_keys: usize,
    /// `run_launcher`/`run_coordinator`/queue keys.
    pub launcher_keys: usize,
    /// `load_from:` code locations (`- python_file:`/`- grpc_server:`/…).
    pub locations: usize,
    /// `module:`/`class:`/`config:`/location-detail keys.
    pub module_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STORAGE_KEYS: &[&str] = &[
    "run_storage:",
    "event_log_storage:",
    "schedule_storage:",
    "local_artifact_storage:",
    "compute_logs:",
    "run_monitoring:",
    "run_retries:",
    "secrets:",
    "retention:",
    "execution:",
    "telemetry:",
    "nux:",
    "code_servers:",
    "remote_instance:",
    "feature_flags:",
    "concurrency:",
    "auto_materialize:",
    "daemon:",
    "backfill:",
    "webserver:",
    "grpc_servers:",
    "schedules:",
    "sensors:",
];

const LAUNCHER_KEYS: &[&str] = &[
    "run_launcher:",
    "run_coordinator:",
    "run_queue:",
    "max_concurrent_runs:",
    "tag_concurrency_limits:",
    "dequeue_interval_seconds:",
    "dequeue_num_workers:",
    "dequeue_use_threads:",
];

const LOCATION_MARKERS: &[&str] = &[
    "- python_file:",
    "- python_module:",
    "- python_package:",
    "- grpc_server:",
    "- grpc:",
    "- module:",
    "- package:",
];

const MODULE_KEYS: &[&str] = &[
    "module:",
    "class:",
    "config:",
    "location_name:",
    "code_source:",
    "attribute:",
    "executable_path:",
    "working_directory:",
    "python_file:",
    "module_name:",
    "package_name:",
    "file_name:",
    "grpc_server:",
    "host:",
    "port:",
    "socket:",
    "server_name:",
    "ssl:",
    "image_name:",
    "container_image:",
    "container_context:",
    "pull_policy:",
    "base_dir:",
    "storage_dir:",
    "postgres_url:",
    "mysql_url:",
    "mssql_url:",
    "max_workers:",
    "poll_interval:",
    "wait_for_local_processes_on_shutdown:",
    "additional_scope:",
    "env_vars:",
    "secrets_path:",
    "ttl:",
    "enabled:",
];

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Detects Dagster instance/workspace YAML files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    let inst = [
        "run_launcher:",
        "event_log_storage:",
        "schedule_storage:",
        "local_artifact_storage:",
        "compute_logs:",
        "run_storage:",
        "instance_concurrency_limits:",
    ]
    .iter()
    .any(|m| t.contains(m));
    let ws = t.contains("load_from:")
        && (t.contains("python_file") || t.contains("python_module") || t.contains("grpc_server"));
    inst || ws
}

impl Dagster {
    /// Parses a Dagster YAML file, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            storage_keys: 0,
            launcher_keys: 0,
            locations: 0,
            module_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim_end();
            if tr.trim_start().starts_with('#') {
                c.comments += 1;
                continue;
            }
            let tr = tr.trim_start();
            if indent(l) == 0 && STORAGE_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.storage_keys += 1;
            }
            if LAUNCHER_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.launcher_keys += 1;
            }
            if LOCATION_MARKERS.iter().any(|k| tr.starts_with(k)) {
                c.locations += 1;
            }
            if MODULE_KEYS
                .iter()
                .any(|k| tr.trim_start_matches("- ").starts_with(k))
            {
                c.module_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_instance_yaml() {
        let b = b"run_launcher:\n  module: dagster.core.launcher\n  class: DefaultRunLauncher\nrun_storage:\n  module: dagster.core.storage.runs\n  class: SqliteRunStorage\nevent_log_storage:\n  module: dagster.core.storage.event_log\n  class: SqliteEventLogStorage\nlocal_artifact_storage:\n  module: dagster.core.storage.root\n  class: LocalArtifactStorage\ncompute_logs:\n  module: dagster.core.storage.local_compute_log_manager\n  class: LocalComputeLogManager\n  config:\n    base_dir: /tmp/logs\n";
        let c = Dagster::parse(b).unwrap();
        assert_eq!(c.storage_keys, 4);
        assert_eq!(c.launcher_keys, 1);
        assert!(c.module_keys >= 10);
    }

    #[test]
    fn detects_workspace_yaml() {
        let b = b"load_from:\n  - python_file: repos/etl.py\n  - python_module:\n      module_name: analytics.defs\n      location_name: analytics\n  - grpc_server:\n      host: localhost\n      port: 4000\n";
        let c = Dagster::parse(b).unwrap();
        assert_eq!(c.locations, 3);
        assert!(c.module_keys >= 5);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Dagster::parse(b"key: value\n").is_none());
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }
}
