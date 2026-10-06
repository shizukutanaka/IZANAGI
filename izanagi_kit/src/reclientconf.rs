//! reclient `reclient.cfg`/`reproxy.cfg`（RBE リモート実行）の検出と
//! 構造カウント。
//!
//! `service`/`instance`/`exec_strategy`/`server_address`/`cas_server`/
//! `log_path`/`remote_cache`/`download_outputs`/`fail_cas`/
//! `num_rbe_workers`/`max_concurrent_uploads`/`max_concurrent_executions`/
//! `file_cache_buckets`/`metrics_project` 等の `key=value` 行を識別する。
//!
//! ```
//! let b = b"service=rbe.example.com:443\ninstance=projects/p/instances/i\nexec_strategy=remote_local_fallback\nserver_address=unix:///tmp/reproxy.sock\n";
//! assert!(izanagi_kit::reclientconf::detect(b));
//! let c = izanagi_kit::reclientconf::ReclientConf::parse(b).unwrap();
//! assert_eq!(c.keys, 4);
//! ```

/// Parsed reclient config summary.
#[derive(Debug, Clone)]
pub struct ReclientConf {
    /// Recognized option keys.
    pub keys: usize,
    /// `key=value` lines.
    pub assignments: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

/// reclient/reproxy option names.
const KEYS: &[&str] = &[
    "abide_by_hard_links",
    "accept_cached",
    "accessor",
    "cas_server",
    "cleanup",
    "compare_execution_logs",
    "depsscanner",
    "depsscanner_address",
    "download_outputs",
    "exec_strategy",
    "exec_strategy_options",
    "fail_cas",
    "file_cache_buckets",
    "file_cache_max_gb",
    "gc_interval_secs",
    "instance",
    "ip_ban_duration",
    "labels",
    "log_path",
    "max_concurrent_executions",
    "max_concurrent_uploads",
    "max_exec_timeout_secs",
    "metrics_namespace",
    "metrics_prefix",
    "metrics_project",
    "num_rbe_workers",
    "operating_system",
    "pipe",
    "platform",
    "preserve_symlinks",
    "proxy_log_format",
    "race",
    "remote_cache",
    "remote_cache_jobs",
    "remote_timeout",
    "remote_wrapper",
    "reproxy_debug",
    "server_address",
    "service",
    "service_no_auth",
    "service_no_security",
    "silence_log",
    "ssl_server_name_override",
    "startup_resource_check",
    "tmp_dir",
    "toolchain_inputs",
    "use_application_default_credentials",
    "v",
    "version",
    "verifier_enabled",
];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#')
            && !tr.starts_with("//")
            && tr.starts_with(k)
            && tr[k.len()..].trim_start().starts_with('=')
    })
}

/// Detect a reclient config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let n = KEYS.iter().filter(|k| key_present(t, k)).count();
    (key_present(t, "service") && n >= 3) || n >= 4
}

impl ReclientConf {
    /// Count categories. Returns `None` when the input does not look like
    /// a reclient config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
                if KEYS
                    .iter()
                    .any(|k| tr.starts_with(k) && tr[k.len()..].trim_start().starts_with('='))
                {
                    c.keys += 1;
                }
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`ReclientConf::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<ReclientConf> {
    ReclientConf::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"service=rbe.example.com:443\ninstance=projects/p/instances/i\nservice_no_security=false\nexec_strategy=remote_local_fallback\nserver_address=unix:///tmp/reproxy.sock\ncas_server=rbe.example.com:443\nremote_cache=localhost:8980\ndownload_outputs=true\nfail_cas=false\nlog_path=/tmp/reproxy.log\nnum_rbe_workers=8\nmax_concurrent_uploads=4\nmax_concurrent_executions=16\nfile_cache_buckets=16\nfile_cache_max_gb=32\nmetrics_project=myproj\n";
        assert!(detect(b));
        let c = ReclientConf::parse(b).unwrap();
        assert_eq!(c.keys, 16);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"service=x\ninstance=y\nport=1\n"));
        assert!(!detect(b"[section]\nkey=v\nkey2=v2\nkey3=v3\nkey4=v4\n"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"# service=rbe\n# instance=i\n# exec_strategy=remote\n"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(ReclientConf::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"service=");
        assert!(!detect(&b));
    }
}
