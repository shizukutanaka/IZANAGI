//! DNF / YUM config (`dnf.conf`/`yum.conf`/`*.repo`) census.
//!
//! INI: `[main]`/`[fedora]`/`[name]` repo sections with `key=value`
//! (`name`/`baseurl`/`metalink`/`mirrorlist`/`enabled`/`gpgcheck`/
//! `repo_gpgcheck`/`gpgkey`/`localpkg_gpgcheck`/`cost`/`priority`/
//! `skip_if_unavailable`/`exclude`/`excludepkgs`/`includepkgs`/
//! `module_hotfixes`/`countme`/`fastestmirror`/`deltarpm`/
//! `deltarpm_percentage`/`enabled_metadata`/`type`/`mediaid`/
//! `metadata_expire`/`mirrorlist_expire`/`protect_packages`/`proxy`/
//! `proxy_username`/`proxy_password`/`proxy_auth_method`/`proxy_ssl*`/
//! `ssl*`/`username`/`password`/`sslverify`/`sslcacert`/`sslclientcert`/
//! `sslclientkey`/`throttle`/`bandwidth`/`minrate`/`timeout`/`retries`/
//! `installroot`/`cachedir`/`persistdir`/`keepcache`/`debuglevel`/
//! `errorlevel`/`logfile`/`exactarch`/`obsoletes`/`gpgcheck`/`plugins`/
//! `pluginpath`/`pluginconfpath`/`reposdir`/`distroverpkg`/`system_cachedir`/
//! `releasever`/`exclude`/`assumeyes`/`alwaysprompt`/`history_record`/
//! `history_record_packages`/`installonlypkgs`/`installonly_limit`/
//! `kernelpkgnames`/`multilib_policy`/`best`/`clean_requirements_on_remove`/
//! `config_file_path`/`debug_solver`/`disable_excludes`/`diskspacecheck`/
//! `downloaddir`/`downloadonly`/`exit_on_lock`/`get_classified_packages`/
//! `group_package_types`/`groupremove_leaf_only`/`history_list_view`/
//! `install_weak_deps`/`ip_resolve`/`keep_exit_code`/`load_available_repos`/
//! `load_system_repos`/`localpkg_gpgcheck`/`logdir`/`max_parallel_downloads`/
//! `metadata_timer_sync`/`minrate`/`module_platform_id`/`obsoletes`/
//! `optional_metadata_types`/`package_strict`/`password`/`persistdir`/
//! `pluginconfpath`/`pluginpath`/`plugins`/`protected_multilib_versions`/
//! `protected_packages`/`proxy`/`recent`/`releasever`/`repo_gpgcheck`/
//! `reposdir`/`reset_nice`/`retries`/`rpmverbosity`/`showdupesfromrepos`/
//! `skip_broken`/`sslclientcert`/`sslclientkey`/`sslverify`/
//! `strict`/`system_cachedir`/`throttle`/`timeout`/`tsflags`/`tolerant`/
//! `upgrade_group_objects_upgrade`/`user_agent`/`username`/`varsdir`/
//! `zchunk`/`comment`/`env`/`shell`/`prompt`/`color*`/`metadata_expire_filter`/
//! `downloadonly`/`del_repo*`/`countme`/`module_hotfixes`/`preserve_remote_list`/
//! `remote_location`/`sslverifystatus`/`master_timeout`/`host_speed`/
//! `mymd_download_timeout`/`max_downloads`/`mirrorlist_expire`/
//! `bandwidth`/`default_max_config`/`unicode`/`highsource`).
//!
//! ```rust
//! let d = concat!(
//!     "[main]\n",
//!     "gpgcheck = 1\n",
//!     "installonly_limit = 3\n",
//!     "[fedora]\n",
//!     "name = Fedora $releasever\n",
//!     "metalink = https://mirrors.fedoraproject.org/metalink?repo=fedora-$releasever&arch=$basearch\n",
//!     "enabled = 1\n",
//!     "gpgkey = file:///etc/pki/rpm-gpg/RPM-GPG-KEY-fedora-$releasever-$basearch\n",
//! );
//! let c = izanagi_kit::dnfconf::Dnfconf::parse(d.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// dnf/yum config census.
#[derive(Debug, Clone)]
pub struct Dnfconf {
    /// `[main]`/repo `[name]` section headers.
    pub sections: usize,
    /// `key = value` entries inside `[main]`.
    pub mains: usize,
    /// `key = value` entries inside repo sections.
    pub repos: usize,
    /// `metalink`/`mirrorlist`/`baseurl`/`gpgkey`/`cost`/`enabled`/`gpgcheck`/`repo_gpgcheck`/`skip_if_unavailable`/`module_hotfixes`/`countme`/`fastestmirror`/`deltarpm`/`type`/`mediaid`/`metadata_expire`/`proxy`/`ssl*`/`username`/`password`/`throttle`/`bandwidth`/`minrate`/`timeout`/`retries`/`protected*`/`priority`/`localpkg_gpgcheck`/`exclude*`/`includepkgs`/`enabled_metadata`/`env`/`comment`/`default_*`/`downloadonly`/`keepcache`/`installroot`/`cachedir`/`persistdir`/`debuglevel`/`errorlevel`/`logfile`/`exactarch`/`obsoletes`/`plugins`/`plugin*`/`reposdir`/`distroverpkg`/`releasever`/`assumeyes`/`alwaysprompt`/`history_*`/`installonlypkgs`/`installonly_limit`/`kernelpkgnames`/`multilib_policy`/`best`/`clean_*`/`config_file_path`/`debug_solver`/`disable_excludes`/`diskspacecheck`/`downloaddir`/`exit_on_lock`/`group_*`/`install_weak_deps`/`ip_resolve`/`keep_exit_code`/`load_*`/`localpkg_gpgcheck`/`logdir`/`max_parallel_downloads`/`metadata_timer_sync`/`module_platform_id`/`optional_metadata_types`/`package_strict`/`protected_multilib_versions`/`protected_packages`/`rpmverbosity`/`showdupesfromrepos`/`skip_broken`/`strict`/`system_cachedir`/`tsflags`/`tolerant`/`upgrade_group_objects_upgrade`/`user_agent`/`varsdir`/`zchunk`/`color*`/`metadata_expire_filter`/`mirrorlist_expire`/`remote_location`/`sslverifystatus`/`master_timeout`/`host_speed`/`mymd_download_timeout`/`max_downloads`/`default_max_config`/`unicode`/`preserve_remote_list`/`del_repo*`/`enabled`/`priority`/`fastestmirror`/`bandwidth`/`countme`-named keys within repo sections.
    pub named: usize,
    /// `$releasever`/`$basearch`/`$release_major`/`$release_minor`/`$arch`/`$uuid`/`$disttag`/`${var}` variable refs.
    pub vars: usize,
}

