//! ferm 設定ファイル (`ferm.conf`/`ferm.d/*`) の解析。
//!
//! `@def $NAME = value;`/`domain (ip ip6)`/`table filter`/`chain INPUT`/
//! `policy DROP`/ルール文 (`proto tcp dport 22 ACCEPT`)/`@include`/`@hook`/
//! `@if`/`@else`/`@resolve`/`@ipfilter`/`@eq`/`@not`/`@subchain`/
//! `saddr`/`daddr`/`mod`/`state`/`outerface`/`target` 形を持つ設定を検出し、
//! ブロック・ルール・ディレクティブ数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::ferm;
//!
//! let text = br#"@def $EXT_IF = eth0;
//! domain ip table filter {
//!     chain INPUT {
//!         policy DROP;
//!         proto tcp dport 22 ACCEPT;
//!     }
//! }
//! "#;
//!
//! assert!(ferm::detect(text));
//! let c = ferm::parse(text).unwrap();
//! assert_eq!(c.directives, 1);
//! assert_eq!(c.policies, 1);
//! ```

/// ferm の `@`-ディレクティブ。
const KNOWN_DIRECTIVES: &[&str] = &[
    "@def",
    "@include",
    "@hook",
    "@if",
    "@else",
    "@end",
    "@resolve",
    "@ipfilter",
    "@eq",
    "@ne",
    "@not",
    "@subchain",
    "@assert",
    "@emit",
    "@cat",
    "@subst",
    "@export",
    "@real",
    "@keys",
    "@call",
    "@debug",
    "@error",
    "@warn",
    "@abort",
    "@repeat",
    "@exists",
];

/// ferm 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `@def`/`@include`/`@hook`/`@resolve`/`@if`/`@else`/`@end`/`@subchain`/`@assert`/`@emit`/`@cat`/`@subst`/`@export`/`@real`/`@keys`/`@call`/`@debug`/`@error`/`@warn`/`@abort`/`@repeat`/`@exists`/`@ipfilter`/`@eq`/`@ne`/`@not` ディレクティブ行数。
    pub directives: usize,
    /// `domain …`/`domain (…)` ブロック数。
    pub domains: usize,
    /// `table <name>` ブロック数。
    pub tables: usize,
    /// `chain <name>` ブロック数。
    pub chains: usize,
    /// `policy <p>;` 宣言数。
    pub policies: usize,
    /// ルール行数 (`;` 終わりのマッチ/ターゲット文)。
    pub rules: usize,
    /// 継続行を含む中括弧ブロック開始 (`{`) 数。
    pub blocks: usize,
}

/// `b` が ferm.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.directives >= 1 && c.rules >= 1)
        || (c.domains >= 1 && c.policies >= 1)
        || (c.tables >= 1 && c.chains >= 1 && c.rules >= 1)
}

