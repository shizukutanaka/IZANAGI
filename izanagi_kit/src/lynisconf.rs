//! Lynis `default.prf`/`custom.prf` プロファイルの検出と構造カウント。
//!
//! `config:key=value` 形の設定と、`test:XXXX-0000`/`skip-test=XXXX-0000`/
//! `option group=test` 等の Lynis 固有ディレクティブを識別する。
//!
//! ```
//! let c = izanagi_kit::lynisconf::parse(
//!     b"config:kernel=yes\nconfig:firewall=yes\nskip-test=AUTH-9328\nskip-test=HTTP-6702\ntest:FILE-6310\n").unwrap();
//! assert!(c.options >= 4);
//! assert!(izanagi_kit::lynisconf::detect(
//!     b"config:kernel=yes\nskip-test=AUTH-9328\ntest:FILE-6310\n"));
//! ```

/// `key=value` 形で使われる既定キー(プレフィックス含む)。
const KEYS: &[&str] = &[
    "allow-syslog-remote-logging",
    "check-update",
    "compact",
    "compliance_standards",
    "config",
    "cronjob",
    "debug",
    "error-on-warnings",
    "import",
    "logfile",
    "mail",
    "non-interactive",
    "option",
    "profile",
    "quick",
    "report",
    "report-file",
    "scanner-ip",
    "self-test",
    "self-test-data",
    "server",
    "skip-test",
    "test",
    "upload",
    "use-colors",
    "verbose",
    "warnings",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知 `key=value`/`config:k=v`/テスト行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn known_line(t: &str) -> bool {
    // `config:key=value`、`test:ID`、`skip-test=ID`、`key=value`、`option group=test`。
    if t.starts_with("config:") && t[7..].contains('=') {
        return true;
    }
    if t.starts_with("test:") || t.starts_with("option ") {
        return true;
    }
    let Some(eq) = t.find('=') else { return false };
    let k = t[..eq].trim();
    KEYS.contains(&k) || k.starts_with("config:") || k.starts_with("test:")
}

/// `custom.prf` らしさを判定する。
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
        if known_line(t) {
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
        if known_line(t) {
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

    const SAMPLE: &[u8] = b"# custom.prf\nconfig:kernel=yes\nconfig:firewall=yes\nconfig:usb_storage=yes\ndebug=no\nnon-interactive=yes\nskip-test=AUTH-9328\nskip-test=HTTP-6702\ntest:FILE-6310\noption group=Debian\nuse-colors=yes\n";

    #[test]
    fn lynisconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 10);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_lynisconf() {
        assert!(!detect(b"KEY=VAL\n"));
        assert!(parse(b"text\n").is_none());
    }
}
