//! `.condarc` (conda configuration YAML) census.
//!
//! `channels:`/`default_channels:`/`custom_channels:`/
//! `custom_multichannels:`/`migrated_custom_channels:`/
//! `channel_priority:`/`envs_dirs:`/`pkgs_dirs:`/`ssl_verify:`/
//! `proxy_servers:`/`create_default_packages:`/`auto_update_conda:`/
//! `allow_non_channel_urls:`/`restore_free_channel:`/
//! `show_channel_urls:`/`offline:`/`use_only_tar_bz2:`/`repodata_fns:`/
//! `repodata_threads:`/`remote_connect_timeout_secs:`/
//! `remote_read_timeout_secs:`/`remote_max_retries:`/
//! `remote_backoff_factor:`/`local_repodata_ttl:`/
//! `add_pip_as_python_dependency:`/`auto_activate_base:`/
//! `auto_stack:`/`changeps1:`/`add_anaconda_token:`/`alph:`/`clobber:`/
//! `force:`/`force_reinstall:`/`freeze_installed:`/`update_dependencies:`/
//! `channel_alias:`/`sat_solver:`/`solver_class:`/`deps_modifier:`/
//! `track_features:`/`pinned_packages:`/`disallowed_packages:`/
//! `env_prompt:`/`subdir:`/`subdirs:`/`whitelist_channels:`/
//! `proxy_servers`/`use_index_cache:`/`pip_interop_enabled:`/
//! `notify_outdated_conda:`/`quiet:`/`verbosity:`/`report_errors:`/
//! `retry_clean_cache:`/`override_channels_enabled:`/`rollback_enabled:`/
//! `solver_ignore_timestamps:`/`safety_checks:`/`path_conflict:`/
//! `shortcuts:`/`non_admin_enabled:`/`number_channel_notices:`/
//! `experimental:`/`json:`/`console:`/`error_upload_url:`/`bld_path:`/
//! `conda_build:`/`root_dir:`/`default_python:`/`enable_private_channels:`/
//! `register_envs:`/`unsatisfiable_hints:`/`unsatisfiable_hints_check_depth:`/
//! `aggressive_update_packages:`/`platform:`/`target_prefix_override:`/
//! `allow_cycles:`/`install_equivalent:`/`context:`/`data:`.
//!
//! ```rust
//! let c = "channels:\n  - conda-forge\n  - defaults\nchannel_priority: strict\nssl_verify: true\n";
//! let k = izanagi_kit::condarc::Condarc::parse(c.as_bytes()).unwrap();
//! assert_eq!(k.sections, 3);
//! assert_eq!(k.channels, 2);
//! ```

/// .condarc census.
#[derive(Debug, Clone)]
pub struct Condarc {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// `- channel` entries under `channels:`/`default_channels:`/etc.
    pub channels: usize,
    /// Nested `key: value` settings.
    pub settings: usize,
    /// Recognised .condarc option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "channels",
    "default_channels",
    "custom_channels",
    "custom_multichannels",
    "migrated_custom_channels",
    "channel_priority",
    "envs_dirs",
    "pkgs_dirs",
    "ssl_verify",
    "proxy_servers",
    "create_default_packages",
    "auto_update_conda",
    "allow_non_channel_urls",
    "restore_free_channel",
    "show_channel_urls",
    "offline",
    "use_only_tar_bz2",
    "repodata_fns",
    "repodata_threads",
    "remote_connect_timeout_secs",
    "remote_read_timeout_secs",
    "remote_max_retries",
    "remote_backoff_factor",
    "local_repodata_ttl",
    "add_pip_as_python_dependency",
    "auto_activate_base",
    "auto_stack",
    "changeps1",
    "add_anaconda_token",
    "alph",
    "clobber",
    "force",
    "force_reinstall",
    "freeze_installed",
    "update_dependencies",
    "channel_alias",
    "sat_solver",
    "solver_class",
    "deps_modifier",
    "track_features",
    "pinned_packages",
    "disallowed_packages",
    "env_prompt",
    "subdir",
    "subdirs",
    "whitelist_channels",
    "use_index_cache",
    "pip_interop_enabled",
    "notify_outdated_conda",
    "quiet",
    "verbosity",
    "report_errors",
    "retry_clean_cache",
    "override_channels_enabled",
    "rollback_enabled",
    "solver_ignore_timestamps",
    "safety_checks",
    "path_conflict",
    "shortcuts",
    "non_admin_enabled",
    "number_channel_notices",
    "experimental",
    "json",
    "console",
    "error_upload_url",
    "bld_path",
    "conda_build",
    "root_dir",
    "default_python",
    "enable_private_channels",
    "register_envs",
    "unsatisfiable_hints",
    "unsatisfiable_hints_check_depth",
    "aggressive_update_packages",
    "platform",
    "target_prefix_override",
    "allow_cycles",
    "install_equivalent",
    "context",
    "data",
    "signtool_metadata_policy",
    "client_ssl_cert",
    "client_ssl_cert_key",
    "verify_with_user",
];