fn is_rule_line(line: &str) -> bool {
    // `proto tcp dport 22 ACCEPT;` や `target ACCEPT;` のような ferm ルール。
    let trimmed = line.trim();
    if !trimmed.ends_with(';') || trimmed.contains('{') {
        return false;
    }
    let mut it = trimmed.trim_end_matches(';').split_whitespace();
    let Some(first) = it.next() else {
        return false;
    };
    matches!(
        first,
        "proto"
            | "protocol"
            | "saddr"
            | "daddr"
            | "sport"
            | "dport"
            | "source"
            | "destination"
            | "interface"
            | "outerface"
            | "state"
            | "mod"
            | "target"
            | "jump"
            | "goto"
            | "match"
            | "syn"
            | "fragment"
            | "icmp-type"
            | "icmpv6-type"
            | "tcp-flags"
            | "tos"
            | "ttl"
            | "mac"
            | "pkttype"
            | "addrtype"
            | "comment"
            | "limit"
            | "connlimit"
            | "hashlimit"
            | "recent"
            | "string"
            | "length"
            | "u32"
            | "helper"
            | "ctstate"
            | "ct"
            | "conntrack"
            | "log"
            | "nflog"
            | "mark"
            | "connmark"
            | "set"
            | "tos-match"
            | "ecn"
            | "hl"
            | "rt"
            | "bpf"
            | "cluster"
            | "cpu"
            | "dccp"
            | "dst"
            | "dst-opts"
            | "esp"
            | "eui64"
            | "fuzzy"
            | "geoip"
            | "hbh"
            | "iprange"
            | "ipvs"
            | "kernel"
            | "mh"
            | "osf"
            | "owner"
            | "physdev"
            | "policy"
            | "quota"
            | "rateest"
            | "realm"
            | "rpfilter"
            | "sctp"
            | "socket"
            | "statistic"
            | "time"
            | "udp"
            | "unclean"
            | "vtag"
            | "ACCEPT"
            | "DROP"
            | "REJECT"
            | "RETURN"
            | "QUEUE"
            | "NFQUEUE"
            | "LOG"
            | "SNAT"
            | "DNAT"
            | "MASQUERADE"
            | "REDIRECT"
            | "NOTRACK"
            | "MARK"
            | "CONNMARK"
            | "SECMARK"
            | "TARPIT"
            | "CHECKSUM"
            | "CLASSIFY"
            | "CLUSTERIP"
            | "ECN"
            | "HL"
            | "HMARK"
            | "IDLETIMER"
            | "LED"
            | "NFLOG"
            | "RATEEST"
            | "TCPMSS"
            | "TCPOPTSTRIP"
            | "TEE"
            | "TOS"
            | "TPROXY"
            | "TRACE"
            | "TTL"
            | "AUDIT"
    )
}

/// ferm.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        directives: 0,
        domains: 0,
        tables: 0,
        chains: 0,
        policies: 0,
        rules: 0,
        blocks: 0,
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
        if line.ends_with('{') || line.ends_with('{'.to_string().as_str()) {
            counts.blocks += 1;
        }
        let first = line.split_whitespace().next().unwrap_or("");
        if first.starts_with('@') {
            if KNOWN_DIRECTIVES.contains(&first) {
                counts.directives += 1;
                saw_any = true;
            }
            continue;
        }
        match first {
            "domain" | "domains" => {
                counts.domains += 1;
                saw_any = true;
            }
            "table" => {
                if line.contains('{') {
                    counts.tables += 1;
                    saw_any = true;
                }
            }
            "chain" => {
                counts.chains += 1;
                saw_any = true;
            }
            "policy" => {
                counts.policies += 1;
                saw_any = true;
            }
            _ => {
                if is_rule_line(line) {
                    counts.rules += 1;
                    saw_any = true;
                }
            }
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

    const SAMPLE: &[u8] = br#"# ferm.conf
@def $EXT_IF = eth0;
@def $LAN = 192.168.0.0/24;
@include /etc/ferm/conf.d/*.conf;

domain (ip ip6) {
    table filter {
        chain INPUT {
            policy DROP;
            mod conntrack ctstate INVALID DROP;
            mod conntrack ctstate (ESTABLISHED RELATED) ACCEPT;
            interface lo ACCEPT;
            proto tcp dport 22 ACCEPT;
            proto tcp dport (80 443) ACCEPT;
            proto icmp icmp-type echo-request ACCEPT;
            proto udp dport 53 ACCEPT;
        }
        chain FORWARD {
            policy DROP;
            interface $EXT_IF outerface $EXT_IF mod conntrack ctstate (ESTABLISHED RELATED) ACCEPT;
        }
        chain OUTPUT {
            policy ACCEPT;
        }
    }
    table nat {
        chain POSTROUTING {
            saddr $LAN outerface $EXT_IF MASQUERADE;
        }
    }
}

@hook post "iptables-restore < /etc/iptables.bak";
"#;

    #[test]
    fn detects_ferm() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.directives, 4);
        assert_eq!(c.domains, 1);
        assert_eq!(c.tables, 2);
        assert_eq!(c.chains, 4);
        assert_eq!(c.policies, 3);
        assert_eq!(c.rules, 9);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"key = value"));
    }
}
