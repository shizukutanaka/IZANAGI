//! `eggdrop.conf` 検出モジュール。
//!
//! Eggdrop ボットの設定は Tcl スクリプトで、`set <var> <value>`、
//! `listen`、`logfile`、`channel add`、`source scripts/...`、
//! `loadmodule` 等の呼び出しが特徴。
//!
//! ```
//! let b = br#"set username "eggdrop"
//! set nick "LamestBot"
//! set altnick "L?mestBot"
//! set owner "admin"
//! listen 3333 all
//! logfile mco * "logs/eggdrop.log"
//! channel add #lamest
//! loadmodule blowfish
//! "#;
//! let c = izanagi_kit::eggdrop::parse(b);
//! assert!(izanagi_kit::eggdrop::detect(b));
//! assert_eq!(c.set_lines, 4);
//! ```

const COMMANDS: &[&str] = &[
    "channel add",
    "channel set",
    "die",
    "listen",
    "loadmodule",
    "logfile",
    "set",
    "source",
    "unbind",
    "unloadmodule",
];

const SET_VARS: &[&str] = &[
    "addlang",
    "altnick",
    "botnet-nick",
    "config",
    "dcc_ports",
    "eggdir",
    "logfile-suffix",
    "mod-path",
    "net-type",
    "nick",
    "nick-len",
    "owner",
    "realname",
    "server",
    "server-online-wait",
    "username",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn command_line(t: &str) -> bool {
    COMMANDS.iter().any(|c| t.starts_with(c)) && t.find(|c: char| c.is_alphabetic()).is_some()
}

fn set_line(t: &str, k: &str) -> bool {
    let Some(rest) = t.strip_prefix("set ") else {
        return false;
    };
    let rest = rest.trim_start();
    if !rest.starts_with(k) {
        return false;
    }
    let r = &rest[k.len()..];
    r.is_empty() || r.starts_with([' ', '\t'])
}

/// `b` が eggdrop.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut cmds = 0usize;
    let mut known = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if SET_VARS.iter().any(|k| set_line(tr, k)) {
            known += 1;
            cmds += 1;
        } else if command_line(tr) {
            cmds += 1;
        }
    }
    known >= 2 || (known >= 1 && cmds >= 4)
}

/// eggdrop.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct EggdropConf {
    /// `set` 行数。
    pub set_lines: usize,
    /// 既知 `set` 変数行数。
    pub known_sets: usize,
    /// `listen`/`logfile`/`channel add`/`loadmodule`/`source` 行数。
    pub command_lines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を eggdrop.conf として統計する。
pub fn parse(b: &[u8]) -> EggdropConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = EggdropConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("set ") {
            c.set_lines += 1;
            if SET_VARS.iter().any(|k| set_line(tr, k)) {
                c.known_sets += 1;
            }
        } else if command_line(tr) {
            c.command_lines += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"set username "eggdrop"
set nick "LamestBot"
listen 3333 all
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.known_sets, 2);
        assert_eq!(c.command_lines, 1);
    }

    #[test]
    fn detects_full() {
        let b = br#"set username "eggdrop"
set nick "LamestBot"
set altnick "L?mestBot"
set owner "admin"
listen 3333 all
logfile mco * "logs/eggdrop.log"
channel add #lamest
loadmodule blowfish
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.set_lines, 4);
        assert_eq!(c.command_lines, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"set foo 1\nset bar 2\nset baz 3\n"));
        assert!(!detect(b"listen 80\n"));
        assert!(!detect(b"puts hello\nproc x {} {}\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.set_lines, 0);
    }
}
