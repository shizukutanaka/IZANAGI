//! ccache `ccache.conf` 形式の検出と構造カウント。
//!
//! `cache_dir`/`max_size`/`compression`/`sloppiness`/`compiler_check`/
//! `run_second_cpp`/`remote_storage`/`secondary_storage`/`base_dir` 等の
//! ccache 設定オプションを `key = value` 行で識別する。
//!
//! ```
//! let b = b"cache_dir = /var/cache/ccache\nmax_size = 5G\ncompression = true\ncompiler_check = mtime\nsloppiness = file_macro,time_macros\n";
//! assert!(izanagi_kit::ccache::detect(b));
//! let c = izanagi_kit::ccache::Ccache::parse(b).unwrap();
//! assert_eq!(c.keys, 5);
//! ```

/// Parsed ccache.conf summary.
#[derive(Debug, Clone)]
pub struct Ccache {
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// ccache option names.
const KEYS: &[&str] = &[
    "absolute_paths_in_stderr",
    "base_dir",
    "cache_dir",
    "compiler",
    "compiler_check",
    "compiler_type",
    "compression",
    "compression_level",
    "config_path",
    "cpp_extension",
    "debug",
    "debug_dir",
    "debug_level",
    "default_compiler_type",
    "depend_mode",
    "direct_mode",
    "disable",
    "extra_files_to_hash",
    "file_clone",
    "hard_link",
    "hash_dir",
    "ignore_options",
    "inode_cache",
    "keep_comments_cpp",
    "log_file",
    "max_files",
    "max_size",
    "msvc_dep_prefix",
    "namespace",
    "path",
    "pch_external_checksum",
    "prefix_command",
    "prefix_command_cpp",
    "read_only",
    "read_only_direct",
    "recache",
    "remote_local_storage",
    "remote_only",
    "remote_storage",
    "reshare",
    "run_second_cpp",
    "secondary_storage",
    "sloppiness",
    "stats",
    "stats_log",
    "stats_zero",
    "temporary_dir",
    "umask",
];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#') && tr.split('=').next().is_some_and(|p| p.trim() == k)
    })
}

/// Detect a ccache config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| key_present(t, k)).count() >= 3
}

impl Ccache {
    /// Count categories. Returns `None` when the input does not look like
    /// a ccache config.
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
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
                if let Some(k) = tr.split('=').next() {
                    if KEYS.contains(&k.trim()) {
                        c.keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`Ccache::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ccache> {
    Ccache::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# ccache\ncache_dir = /var/cache/ccache\nmax_size = 10G\ncompression = true\ncompression_level = 6\ncompiler_check = content\nsloppiness = file_macro,time_macros,include_file_mtime\nbase_dir = /home/user\ndepend_mode = true\ndirect_mode = true\nremote_storage = redis://redis:6379\nrun_second_cpp = false\nstats = true\numask = 002\n";
        assert!(detect(b));
        let c = Ccache::parse(b).unwrap();
        assert_eq!(c.keys, 13);
        assert_eq!(c.assignments, 13);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_ini() {
        assert!(!detect(b"host = x\nport = 1\nuser = y\n"));
        assert!(!detect(b"[section]\nkey = v\n"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"# cache_dir = /x\n# max_size = 5G\n# compression = true\n"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(Ccache::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"cache_dir");
        assert!(!detect(&b));
    }
}
