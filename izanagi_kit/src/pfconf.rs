//! OpenBSD pf.conf (Packet Filter) 設定ファイルの解析。
//!
//! `set`/`scrub`/`nat`/`rdr`/`binat`/`pass`/`block`/`match`/`anchor`/
//! `table <name>`/`altq`/`queue`/`antispoof` 文と `<名> = <値>` マクロの
//! 形を持つ設定を検出し、文種別の数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::pfconf;
//!
//! let text = br#"ext_if = \"em0\"
//! set skip on lo
//! block return
//! pass in on $ext_if proto tcp to port 22 keep state
//! "#;
//!
//! assert!(pfconf::detect(text));
//! let c = pfconf::parse(text).unwrap();
//! assert_eq!(c.macros, 1);
//! assert_eq!(c.filter_rules, 2);
//! ```

/// pf.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<名> = <値>` マクロ定義数。
    pub macros: usize,
    /// `set …` オプション文数。
    pub options: usize,
    /// `table <…>` 定義数。
    pub tables: usize,
    /// `pass`/`block` フィルタルール数。
    pub filter_rules: usize,
    /// `nat`/`rdr`/`binat`/`match`/`nat-to`/`rdr-to` 変換ルール数。
    pub nat_rules: usize,
    /// `queue`/`altq` キュー文数。
    pub queues: usize,
    /// `anchor`/`load anchor`/`antispoof`/`scrub` その他文数。
    pub misc_statements: usize,
    /// 継続行 (`\` 終わり) 数。
    pub continuations: usize,
}

/// `b` が pf.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.filter_rules >= 2 || (c.filter_rules >= 1 && (c.options >= 1 || c.tables >= 1))
}

/// pf.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        macros: 0,
        options: 0,
        tables: 0,
        filter_rules: 0,
        nat_rules: 0,
        queues: 0,
        misc_statements: 0,
        continuations: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.ends_with('\\') {
            counts.continuations += 1;
        }
        let first = line.split_whitespace().next().unwrap_or("");
        match first {
            "set" => {
                counts.options += 1;
                saw_any = true;
            }
            "table" => {
                counts.tables += 1;
                saw_any = true;
            }
            "pass" | "block" => {
                counts.filter_rules += 1;
                saw_any = true;
            }
            "nat" | "rdr" | "binat" | "match" | "no" | "nat-to" | "rdr-to" => {
                counts.nat_rules += 1;
                saw_any = true;
            }
            "queue" | "altq" | "scheduler" => {
                counts.queues += 1;
                saw_any = true;
            }
            "anchor" | "load" | "antispoof" | "scrub" | "include" | "dummynet" | "rdom"
            | "keepcounters" => {
                counts.misc_statements += 1;
                saw_any = true;
            }
            _ => {
                // `name = value` マクロ(引用符囲み値許容)。
                if line.contains('=')
                    && first
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
                {
                    counts.macros += 1;
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

    const SAMPLE: &[u8] = br#"# pf.conf
ext_if = "em0"
int_if = "em1"
icmp_types = "{ echoreq, unreach }"
trusted = "{ 10.0.0.0/24, 192.168.0.0/24 }"

set block-policy return
set loginterface $ext_if
set skip on lo
set state-policy if-bound
scrub in on $ext_if all fragment reassemble

table <bruteforce> persist

nat on $ext_if from $trusted to any -> ($ext_if)
rdr pass on $ext_if proto tcp to port 80 -> 10.0.0.5 port 8080

altq on $ext_if cbq bandwidth 1Mb queue { q_default, q_bulk }
queue q_default bandwidth 90% cbq(default)

pass out on $ext_if inet keep state
block return in on $ext_if all
pass in on $ext_if proto tcp to port 22 keep state \
    (max-src-conn 10, max-src-conn-rate 3/30, overload <bruteforce>)
pass in on $ext_if proto tcp to $trusted port {80, 443} keep state
antispoof for $ext_if inet
anchor "ftp-proxy/*"
"#;

    #[test]
    fn detects_pf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.macros, 4);
        assert_eq!(c.options, 4);
        assert_eq!(c.tables, 1);
        assert_eq!(c.nat_rules, 2);
        assert_eq!(c.queues, 2);
        assert_eq!(c.filter_rules, 4);
        assert_eq!(c.misc_statements, 3);
        assert_eq!(c.continuations, 1);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"pass = word"));
    }
}
