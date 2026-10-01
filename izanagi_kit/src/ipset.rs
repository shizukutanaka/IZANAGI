//! ipset 設定ファイル (`ipset save`/`ipset restore`/`ipset -! restore` 入力) の解析。
//!
//! `create <name> <type> [options]` / `add <name> <member>` / `destroy`/`swap`/
//! `rename`/`flush`/`test`/`help` コマンドの形を持つ設定を検出し、セット
//! 作成数・要素追加数・型別数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::ipset;
//!
//! let text = br#"create trusted hash:ip family inet hashsize 1024 maxelem 65536
//! add trusted 10.0.0.1
//! add trusted 10.0.0.2
//! create nets hash:net family inet
//! add nets 192.168.0.0/24
//! "#;
//!
//! assert!(ipset::detect(text));
//! let c = ipset::parse(text).unwrap();
//! assert_eq!(c.creates, 2);
//! assert_eq!(c.adds, 3);
//! ```

/// ipset の既知セット型。
const KNOWN_TYPES: &[&str] = &[
    "hash:ip",
    "hash:net",
    "hash:ip,port",
    "hash:net,port",
    "hash:ip,mark",
    "hash:net,net",
    "hash:net,port,net",
    "hash:ip,port,ip",
    "hash:ip,port,net",
    "hash:mac",
    "hash:net,iface",
    "hash:net,link",
    "bitmap:ip",
    "bitmap:ip,mac",
    "bitmap:port",
    "list:set",
];

/// ipset 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `create <name> <type>` 行数。
    pub creates: usize,
    /// `add <name> <member>` 行数。
    pub adds: usize,
    /// `destroy`/`swap`/`rename`/`flush`/`test`/`list`/`save`/`restore`/`help`/`quit`/`del` 行数。
    pub other_commands: usize,
    /// 既知セット型 (`hash:*`/`bitmap:*`/`list:set`) の指定数。
    pub typed_sets: usize,
    /// IPv6 メンバー (`:` 含み) を持つ add 行数。
    pub ipv6_members: usize,
    /// `timeout N`/`comment "…"`/`skbinfo` オプション付き add 行数。
    pub optioned_adds: usize,
}

/// `b` が ipset save/restore 形式らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.creates >= 1 && c.typed_sets >= 1) || (c.creates >= 1 && c.adds >= 2)
}

/// ipset 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        creates: 0,
        adds: 0,
        other_commands: 0,
        typed_sets: 0,
        ipv6_members: 0,
        optioned_adds: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let Some(cmd) = it.next() else {
            continue;
        };
        match cmd {
            "create" => {
                counts.creates += 1;
                saw_any = true;
                if let Some(ty) = it.nth(1) {
                    if KNOWN_TYPES.contains(&ty) {
                        counts.typed_sets += 1;
                    }
                }
            }
            "add" | "del" => {
                if cmd == "add" {
                    counts.adds += 1;
                    saw_any = true;
                    let member = it.nth(1).unwrap_or("");
                    if member.bytes().filter(|c| *c == b':').count() >= 2 {
                        counts.ipv6_members += 1;
                    }
                    if line.contains("timeout ")
                        || line.contains("comment ")
                        || line.contains("skbinfo ")
                    {
                        counts.optioned_adds += 1;
                    }
                } else {
                    counts.other_commands += 1;
                    saw_any = true;
                }
            }
            "destroy" | "swap" | "rename" | "flush" | "test" | "list" | "save" | "restore"
            | "help" | "quit" | "version" | "get_byname" | "get_byindex" | "header" | "type"
            | "exist" | "create-help" => {
                counts.other_commands += 1;
                saw_any = true;
            }
            _ => {}
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

    const SAMPLE: &[u8] = br#"create trusted hash:ip family inet hashsize 1024 maxelem 65536
create trusted6 hash:ip family inet6 hashsize 1024 maxelem 65536
create nets hash:net family inet
create services hash:ip,port family inet
create ports bitmap:port range 0-65535
create sets list:set size 4
add trusted 10.0.0.1
add trusted 10.0.0.2 timeout 300
add trusted6 2001:db8::1
add nets 192.168.0.0/24
add nets 172.16.0.0/16 comment "office"
add services 10.0.0.1,tcp:80
add services 10.0.0.1,udp:53
add ports 80
add ports 443
destroy oldset
"#;

    #[test]
    fn detects_ipset() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.creates, 6);
        assert_eq!(c.typed_sets, 6);
        assert_eq!(c.adds, 9);
        assert_eq!(c.ipv6_members, 1);
        assert_eq!(c.optioned_adds, 2);
        assert_eq!(c.other_commands, 1);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"add foo bar"));
    }
}
