//! Rootkit Hunter `rkhunter.conf` の検出と構造カウント。
//!
//! `KEY=VAL`/`KEY VAL` 形の設定行、`ALLOW_*`/`*_WHITELIST`/`DISABLE_TESTS`/
//! `ENABLE_TESTS`/`SCRIPTWHITELIST`/`RTKT_*`/`PKGMGR` 等の既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::rkhunter::parse(
//!     b"ROTATE_MIRRORS=1\nUPDATE_MIRRORS=1\nMIRRORS_MODE=0\nDISABLE_TESTS=suspscan hidden_procs deleted_files packet_cap_apps apps\nENABLE_TESTS=all\nALLOW_SSH_ROOT_USER=no\n").unwrap();
//! assert!(c.options >= 5);
//! assert!(izanagi_kit::rkhunter::detect(
//!     b"ALLOW_SSH_ROOT_USER=no\nDISABLE_TESTS=apps\nENABLE_TESTS=all\n"));
//! ```

/// rkhunter.conf 既知キー。
const KEYS: &[&str] = &[
    "ALLOWDEVFILE",
    "ALLOWHIDDENDIR",
    "ALLOWHIDDENFILE",
    "ALLOWIPCPROC",
    "ALLOWIPCDIR",
    "ALLOWPROMISCIF",
    "ALLOW_SSH_PROT_V1",
    "ALLOW_SSH_ROOT_USER",
    "ALLOW_SYSLOG_REMOTE_LOGGING",
    "APP_WHITELIST",
    "ARCHIVE_DIR",
    "ATTRWHITELIST",
    "AUTO_X_DETECT",
    "BINDIR",
    "COPY_LOG_ON_ERROR",
    "DBDIR",
    "DISABLE_TESTS",
    "DISABLE_UNHIDE",
    "ENABLE_TESTS",
    "EPOLL_SEG_SIZE",
    "ETCDIR",
    "EXISTWHITELIST",
    "IGNORE_PRELINK_DEP_WARNINGS",
    "IMMUTABLE_SET",
    "IMMUTABLE_WHITELIST",
    "INET6",
    "INSTALLDIR",
    "IPC_WHITELIST",
    "LANGUAGE",
    "LOCKDIR",
    "LOGFILE",
    "MAIL-ON-WARNING",
    "MIRRORS_MODE",
    "OS_VERSION_FILE",
    "PASSWORD_FILE",
    "PATH",
    "PHALANX2_DIR_WHITELIST",
    "PIDFILE",
    "PKGMGR",
    "PKGMGR_NO_VRFY",
    "PORT_WHITELIST",
    "PORT_PATH_WHITELIST",
    "PWD_LOCK",
    "ROTATE_MIRRORS",
    "RTKT_DIR_WHITELIST",
    "RTKT_FILE_WHITELIST",
    "SCAN_MODE_DEV",
    "SCRIPTDIR",
    "SCRIPTWHITELIST",
    "SHARED_LIB_WHITELIST",
    "SHARED_LIB_WARN_ON_MISSING",
    "SHOW_LOCK_MSGS",
    "SHOW_SUMMARY",
    "SIGHUP_JOB_RUNNING",
    "STARTUP_PATHS",
    "SUSPSCAN_DIRS",
    "SUSPSCAN_MAXSIZE",
    "SUSPSCAN_PATHS",
    "SUSPSCAN_TEMP",
    "SUSPSCAN_WHITELIST",
    "SYSLOG_PRIORITY",
    "TMPDIR",
    "UIPC_PATHS",
    "UPDATE_MIRRORS",
    "USE_LOCKING",
    "USE_SUNSUM",
    "USE_UNHIDE",
    "USER_FILEPROP_FILES_DIRS",
    "WEB_CMD",
    "WHITELISTED_IS_WHITE",
    "XINETD_CONF_PATH",
    "XINETD_ALLOWED_SVC",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn known_key(t: &str) -> bool {
    let end = t
        .find(|c: char| c == '=' || c.is_whitespace())
        .unwrap_or(t.len());
    let k = t[..end].trim();
    KEYS.contains(&k) || k.ends_with("_WHITELIST")
}

/// `rkhunter.conf` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if known_key(t) {
            hits += 1;
            if hits >= 3 {
                return true;
            }
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
    let mut c = Counts {
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
        if known_key(t) {
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

    const SAMPLE: &[u8] = b"# rkhunter.conf\nROTATE_MIRRORS=1\nUPDATE_MIRRORS=1\nMIRRORS_MODE=0\nWEB_CMD=\"/bin/false\"\nPKGMGR=DPKG\nPKGMGR_NO_VRFY=\"\"\nDISABLE_TESTS=\"suspscan hidden_procs deleted_files packet_cap_apps apps\"\nENABLE_TESTS=all\nALLOW_SSH_ROOT_USER=no\nALLOW_SSH_PROT_V1=0\nSCRIPTWHITELIST=/usr/bin/egrep\nSCRIPTWHITELIST=/usr/bin/fgrep\nRTKT_FILE_WHITELIST=/etc/.java\nSCAN_MODE_DEV=THOROUGH\n";

    #[test]
    fn rkhunter() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 14);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_rkhunter() {
        assert!(!detect(b"KEY=VAL\n"));
        assert!(parse(b"text\n").is_none());
    }
}
