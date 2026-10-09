//! CRI-O `crio.conf` の検出と構造カウント。
//!
//! `[crio]`/`[crio.runtime]`/`[crio.image]`/`[crio.network]`/`[crio.metrics]`/
//! `[crio.tracing]`/`[crio.api]`/`[crio.stats]`/`[crio.nri]`/`[crio.works]`
//! テーブルと、`default_runtime`/`conmon`/`pause_image`/`cni_default_network`/
//! `plugin_dirs`/`enable_metrics`/`metrics_port`/`grpc` 等のキーを識別する。
//!
//! ```
//! let c = izanagi_kit::crio::parse(
//!     b"[crio]\nroot = \"/var/lib/containers/storage\"\nrunroot = \"/run/containers/storage\"\n[crio.runtime]\ndefault_runtime = \"crun\"\n").unwrap();
//! assert!(c.sections >= 2);
//! assert!(izanagi_kit::crio::detect(
//!     b"[crio.image]\npause_image = \"registry.k8s.io/pause:3.9\"\n"));
//! ```

use crate::textutil::strip_bom;
/// テーブル(先頭 `crio` プレフィックス)。
const TABLES: &[&str] = &["crio"];

/// 既知キー(サブテーブル内を含む)。
const KEYS: &[&str] = &[
    "add_inheritable_capabilities",
    "additional_devices",
    "apparmor_profile",
    "big_files_temporary_dir",
    "bind_mount_prefix",
    "cgroup_manager",
    "cni_config_dir",
    "cni_default_network",
    "cni_plugin_dirs",
    "conmon",
    "conmon_cgroup",
    "conmon_env",
    "container_attach_socket_dir",
    "container_exits_dir",
    "ctr_stop_timeout",
    "decryption_keys_path",
    "default_capabilities",
    "default_mounts",
    "default_mounts_file",
    "default_runtime",
    "default_sysctls",
    "device_ownership_from_security_context",
    "drop_infra_ctr",
    "enable_metrics",
    "enable_tracing",
    "filter_relabel",
    "grpc_max_msg_size",
    "grpc_max_recv_msg_size",
    "grpc_max_send_msg_size",
    "hooks_dir",
    "image_volumes",
    "internal_wipe",
    "irqbalance_config_file",
    "irqbalance_config_restore_file",
    "label",
    "log",
    "log_dir",
    "log_filter",
    "log_level",
    "log_size_max",
    "log_to_journald",
    "max_open_files",
    "max_processes",
    "metrics_cert",
    "metrics_collector_include",
    "metrics_collectors",
    "metrics_host",
    "metrics_key",
    "metrics_port",
    "metrics_socket",
    "namespaces_dir",
    "network_dir",
    "no_pivot",
    "pause_command",
    "pause_image",
    "pause_image_auth_file",
    "pids_limit",
    "plugin_dirs",
    "pull_progress_timeout",
    "read_only",
    "root",
    "runroot",
    "runtimes",
    "runtime_config_path",
    "runtime_path",
    "runtime_reconcile_interval",
    "runtime_root",
    "runtime_type",
    "runtime_unshare_workloads",
    "seccomp_profile",
    "selinux",
    "separate_pull_cgroup",
    "stats_collection_period",
    "storage_driver",
    "storage_option",
    "stream_address",
    "stream_enable_tls",
    "stream_idle_timeout",
    "stream_port",
    "timed_out_state",
    "uid_mappings",
    "umask",
    "version_file",
    "version_file_persist",
    "workloads",
    "works_dir",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[crio.*]`/`[crio]` テーブル行数。
    pub sections: usize,
    /// 既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn table_of(t: &str) -> &str {
    if !(t.starts_with('[') && t.contains(']')) {
        return "";
    }
    &t[1..t.find(']').unwrap_or(1)]
}

fn is_crio_table(t: &str) -> bool {
    let inner = table_of(t);
    !inner.is_empty() && TABLES.contains(&inner.split('.').next().unwrap_or(""))
}

fn known_key(t: &str) -> bool {
    let Some(eq) = t.find('=') else { return false };
    KEYS.contains(&t[..eq].trim())
}

/// `crio.conf` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut crio_table = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') {
            crio_table = is_crio_table(t);
            hits += usize::from(crio_table);
            continue;
        }
        if known_key(t) {
            hits += 1;
        }
        if hits >= 3 || (crio_table && hits >= 2) {
            return true;
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') {
            c.sections += 1;
        } else if known_key(t) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# crio.conf\n[crio]\nroot = \"/var/lib/containers/storage\"\nrunroot = \"/run/containers/storage\"\nstorage_driver = \"overlay\"\nlog_level = \"info\"\n\n[crio.api]\ngrpc_max_recv_msg_size = 8388608\n\n[crio.runtime]\ndefault_runtime = \"crun\"\nconmon = \"/usr/bin/conmon\"\nconmon_cgroup = \"pod\"\npids_limit = 1024\nselinux = false\n\n[crio.image]\npause_image = \"registry.k8s.io/pause:3.9\"\npause_image_auth_file = \"\"\n\n[crio.network]\nnetwork_dir = \"/etc/cni/net.d/\"\nplugin_dirs = [\"/opt/cni/bin/\"]\n\n[crio.metrics]\nenable_metrics = true\nmetrics_port = 9090\n";

    #[test]
    fn crio() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.options, 16);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_crio() {
        assert!(!detect(b"[other]\nkey = 1\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
