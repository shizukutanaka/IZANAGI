//! Icecream `icecc.conf`/`ICECC_*` 環境設定の検出と構造カウント。
//!
//! `ICECC_NETNAME`/`ICECC_SCHEDULER`/`ICECC_MAX_JOBS`/
//! `ICECC_VERSION`/`ICECC_REMOTE_CPP`/`ICECC_CLANG_REMOTE_CPP`/
//! `ICECC_TEST_ENV`/`ICECC_CARET_WORKAROUND` 等の `ICECC_*`/`ICECREAM_*`
//! `KEY=value` 行を識別する。
//!
//! ```
//! let b = b"ICECC_NETNAME=cluster1\nICECC_SCHEDULER=192.168.1.5\nICECC_MAX_JOBS=32\n";
//! assert!(izanagi_kit::iceccconf::detect(b));
//! let c = izanagi_kit::iceccconf::IceccConf::parse(b).unwrap();
//! assert_eq!(c.keys, 3);
//! ```

/// Parsed icecream config summary.
#[derive(Debug, Clone)]
pub struct IceccConf {
    /// `ICECC_*`/`ICECREAM_*` keys.
    pub keys: usize,
    /// `key=value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Icecream variable names.
const KEYS: &[&str] = &[
    "ICECC_ALLOW_IN_TESTS",
    "ICECC_CARET_WORKAROUND",
    "ICECC_CLANG_REMOTE_CPP",
    "ICECC_CLANG_REMOTEBUILD",
    "ICECC_COLOR_DIAGNOSTICS",
    "ICECC_COMPILE_UNREACHABLE",
    "ICECC_COMPRESSED_DEBUG",
    "ICECC_CREATE_X",
    "ICECC_DEBUG",
    "ICECC_EXTRAFILES",
    "ICECC_LOGFILE",
    "ICECC_MAX_JOBS",
    "ICECC_NETNAME",
    "ICECC_NO_UNICORN",
    "ICECC_PREFERRED_HOST",
    "ICECC_REMOTE_CPP",
    "ICECC_REPEAT_RATE",
    "ICECC_RUN",
    "ICECC_SCHEDULER",
    "ICECC_SCHEDULER_PORT",
    "ICECC_TEST_ENV",
    "ICECC_TOML",
    "ICECC_VERBOSE",
    "ICECC_VERSION",
    "ICECC_WAIT_FOR_INVOKER",
    "ICECC_WHICH",
    "ICECREAM_CONFIG",
    "ICECREAM_LOG_FILE",
    "ICECREAM_LOG_FILE_PREFIX",
    "ICECREAM_MAX_JOBS",
    "ICECREAM_NETNAME",
    "ICECREAM_SCHEDULER",
    "ICECREAM_TEST_ADD_HEADER",
    "ICECREAM_TEST_ADD_MACRO",
    "ICECREAM_TEST_APPEND_HEADER",
    "ICECREAM_TEST_COMPILE_LINUX",
    "ICECREAM_TEST_OUTPUT",
    "ICECREAM_TEST_REMOVED",
    "ICECREAM_TEST_RATE",
    "ICECREAM_TEST_SHIM",
    "ICECREAM_TEST_SHORT_LOG",
    "ICECREAM_TEST_SUB_MESSAGE",
    "ICECREAM_TEST_WATCH_HEADER",
];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#') && (tr.starts_with(k) && tr[k.len()..].trim_start().starts_with('='))
    })
}

/// Detect an icecream config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| key_present(t, k)).count() >= 2
}

impl IceccConf {
    /// Count categories. Returns `None` when the input does not look like
    /// an icecream config.
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

/// Convenience wrapper around [`IceccConf::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<IceccConf> {
    IceccConf::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# icecream daemon\nICECC_NETNAME=cluster1\nICECC_SCHEDULER=192.168.1.5\nICECC_MAX_JOBS=32\nICECC_VERSION=/opt/icecream/gcc12.tar.gz\nICECC_REMOTE_CPP=1\nICECC_CLANG_REMOTE_CPP=1\nICECC_TEST_ENV=/tmp/env.tar.gz\nICECC_VERBOSE=1\n";
        assert!(detect(b));
        let c = IceccConf::parse(b).unwrap();
        assert_eq!(c.keys, 8);
        assert_eq!(c.assignments, 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"ICEC=x\nFOO_BAR=2\n"));
        assert!(!detect(b"export PATH=/bin\n"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(b"# ICECC_NETNAME=x\n# ICECC_MAX_JOBS=8\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(IceccConf::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"ICECC_NETNAME");
        assert!(!detect(&b));
    }
}
