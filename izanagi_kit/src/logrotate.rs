//! logrotate 設定ファイル (`logrotate.conf` / `logrotate.d/*`) の解析。
//!
//! `/var/log/syslog { … }` ブロックと `rotate`/`daily`/`size`/`prerotate` 系
//! ディレクティブの形を持つ設定を検出し、ブロック数・ディレクティブ数・
//! スクリプトブロック数などを整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::logrotate;
//!
//! let text = br#"/var/log/syslog {
//!     weekly
//!     rotate 4
//!     missingok
//!     postrotate
//!         /usr/bin/killall -HUP syslogd
//!     endscript
//! }
//! "#;
//!
//! assert!(logrotate::detect(text));
//! let c = logrotate::parse(text).unwrap();
//! assert_eq!(c.blocks, 1);
//! assert_eq!(c.directives, 4);
//! assert_eq!(c.script_blocks, 1);
//! ```

/// 既知の logrotate ディレクティブ(スクリプトを伴わないもの)。
const KNOWN_DIRECTIVES: &[&str] = &[
    "daily",
    "weekly",
    "monthly",
    "yearly",
    "rotate",
    "size",
    "minsize",
    "maxsize",
    "missingok",
    "nomissingok",
    "ifempty",
    "notifempty",
    "olddir",
    "noolddir",
    "copytruncate",
    "nocopytruncate",
    "compress",
    "nocompress",
    "compresscmd",
    "uncompresscmd",
    "compressext",
    "compressoptions",
    "delaycompress",
    "nodelaycompress",
    "create",
    "createolddir",
    "nocreate",
    "nocreateolddir",
    "copy",
    "nocopy",
    "mail",
    "nomail",
    "mailfirst",
    "mailthen",
    "mailwo",
    "maillast",
    "sharedscripts",
    "nosharedscripts",
    "dateext",
    "nodateext",
    "dateformat",
    "dateyesterday",
    "datehourago",
    "extension",
    "tabooext",
    "taboopat",
    "include",
    "shred",
    "shredcycles",
    "su",
    "allowhardlink",
    "noallowhardlink",
    "renamecopy",
    "norenamecopy",
    "rotateitem",
];

/// スクリプトブロックを開始するディレクティブ。
const SCRIPT_STARTERS: &[&str] = &[
    "prerotate",
    "postrotate",
    "firstaction",
    "lastaction",
    "preremove",
];

/// logrotate ブロックの集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `path {` で始まるログファイルブロック数。
    pub blocks: usize,
    /// ディレクティブ行(スクリプト開始を含む)の総数。
    pub directives: usize,
    /// 既知ディレクティブ数。
    pub known_directives: usize,
    /// `prerotate`/`postrotate`/`firstaction`/`lastaction`/`preremove` ブロック数。
    pub script_blocks: usize,
    /// `rotate N` 等の数値を取るディレクティブ数。
    pub numeric_directives: usize,
}

/// `b` が logrotate 設定ファイルらしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.blocks >= 1 && c.known_directives >= 2) || c.known_directives >= 4
}

/// logrotate 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        blocks: 0,
        directives: 0,
        known_directives: 0,
        script_blocks: 0,
        numeric_directives: 0,
    };
    let mut in_block = false;
    let mut in_script = false;
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if in_script {
            if line == "endscript" || line.starts_with("endscript") {
                in_script = false;
            }
            continue;
        }
        if let Some(head_raw) = line.strip_suffix('{') {
            let head = head_raw.trim();
            let first = head.split_whitespace().next().unwrap_or("");
            if first.starts_with('/') || first.starts_with('"') || first.contains('*') {
                counts.blocks += 1;
                in_block = true;
                saw_any = true;
            }
            continue;
        }
        if line.starts_with('}') {
            in_block = false;
            continue;
        }
        let word = line.split_whitespace().next().unwrap_or("");
        if word.is_empty() || !word.bytes().all(|c| c.is_ascii_alphabetic() || c == b'_') {
            continue;
        }
        let lower = word.to_ascii_lowercase();
        if lower == "endscript" {
            continue;
        }
        if SCRIPT_STARTERS.contains(&lower.as_str()) {
            counts.script_blocks += 1;
            in_script = true;
        }
        let _ = in_block;
        counts.directives += 1;
        saw_any = true;
        if KNOWN_DIRECTIVES.contains(&lower.as_str()) {
            counts.known_directives += 1;
        }
        let rest = line[word.len()..].trim();
        if rest.bytes().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            counts.numeric_directives += 1;
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"# logrotate.conf
weekly
rotate 4
create
compress

/var/log/wtmp {
    monthly
    rotate 5
    missingok
}

/var/log/syslog {
    weekly
    rotate 4
    missingok
    sharedscripts
    postrotate
        /usr/bin/killall -HUP syslogd
    endscript
}
"#;

    #[test]
    fn detects_logrotate() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.blocks, 2);
        assert_eq!(c.directives, 12);
        assert_eq!(c.known_directives, 11);
        assert_eq!(c.script_blocks, 1);
        assert_eq!(c.numeric_directives, 3);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"key = value"));
        assert!(parse(b"hello").is_some());
    }
}
