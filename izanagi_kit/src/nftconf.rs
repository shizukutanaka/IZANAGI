//! nftables 設定ファイル (`nftables.conf` / `nft list ruleset` 出力) の解析。
//!
//! `table <family> <name> {` / `chain <name> {` / `type <type> hook <hook>
//! priority <n>; policy <p>;` / ルール文 (`ct state`、`tcp dport`、`accept`)
//! / `set <name> {`/`map <name> {` の形を持つ設定を検出し、ブロック・
//! ルール・判定文の数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::nftconf;
//!
//! let text = br#"table ip filter {
//!     chain input {
//!         type filter hook input priority 0; policy drop;
//!         ct state established,related accept
//!         tcp dport 22 accept
//!     }
//! }
//! "#;
//!
//! assert!(nftconf::detect(text));
//! let c = nftconf::parse(text).unwrap();
//! assert_eq!(c.tables, 1);
//! assert_eq!(c.chains, 1);
//! assert_eq!(c.verdicts, 3);
//! ```

/// 判定・終端文キーワード。
const VERDICTS: &[&str] = &[
    "accept",
    "drop",
    "reject",
    "jump",
    "goto",
    "return",
    "continue",
    "masquerade",
    "snat",
    "dnat",
    "redirect",
    "log",
    "counter",
    "notrack",
    "queue",
    "limit",
    "nat",
    "tproxy",
    "synproxy",
    "flow",
];

/// nft ファミリー名。
const FAMILIES: &[&str] = &["ip", "ip6", "inet", "arp", "bridge", "netdev"];

/// nftables.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `table <fam> <name>` ブロック数。
    pub tables: usize,
    /// `chain <name>` ブロック数。
    pub chains: usize,
    /// `set`/`map`/`flowtable`/`counter`/`quota` 名付きブロック数。
    pub named_objects: usize,
    /// `type … hook … priority` 宣言数。
    pub chain_types: usize,
    /// `policy <p>;` 宣言数。
    pub policies: usize,
    /// 判定・終端文を含むルール行数。
    pub verdicts: usize,
    /// `elements = {` 要素定義数。
    pub element_blocks: usize,
}

/// `b` が nftables.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.tables >= 1 && (c.chains >= 1 || c.named_objects >= 1))
        || (c.chain_types >= 1 && c.verdicts >= 1)
}

fn stmt_words(line: &str) -> impl Iterator<Item = &str> {
    line.split([' ', '\t', ';', '{', '}'])
        .filter(|w| !w.is_empty())
}

/// nftables.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        tables: 0,
        chains: 0,
        named_objects: 0,
        chain_types: 0,
        policies: 0,
        verdicts: 0,
        element_blocks: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "}" || line == "};" || line == "{" {
            continue;
        }
        let mut words = stmt_words(line);
        let Some(first) = words.next() else {
            continue;
        };
        match first {
            "table" => {
                if let Some(fam) = words.next() {
                    if FAMILIES.contains(&fam) {
                        counts.tables += 1;
                        saw_any = true;
                    }
                }
            }
            "chain" => {
                counts.chains += 1;
                saw_any = true;
            }
            "set" | "map" | "flowtable" | "counter" | "quota" | "ct" | "secmark" => {
                if first != "ct" || words.next().is_some_and(|w| w == "helper") {
                    counts.named_objects += 1;
                }
                saw_any = true;
            }
            "type" => {
                if line.contains("hook") {
                    counts.chain_types += 1;
                    saw_any = true;
                }
                if line.contains("policy ") || line.contains("policy\t") {
                    counts.policies += 1;
                }
            }
            "policy" => {
                counts.policies += 1;
            }
            "elements" => {
                counts.element_blocks += 1;
                saw_any = true;
            }
            "add" | "insert" | "delete" | "flush" | "list" | "describe" | "create" | "rename"
            | "reset" | "monitor" | "export" | "import" | "include" | "define" | "undefine"
            | "destroy" | "get" | "check" => {
                saw_any = true;
            }
            _ => {}
        }
        if VERDICTS.iter().any(|v| {
            line.ends_with(v)
                || line.contains(&format!(" {v} "))
                || line.contains(&format!(" {v};"))
                || line.contains(&format!(" {v}$"))
        }) || VERDICTS.contains(
            &line
                .trim_end_matches(';')
                .split_whitespace()
                .last()
                .unwrap_or(""),
        ) {
            counts.verdicts += 1;
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

    const SAMPLE: &[u8] = br#"#!/usr/sbin/nft -f
flush ruleset

table ip filter {
    chain input {
        type filter hook input priority 0; policy drop;
        ct state invalid drop
        ct state established,related accept
        iifname "lo" accept
        tcp dport 22 accept
        tcp dport {80, 443} accept
        icmp type echo-request accept
    }
    chain forward {
        type filter hook forward priority 0; policy drop;
        ip saddr 10.0.0.0/8 ct state established accept
    }
    set trusted {
        type ipv4_addr
        elements = {10.0.0.1, 10.0.0.2}
    }
}

table ip6 filter {
    chain input {
        type filter hook input priority 0; policy drop;
        icmpv6 type { echo-request, nd-neighbor-solicit } accept
    }
}
"#;

    #[test]
    fn detects_nft() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tables, 2);
        assert_eq!(c.chains, 3);
        assert_eq!(c.chain_types, 3);
        assert_eq!(c.policies, 3);
        assert_eq!(c.named_objects, 1);
        assert_eq!(c.element_blocks, 1);
        assert_eq!(c.verdicts, 11);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"table = ip"));
    }
}
