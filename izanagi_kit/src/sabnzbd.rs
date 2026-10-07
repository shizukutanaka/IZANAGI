//! `sabnzbd.ini` 検出モジュール。
//!
//! SABnzbd の設定は INI 風で、`[misc]`/`[folders]`/`[servers]`/
//! `[categories]`/`[script]`/`[logging]` セクション、`[servers]`
//! 配下の `[[name]]` サブテーブル、および `host`/`port`/`username`/
//! `password`/`api_key`/`nzb_key`/`admin_dir`/`download_dir`/
//! `complete_dir`/`incomplete_dir`/`log_dir`/`cache_dir`/`https_port`/
//! `enable_https`/`bandwidth_max`/`bandwidth_perc`/`permissions`/
//! `top_only`/`script`/`priority`/`pp`/`unpack_check`/`quota`/
//! `speedlimit`/`rating_enable`/`cleanup_list`/`ignore_samples`/
//! `pre_check`/`fail_hopeless_jobs`/`par_option`/`nice`/`ionice`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"[misc]\n\
//!           host = 0.0.0.0\n\
//!           port = 8080\n\
//!           api_key = abc123\n\
//!           [folders]\n\
//!           download_dir = /downloads/incomplete\n\
//!           complete_dir = /downloads/complete\n";
//! let c = izanagi_kit::sabnzbd::parse(b);
//! assert!(izanagi_kit::sabnzbd::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "[misc]",
    "[folders]",
    "[servers]",
    "[categories]",
    "[script]",
    "[logging]",
    "[growl]",
    "[ncenter]",
    "[acenter]",
    "[prowl]",
    "[pushover]",
    "[pushbullet]",
    "[nscript]",
    "[sorters]",
    "[rss]",
    "[newzbin]",
];

const KEYS: &[&str] = &[
    "admin_dir",
    "allow_64bit_tools",
    "ampm",
    "api_key",
    "auto_disconnect",
    "auto_sort",
    "bandwidth_max",
    "bandwidth_perc",
    "cache_dir",
    "categories",
    "check_free_space",
    "cleanup_list",
    "complete_dir",
    "complete_free",
    "config_lock",
    "dirscan_dir",
    "dirscan_priority",
    "dirscan_script",
    "dirscan_speed",
    "disable_api_key",
    "download_dir",
    "download_free",
    "email_account",
    "email_cats",
    "email_dir",
    "email_endjob",
    "email_full",
    "email_from",
    "email_pwd",
    "email_rss",
    "email_server",
    "email_to",
    "empty_cat",
    "enable_https",
    "fail_hopeless_jobs",
    "fail_on_crc",
    "fix_associations",
    "folder_delete",
    "folder_max_length",
    "help_button",
    "host",
    "host_whitelist",
    "https_chain",
    "https_key",
    "https_port",
    "ignore_samples",
    "ignore_wrong_unrar",
    "incomplete_dir",
    "ionice",
    "ipv6_servers",
    "language",
    "local_range",
    "log_dir",
    "login_url",
    "max_art_opt",
    "max_art_tries",
    "max_url_retries",
    "movie_rename_limit",
    "new_nzb_on_failure",
    "nice",
    "no_dupes",
    "notified_new_skin",
    "nzb_backup_dir",
    "nzb_key",
    "osx_menu",
    "osx_speed",
    "overwrite_files",
    "par_option",
    "password",
    "pause_on_pwrar",
    "permissions",
    "port",
    "pre_check",
    "pre_script",
    "prio_sort_list",
    "queue_complete",
    "queue_limit",
    "quick_check",
    "quota_day",
    "quota_period",
    "quota_size",
    "rating_enable",
    "refresh_rate",
    "reject_duplicate",
    "req_rate",
    "require_modern_tls",
    "rss_filenames",
    "rss_rate",
    "safe_postproc",
    "script_can_fail",
    "send_group",
    "series_rename",
    "show_sysload",
    "speedlimit",
    "ssl_type",
    "start_paused",
    "top_only",
    "tv_sort_string",
    "uniconfig",
    "unpack_check",
    "unpack_fail",
    "use_pickle",
    "username",
    "wait_for_dfolder",
    "warn_empty_nzb",
    "warn_dupl_jobs",
    "web_color",
    "web_watchdog",
];

fn is_section(t: &str) -> bool {
    SECTIONS.contains(&t) || (t.starts_with("[[") && t.ends_with("]]"))
}

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が sabnzbd.ini に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if is_section(tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 4 || (secs >= 2 && keys >= 1)
}

/// sabnzbd.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct Sabnzbd {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を sabnzbd.ini として統計する。
pub fn parse(b: &[u8]) -> Sabnzbd {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Sabnzbd::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if is_section(tr) {
            c.sections += 1;
        } else if key_present(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[misc]\nhost = 0.0.0.0\nport = 8080\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn detects_keys() {
        let b = b"api_key = a\nnzb_key = b\ndownload_dir = c\ncomplete_dir = d\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[misc]\n"));
        assert!(!detect(b"[misc]\nkey = x\n"));
        assert!(!detect(b"host = a\nport = b\napi_key = c\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
