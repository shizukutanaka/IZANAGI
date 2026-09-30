//! Snort / Suricata rule file scanner.
//!
//! A rule line is `ACTION PROTO SRC SRC_PORT DIR DST DST_PORT
//! (option:value; …)` where `ACTION` is one of `alert`, `drop`,
//! `pass`, `log`, `reject`, `sdrop`, `activate`, `dynamic` (Snort)
//! plus `rejectsrc`/`rejectdst`/`rejectboth` (Suricata). `#` lines
//! are comments; `()`-free lines like `var`/`config` are skipped.
//!
//! ```
//! let d = br#"# c
//! alert tcp any any -> any any (msg:"x"; sid:1000001; rev:1;)
//! drop udp any any -> any 53 (msg:"y"; sid:2; classtype:trojan-activity;)
//! "#;
//! let s = izanagi_kit::snort::parse(d).unwrap();
//! assert_eq!(s.rules.len(), 2);
//! assert_eq!(s.rules[0].sid, Some(1_000_001));
//! ```
//!
//! Reference: Snort rule-writing documentation ("Rules Anatomy" —
//! header `action proto addr port direction addr port` + option
//! tree) and the Suricata rule format documentation.

/// One parsed rule.
#[derive(Debug, Clone, PartialEq)]
pub struct SnortRule {
    /// Header action (`alert`, `drop`, …).
    pub action: String,
    /// Protocol token (`tcp`, `udp`, `icmp`, `ip`, `http`, …).
    pub protocol: String,
    /// `msg:"…"` option.
    pub msg: Option<String>,
    /// `sid:` option.
    pub sid: Option<u64>,
    /// `rev:` option.
    pub rev: Option<u64>,
    /// `classtype:` option.
    pub classtype: Option<String>,
    /// `gid:` option.
    pub gid: Option<u64>,
}

/// Parsed `.rules` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Snort {
    /// Rules in file order.
    pub rules: Vec<SnortRule>,
    /// `#`-comment line count.
    pub comments: usize,
}

const ACTIONS: &[&str] = &[
    "alert",
    "drop",
    "pass",
    "log",
    "reject",
    "sdrop",
    "activate",
    "dynamic",
    "rejectsrc",
    "rejectdst",
    "rejectboth",
];

fn opt<'a>(opts: &'a str, key: &str) -> Option<&'a str> {
    for part in opts.split(';') {
        let p = part.trim();
        if let Some(v) = p.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
            return Some(v.trim());
        }
    }
    None
}

fn opt_num(opts: &str, key: &str) -> Option<u64> {
    opt(opts, key)?.parse().ok()
}

fn opt_str(opts: &str, key: &str) -> Option<String> {
    let v = opt(opts, key)?;
    let v = v.trim_matches('"');
    Some(v.to_string())
}

/// Parse a `.rules` file; `None` when no rule line exists.
pub fn parse(d: &[u8]) -> Option<Snort> {
    let text = core::str::from_utf8(d).ok()?;
    let mut rules = Vec::new();
    let mut comments = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(stripped) = line.strip_prefix('#') {
            let _ = stripped;
            comments += 1;
            continue;
        }
        let open = match line.find('(') {
            Some(i) => i,
            None => continue,
        };
        if !line.ends_with(')') {
            continue;
        }
        let header = line[..open].trim();
        let mut tok = header.split_whitespace();
        let action = match tok.next() {
            Some(a) if ACTIONS.contains(&a) => a,
            _ => continue,
        };
        let protocol = match tok.next() {
            Some(p) => p.to_string(),
            None => continue,
        };
        let opts = &line[open + 1..line.len() - 1];
        rules.push(SnortRule {
            action: action.to_string(),
            protocol,
            msg: opt_str(opts, "msg"),
            sid: opt_num(opts, "sid"),
            rev: opt_num(opts, "rev"),
            classtype: opt_str(opts, "classtype"),
            gid: opt_num(opts, "gid"),
        });
    }
    if rules.is_empty() {
        None
    } else {
        Some(Snort { rules, comments })
    }
}

/// `true` if the buffer looks like a Snort/Suricata rule file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = br#"# comment
var HOME_NET any
alert tcp any any -> any any (msg:"x"; sid:1000001; rev:1;)
drop udp any any -> any 53 (msg:"y"; sid:2; classtype:trojan-activity; gid:1;)
"#;

    #[test]
    fn parses() {
        let s = parse(DOC).unwrap();
        assert_eq!(s.rules.len(), 2);
        assert_eq!(s.comments, 1);
        assert_eq!(s.rules[0].action, "alert");
        assert_eq!(s.rules[0].protocol, "tcp");
        assert_eq!(s.rules[0].msg.as_deref(), Some("x"));
        assert_eq!(s.rules[0].sid, Some(1_000_001));
        assert_eq!(s.rules[0].rev, Some(1));
        assert_eq!(s.rules[1].classtype.as_deref(), Some("trojan-activity"));
        assert_eq!(s.rules[1].gid, Some(1));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# only comments\n#x").is_none());
        assert!(parse(b"var X y\nconfig z").is_none());
        assert!(parse(b"garbage tcp any any").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"alert tcp")); // no options
    }
}
