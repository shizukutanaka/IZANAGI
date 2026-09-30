//! Census of an `rsnapshot.conf` file.
//!
//! Tab-separated `key<TAB>value` directives: `config_version`,
//! `snapshot_root`, `cmd_*` helper paths (`cmd_cp`/`cmd_rsync`/
//! `cmd_ssh`/`cmd_logger`/`cmd_du`/`cmd_rsnapshot_diff`/`cmd_rm`/
//! `cmd_preexec`/`cmd_postexec`), `retain`, `backup`/`backup_script`/
//! `backup_exec`, `include`/`exclude`/`include_file`/`exclude_file`,
//! `link_dest`, `sync_first`, `one_fs`, `verbose`/`loglevel`/`logfile`/
//! `lockfile`, `ssh_args`/`du_args`/`rsync_*_args`, `no_create_root`.
//! `#` comments. Counts directive classes.
//!
//! ```rust
//! let c = izanagi_kit::rsnapshot::Rsnapshot::parse(
//!     b"config_version\t1.2\nsnapshot_root\t/.snapshots/\nretain\tdaily\t7\n\
//!       backup\t/home/\tlocalhost/\n",
//! ).unwrap();
//! assert_eq!(c.settings, 2);
//! assert_eq!(c.backups, 1);
//! ```
#![forbid(unsafe_code)]

/// rsnapshot.conf census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rsnapshot {
    /// `retain`/`interval` entries.
    pub retains: usize,
    /// `backup`/`backup_script`/`backup_exec` specs.
    pub backups: usize,
    /// `include`/`exclude`/`include_file`/`exclude_file` filters.
    pub filters: usize,
    /// `cmd_*` helper-command paths.
    pub cmds: usize,
    /// Other `key<TAB>value`/`key value` settings.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like rsnapshot.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("snapshot_root") || t.contains("rsnapshot"))
        && (t.contains("config_version")
            || t.contains("backup\t")
            || t.contains("backup  ")
            || t.contains("retain")
            || t.contains("interval"))
}

impl Rsnapshot {
    /// Parse rsnapshot.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            retains: 0,
            backups: 0,
            filters: 0,
            cmds: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let key = l.split_whitespace().next().unwrap_or("");
            match key {
                "retain" | "interval" => c.retains += 1,
                "backup" | "backup_script" | "backup_exec" => c.backups += 1,
                "include" | "exclude" | "include_file" | "exclude_file" => c.filters += 1,
                k if k.starts_with("cmd_") => c.cmds += 1,
                "" => {}
                _ => c.settings += 1,
            }
        }
        if c.retains + c.backups + c.settings == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# rsnapshot\n",
            "config_version\t1.2\n",
            "snapshot_root\t/.snapshots/\n",
            "cmd_cp\t/bin/cp\n",
            "cmd_rsync\t/usr/bin/rsync\n",
            "cmd_ssh\t/usr/bin/ssh\n",
            "retain\tdaily\t7\n",
            "retain\tweekly\t4\n",
            "exclude\t*.tmp\n",
            "backup\t/home/\tlocalhost/\n",
            "backup\t/etc/\tlocalhost/\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Rsnapshot::parse(b.as_bytes()).unwrap();
        assert_eq!(c.retains, 2);
        assert_eq!(c.backups, 2);
        assert_eq!(c.filters, 1);
        assert_eq!(c.cmds, 3);
        assert_eq!(c.settings, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Rsnapshot::parse(b"# none\n").is_none());
    }
}