/// Whether the buffer looks like a .condarc.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("channel_priority")
        || t.contains("envs_dirs:")
        || t.contains("pkgs_dirs:")
        || t.contains("auto_activate_base")
        || (t.contains("channels:")
            && (t.contains("ssl_verify")
                || t.contains("proxy_servers")
                || t.contains("create_default_packages")))
        || t.contains("repodata_fns")
        || t.contains("local_repodata_ttl")
}

impl Condarc {
    /// Parse a .condarc into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            channels: 0,
            settings: 0,
            named: 0,
        };
        let mut chan_ctx = false;
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if indent == 0 {
                chan_ctx = false;
                if let Some(colon) = s.find(':') {
                    c.sections += 1;
                    let key = &s[..colon];
                    if KEYS.contains(&key) {
                        c.named += 1;
                    }
                    if matches!(
                        key,
                        "channels"
                            | "default_channels"
                            | "envs_dirs"
                            | "pkgs_dirs"
                            | "track_features"
                            | "pinned_packages"
                            | "disallowed_packages"
                            | "whitelist_channels"
                            | "aggressive_update_packages"
                            | "repodata_fns"
                            | "subdirs"
                            | "create_default_packages"
                            | "custom_channels"
                            | "custom_multichannels"
                            | "migrated_custom_channels"
                    ) {
                        chan_ctx = true;
                    }
                }
                continue;
            }
            if chan_ctx && s.starts_with('-') {
                c.channels += 1;
                continue;
            }
            if let Some(colon) = s.find(':') {
                c.settings += 1;
                let key = s[..colon].trim().trim_start_matches('-').trim();
                if KEYS.contains(&key) {
                    c.named += 1;
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
    fn parses_condarc() {
        let b = concat!(
            "channels:\n",
            "  - conda-forge\n",
            "  - bioconda\n",
            "  - defaults\n",
            "default_channels:\n",
            "  - https://repo.anaconda.com/pkgs/main\n",
            "channel_priority: strict\n",
            "envs_dirs:\n",
            "  - ~/conda-envs\n",
            "pkgs_dirs:\n",
            "  - ~/conda-pkgs\n",
            "ssl_verify: true\n",
            "proxy_servers:\n",
            "  http: http://proxy:8080\n",
            "  https: https://proxy:8080\n",
            "auto_update_conda: false\n",
            "auto_activate_base: false\n",
            "show_channel_urls: true\n",
            "pip_interop_enabled: true\n",
            "repodata_fns:\n",
            "  - repodata.json\n",
            "  - current_repodata.json\n",
            "local_repodata_ttl: 1\n",
            "remote_connect_timeout_secs: 9\n",
            "remote_max_retries: 3\n",
            "create_default_packages:\n",
            "  - python\n",
            "  - pip\n",
        );
        let c = Condarc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 16);
        assert_eq!(c.channels, 10);
        assert_eq!(c.settings, 2);
        assert!(c.named >= 15);
    }

    #[test]
    fn rejects_other() {
        assert!(Condarc::parse(b"foo: 1").is_none());
    }
}
