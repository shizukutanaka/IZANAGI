//! Goma `GOMA_*` 環境設定（compiler_proxy 制御）の検出と構造カウント。
//!
//! `GOMA_SERVER_HOST`/`GOMA_USE_SSL`/`GOMA_DEPS_CACHE_FILE`/
//! `GOMA_HERMETIC`/`GOMA_LOCAL_OUTPUT_CACHE`/`GOMA_MAX_SUBPROCS`/
//! `GOMA_COMPILER_PROXY_ENABLE_CRASH_DUMP` 等の `GOMA_*` `KEY=value`
//! 行を識別する。
//!
//! ```
//! let b = b"GOMA_SERVER_HOST=goma.example.com\nGOMA_USE_SSL=true\nGOMA_DEPS_CACHE_FILE=/tmp/goma.deps\n";
//! assert!(izanagi_kit::gomaconf::detect(b));
//! let c = izanagi_kit::gomaconf::GomaConf::parse(b).unwrap();
//! assert_eq!(c.keys, 3);
//! ```

/// Parsed Goma config summary.
#[derive(Debug, Clone)]
pub struct GomaConf {
    /// `GOMA_*` keys.
    pub keys: usize,
    /// `key=value` lines.
    pub assignments: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

/// Goma variable names.
const KEYS: &[&str] = &[
    "GOMA_API_KEY_FILE",
    "GOMA_ARBITRARY_TOOLCHAIN_SUPPORT",
    "GOMA_BACKOFF",
    "GOMA_CACHE_DIR",
    "GOMA_CACHE_SIZE",
    "GOMA_COFETCH",
    "GOMA_COMPILER_PROXY_ENABLE_CRASH_DUMP",
    "GOMA_COMPILER_PROXY_ENABLE_FINER_SCHEDULING",
    "GOMA_COMPILER_PROXY_ENABLE_RECORD_LIST_FILE",
    "GOMA_COMPILER_PROXY_ENABLE_SEND_INFORMATION",
    "GOMA_COMPILER_PROXY_FAIL",
    "GOMA_COMPILER_PROXY_FORCE_KILL",
    "GOMA_COMPILER_PROXY_IGNORE_BASE_INCLUDE_PATHS",
    "GOMA_COMPILER_PROXY_IN_BACKOFF",
    "GOMA_COMPILER_PROXY_INITIALIZED",
    "GOMA_COMPILER_PROXY_KNOWN_LOCATIONS",
    "GOMA_COMPILER_PROXY_PORT",
    "GOMA_COMPILER_PROXY_RUN_NAME",
    "GOMA_CRASHDUMP_FILE",
    "GOMA_CRASHREPORTING_DISABLE",
    "GOMA_DEPS_CACHE_AUTO_EVICT",
    "GOMA_DEPS_CACHE_FILE",
    "GOMA_DEPS_CACHE_PROBABILITY",
    "GOMA_DEPS_CACHE_TABLE_THRESHOLD",
    "GOMA_DEPS_CACHE_TABLE_VERSION",
    "GOMA_DUMP",
    "GOMA_DUMP_CRASH",
    "GOMA_ENABLE_RBE_EXECUTION",
    "GOMA_ENABLE_SUBPROC_KILLER",
    "GOMA_FETCH_INFO",
    "GOMA_FORCE_SUBPROC",
    "GOMA_GCE_INSTANCE",
    "GOMA_GET_REQUESTID",
    "GOMA_HERMETIC",
    "GOMA_HTTP_HOST_PREFIX",
    "GOMA_LOCAL_OUTPUT_CACHE_DIR",
    "GOMA_LOCAL_OUTPUT_CACHE_ENABLE_CACHE",
    "GOMA_LOCAL_OUTPUT_CACHE_MAX_CACHE_AMOUNT_IN_MB",
    "GOMA_LOCAL_OUTPUT_CACHE_PERCENTAGE_CACHE_AMOUNT",
    "GOMA_LOCAL_OUTPUT_CACHE_THRESHOLD_ITEMS",
    "GOMA_LOCAL_RUN_MODE",
    "GOMA_MAX_SUBPROCS",
    "GOMA_MAX_SUBPROCS_LOW",
    "GOMA_MAX_SUBPROCS_WEIGHT",
    "GOMA_MIRROR_NETWORK",
    "GOMA_NEED_SUBPROCS",
    "GOMA_NETWORK_WATCHER",
    "GOMA_REQUESTID_PREFIX",
    "GOMA_RESOURCE_OWNER",
    "GOMA_RETRY",
    "GOMA_SEND_USER_INFO",
    "GOMA_SERVER_HOST",
    "GOMA_SERVER_PORT",
    "GOMA_SERVICE_ACCOUNT_JSON_FILENAME",
    "GOMA_SHARD",
    "GOMA_STORE_TARGET_MACHINE_DATA",
    "GOMA_STORE_USER_INFO",
    "GOMA_STRICT_MODE",
    "GOMA_STUBBY_MIN_MESSAGE_SIZE",
    "GOMA_UNTRUSTED",
    "GOMA_USE_SSL",
    "GOMA_USE_TMP",
    "GOMA_VERBOSE",
    "GOMA_VERIFY_PRODUCT_NAME",
    "GOMA_WONT_SUBPROCS_FOR_UNRECOGNIZED_COMMANDS",
    "GOMAGET_ALTERNATIVE_USER",
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

/// Detect a Goma config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| key_present(t, k)).count() >= 3
}

impl GomaConf {
    /// Count categories. Returns `None` when the input does not look like
    /// a Goma config.
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

/// Convenience wrapper around [`GomaConf::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<GomaConf> {
    GomaConf::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# goma client\nGOMA_SERVER_HOST=goma.example.com\nGOMA_SERVER_PORT=5050\nGOMA_USE_SSL=true\nGOMA_HERMETIC=false\nGOMA_DEPS_CACHE_FILE=/tmp/goma.deps\nGOMA_DEPS_CACHE_PROBABILITY=100\nGOMA_LOCAL_OUTPUT_CACHE_ENABLE_CACHE=true\nGOMA_MAX_SUBPROCS=16\nGOMA_SEND_USER_INFO=true\nGOMA_SERVICE_ACCOUNT_JSON_FILENAME=/keys/goma.json\nGOMA_VERBOSE=1\n";
        assert!(detect(b));
        let c = GomaConf::parse(b).unwrap();
        assert_eq!(c.keys, 11);
        assert_eq!(c.assignments, 11);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"GOMAC=x\nFOO=1\nBAR=2\n"));
        assert!(!detect(b"export LANG=C\n"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"# GOMA_SERVER_HOST=x\n# GOMA_USE_SSL=true\n# GOMA_MAX_SUBPROCS=4\n"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(GomaConf::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"GOMA_SERVER_HOST");
        assert!(!detect(&b));
    }
}
