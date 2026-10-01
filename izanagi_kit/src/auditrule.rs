//! audit ルールファイル (`audit.rules` / `auditctl` ルール) の解析。
//!
//! `-w <path>` ウォッチ、`-a list,action`/`/path` システムコールルール、
//! `-S <syscall>`、`-F field=val`、`-D`/`-b`/`-e`/`-f`/`-i` 系制御フラグの
//! 形を持つ設定を検出し、ルール種別ごとの数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::auditrule;
//!
//! let text = br#"-D
//! -b 8192
//! -w /etc/passwd -p wa -k passwd_changes
//! -a always,exit -F arch=b64 -S open -k file_access
//! -e 2
//! "#;
//!
//! assert!(auditrule::detect(text));
//! let c = auditrule::parse(text).unwrap();
//! assert_eq!(c.watch_rules, 1);
//! assert_eq!(c.syscall_rules, 1);
//! assert_eq!(c.control_flags, 3);
//! ```

/// auditrule ルールの集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `-w`/`/path` によるウォッチルール数。
    pub watch_rules: usize,
    /// `-a`/`-A` で始まるシステムコールルール数。
    pub syscall_rules: usize,
    /// `-S <syscall>` 指定の総数。
    pub syscalls: usize,
    /// `-F <field>` フィルタ指定の総数。
    pub filters: usize,
    /// `-k <key>` キー付きルール数。
    pub keyed_rules: usize,
    /// `-D`/`-b`/`-e`/`-f`/`-i`/`-l`/`-r`/`-s`/`-v`/`-c`/`-d` 制御行数。
    pub control_flags: usize,
}

/// `b` が audit ルールファイルらしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    let rules = c.watch_rules + c.syscall_rules;
    rules >= 2 || (rules >= 1 && c.control_flags >= 2)
}

fn is_path_rule(line: &str) -> bool {
    // auditctl のファイルシステムパス指定 `/path -p ...` 記法。
    line.starts_with('/') && !line.contains("..") && line.contains(" -")
}

/// audit ルールを解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        watch_rules: 0,
        syscall_rules: 0,
        syscalls: 0,
        filters: 0,
        keyed_rules: 0,
        control_flags: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if is_path_rule(line) {
            counts.watch_rules += 1;
            saw_any = true;
            continue;
        }
        if !line.starts_with('-') {
            continue;
        }
        let mut flagged = false;
        for tok in line.split_whitespace() {
            match tok {
                "-w" | "-W" => {
                    counts.watch_rules += 1;
                    flagged = true;
                }
                "-a" | "-A" => {
                    counts.syscall_rules += 1;
                    flagged = true;
                }
                "-S" => {
                    counts.syscalls += 1;
                    flagged = true;
                }
                "-F" => {
                    counts.filters += 1;
                    flagged = true;
                }
                "-k" => {
                    counts.keyed_rules += 1;
                }
                "-D" | "-b" | "-e" | "-f" | "-i" | "-l" | "-r" | "-s" | "-v" | "-d" => {
                    counts.control_flags += 1;
                    flagged = true;
                }
                t if t.len() == 2 && t.starts_with('-') && t.ends_with('c') => {
                    counts.control_flags += 1;
                    flagged = true;
                }
                _ => {}
            }
        }
        if flagged {
            saw_any = true;
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

    const SAMPLE: &[u8] = br#"# audit rules
-D
-b 8192
-f 1
-i
-w /etc/passwd -p wa -k passwd_changes
-w /etc/shadow -p rwa -k shadow_changes
-a always,exit -F arch=b64 -S open -S openat -k file_access
-a always,exit -F arch=b64 -F auid>=1000 -F auid!=-1 -S execve -k exec
-a never,user -F subj_type=crond_t
-e 2
"#;

    #[test]
    fn detects_audit_rules() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.watch_rules, 2);
        assert_eq!(c.syscall_rules, 3);
        assert_eq!(c.syscalls, 3);
        assert_eq!(c.filters, 5);
        assert_eq!(c.keyed_rules, 4);
        assert_eq!(c.control_flags, 5);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"-flag\n--long-option\n"));
    }
}
