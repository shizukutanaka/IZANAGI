//! UFW (Uncomplicated Firewall) `user.rules`/`user6.rules`/`before.rules`/
//! `after.rules` ファイルの解析。
//!
//! iptables-save 方言 + `### tuple ###`/`### RULES ###`/`### LOGGING ###`/
//! `### END RULES ###`/`### RATE LIMITING ###`/`### PORT ###` マーカーと
//! `-A ufw-*`/`ufw-before-*`/`ufw-user-*`/`ufw-logging-*` チェーン参照を
//! 検出し、タプル・ルール・マーカーの数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::ufwrules;
//!
//! let text = br#"### tuple ### allow any 80 0.0.0.0/0 any 0.0.0.0/0
//! -A ufw-user-input -p tcp --dport 80 -j ACCEPT
//! ### END RULES ###
//! COMMIT
//! "#;
//!
//! assert!(ufwrules::detect(text));
//! let c = ufwrules::parse(text).unwrap();
//! assert_eq!(c.tuples, 1);
//! assert_eq!(c.rules, 1);
//! ```

/// ufw ルールの集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `*filter` 等 iptables-save テーブル宣言数。
    pub tables: usize,
    /// `:ufw-*`/`:ufw-before-*` チェーン宣言数。
    pub chain_decls: usize,
    /// `-A <chain> …` ルール追加数。
    pub rules: usize,
    /// `### tuple ###` マーカー数。
    pub tuples: usize,
    /// `### RULES ###`/`### END RULES ###`/`### LOGGING ###`/`### RATE LIMITING ###`/`### PORT ###` マーカー数。
    pub markers: usize,
    /// `-A ufw-` ユーザーチェーンへのルール数。
    pub ufw_rules: usize,
    /// `COMMIT` 行数。
    pub commits: usize,
}

/// `b` が ufw ルールファイルらしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.tuples >= 1 || c.ufw_rules >= 1) && c.rules >= 1
}

fn is_ufw_marker(line: &str) -> bool {
    line.starts_with("###") && line.matches("###").count() >= 2
}

/// ufw ルールを解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        tables: 0,
        chain_decls: 0,
        rules: 0,
        tuples: 0,
        markers: 0,
        ufw_rules: 0,
        commits: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if is_ufw_marker(line) {
            if line.contains("tuple") {
                counts.tuples += 1;
            } else {
                counts.markers += 1;
            }
            saw_any = true;
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with('*') {
            counts.tables += 1;
            saw_any = true;
            continue;
        }
        if line.starts_with(':') {
            counts.chain_decls += 1;
            saw_any = true;
            continue;
        }
        if line == "COMMIT" {
            counts.commits += 1;
            saw_any = true;
            continue;
        }
        if line.starts_with("-A ") || line.starts_with("-I ") || line.starts_with("-D ") {
            counts.rules += 1;
            saw_any = true;
            if line.contains("ufw-") {
                counts.ufw_rules += 1;
            }
            continue;
        }
        if line.starts_with("-N ") {
            counts.chain_decls += 1;
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

    const SAMPLE: &[u8] = br#"*filter
:ufw-user-input - [0:0]
:ufw-user-output - [0:0]
:ufw-user-forward - [0:0]
:ufw-before-logging-input - [0:0]
:ufw-logging-deny - [0:0]
:ufw-logging-allow - [0:0]

### RULES ###

### tuple ### allow any 22 0.0.0.0/0 any 0.0.0.0/0
-A ufw-user-input -p tcp --dport 22 -j ACCEPT
-A ufw-user-input -p udp --dport 22 -j ACCEPT

### tuple ### deny any 25 0.0.0.0/0 any 0.0.0.0/0
-A ufw-user-input -p tcp --dport 25 -j DROP

### END RULES ###

### LOGGING ###
-A ufw-logging-deny -m limit --limit 3/minute --limit-burst 10 -j RETURN
### END LOGGING ###
COMMIT
"#;

    #[test]
    fn detects_ufw_rules() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tables, 1);
        assert_eq!(c.chain_decls, 6);
        assert_eq!(c.rules, 4);
        assert_eq!(c.tuples, 2);
        assert_eq!(c.ufw_rules, 4);
        assert_eq!(c.commits, 1);
    }

    #[test]
    fn rejects_plain_iptables_save() {
        assert!(!detect(b"*filter\n:INPUT ACCEPT [0:0]\nCOMMIT\n"));
        assert!(!detect(b"hello"));
    }
}
