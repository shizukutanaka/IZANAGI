//! Census of a `btrbk.conf` file.
//!
//! Whitespace `key value` directives in a hierarchy:
//! `volume <path>` → `subvolume <path>` → `target <path>`/`ssh_url`,
//! plus `snapshot_dir`/`snapshot_name`/`snapshot_create`,
//! `incremental`, `timestamp_format`, `retention` via
//! `snapshot_preserve*`/`target_preserve*`/`archive_preserve*`,
//! `stream_compress*`/`stream_io_limit`/`rate_limit`,
//! `backend`, `btrfs_path`, `transaction_log`, `dry_run`.
//! `#` comments. Counts volume/subvolume/target blocks.
//!
//! ```rust
//! let c = izanagi_kit::btrbk::Btrbk::parse(
//!     b"volume /mnt/btrfs\n  subvolume home\n    target /mnt/backup\n",
//! ).unwrap();
//! assert_eq!(c.volumes, 1);
//! assert_eq!(c.subvolumes, 1);
//! assert_eq!(c.targets, 1);
//! ```
#![forbid(unsafe_code)]

/// btrbk.conf census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Btrbk {
    /// `volume <path>` blocks.
    pub volumes: usize,
    /// `subvolume <path>` blocks.
    pub subvolumes: usize,
    /// `target <path>`/`target ssh://`/`ssh_url` entries.
    pub targets: usize,
    /// `*_preserve*` retention keys.
    pub preserves: usize,
    /// `stream_*`/`rate_limit`/`send_min`/`send_max` transfer keys.
    pub streams: usize,
    /// Other `key value` settings.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// `key value` keys that count as preserves.
const PRESERVES: &[&str] = &[
    "snapshot_preserve",
    "snapshot_preserve_min",
    "target_preserve",
    "target_preserve_min",
    "archive_preserve",
    "archive_preserve_min",
    "snapshot_preserve_daily",
    "snapshot_preserve_weekly",
    "snapshot_preserve_monthly",
    "target_preserve_daily",
    "target_preserve_weekly",
    "target_preserve_monthly",
];

/// Stream/transfer keys.
const STREAMS: &[&str] = &[
    "stream_compress",
    "stream_compress_level",
    "stream_compress_threads",
    "stream_io_limit",
    "stream_buffer",
    "rate_limit",
    "send_min",
    "send_max",
    "send_receive_delay",
    "raw_target_blocksize",
    "raw_target_split",
];

/// True if `b` looks like btrbk.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("subvolume") && t.contains("volume")) || t.contains("snapshot_preserve")
}

impl Btrbk {
    /// Parse btrbk.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            volumes: 0,
            subvolumes: 0,
            targets: 0,
            preserves: 0,
            streams: 0,
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
                "volume" => c.volumes += 1,
                "subvolume" => c.subvolumes += 1,
                "target" | "ssh_url" | "target_type" => c.targets += 1,
                k if PRESERVES.contains(&k) => c.preserves += 1,
                k if STREAMS.contains(&k) => c.streams += 1,
                "" => {}
                _ => c.settings += 1,
            }
        }
        if c.volumes == 0 && c.subvolumes == 0 {
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
            "# btrbk\n",
            "volume /mnt/btrfs\n",
            "  subvolume home\n",
            "    snapshot_name @home\n",
            "    target /mnt/backup/home\n",
            "    target_preserve_min 2\n",
            "    target_preserve 14d 9w *m\n",
            "  subvolume root\n",
            "    target ssh://nas/backups\n",
            "  stream_compress xz\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Btrbk::parse(b.as_bytes()).unwrap();
        assert_eq!(c.volumes, 1);
        assert_eq!(c.subvolumes, 2);
        assert_eq!(c.targets, 2);
        assert_eq!(c.preserves, 2);
        assert_eq!(c.streams, 1);
        assert_eq!(c.settings, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Btrbk::parse(b"# none\n").is_none());
    }
}