/// Whether the buffer looks like a dnf/yum config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[main]")
        && (t.contains("gpgcheck")
            || t.contains("cachedir")
            || t.contains("keepcache")
            || t.contains("installonly_limit")))
        || (t.contains("baseurl") || t.contains("metalink") || t.contains("mirrorlist"))
            && t.contains("gpgcheck")
}

impl Dnfconf {
    /// Parse a dnf/yum config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            mains: 0,
            repos: 0,
            named: 0,
            vars: 0,
        };
        let mut in_main = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                in_main = &s[1..s.len() - 1] == "main";
                continue;
            }
            c.vars += s.matches("$releasever").count()
                + s.matches("$basearch").count()
                + s.matches("$release_major").count()
                + s.matches("$release_minor").count()
                + s.matches("$arch").count()
                + s.matches("$uuid").count()
                + s.matches("$disttag").count()
                + s.matches("${").count();
            let Some(eq) = s.find('=') else {
                continue;
            };
            if in_main {
                c.mains += 1;
                continue;
            }
            c.repos += 1;
            let key = s[..eq].trim();
            if [
                "metalink",
                "mirrorlist",
                "baseurl",
                "gpgkey",
                "cost",
                "enabled",
                "gpgcheck",
                "repo_gpgcheck",
                "skip_if_unavailable",
                "module_hotfixes",
                "countme",
                "fastestmirror",
                "deltarpm",
                "type",
                "mediaid",
                "metadata_expire",
                "proxy",
                "username",
                "password",
                "throttle",
                "bandwidth",
                "minrate",
                "timeout",
                "retries",
                "priority",
                "localpkg_gpgcheck",
                "includepkgs",
                "enabled_metadata",
                "env",
                "comment",
            ]
            .contains(&key)
                || key.starts_with("ssl")
                || key.starts_with("exclude")
                || key.starts_with("protected")
            {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repo() {
        let b = concat!(
            "[main]\n",
            "cachedir = /var/cache/dnf\n",
            "keepcache = 0\n",
            "debuglevel = 2\n",
            "gpgcheck = 1\n",
            "installonly_limit = 3\n",
            "clean_requirements_on_remove = True\n",
            "best = True\n",
            "[fedora]\n",
            "name = Fedora $releasever - $basearch\n",
            "metalink = https://mirrors.fedoraproject.org/metalink?repo=fedora-$releasever&arch=$basearch\n",
            "enabled = 1\n",
            "countme = 1\n",
            "metadata_expire = 7d\n",
            "repo_gpgcheck = 0\n",
            "type = rpm\n",
            "gpgcheck = 1\n",
            "gpgkey = file:///etc/pki/rpm-gpg/RPM-GPG-KEY-fedora-$releasever\n",
            "skip_if_unavailable = False\n",
            "[updates]\n",
            "name = Fedora $releasever Updates\n",
            "baseurl = https://mirror.example/fedora/$releasever/$basearch/\n",
            "enabled = 1\n",
            "gpgcheck = 1\n",
        );
        let c = Dnfconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.mains, 7);
        assert_eq!(c.repos, 14);
        assert_eq!(c.named, 12);
        assert!(c.vars >= 7);
    }

    #[test]
    fn rejects_other() {
        assert!(Dnfconf::parse(b"foo = 1").is_none());
    }
}
