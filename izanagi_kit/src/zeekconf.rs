//! `local.zeek` / `node.cfg` (Zeek) 検出モジュール。
//!
//! Zeek (旧 Bro、ネットワーク監視) のスクリプトは `@load` /
//! `@load-sigs` / `@load-plugin` / `@ifdef` / `redef` / `export` /
//! `global` 等の宣言で構成される。`node.cfg` 側は INI で
//! `[manager]`/`[proxy]`/`[worker]`/`[logger]` セクション。
//!
//! ```
//! let b = br#"# Zeek local site policy
//! @load base/frameworks/cluster
//! @load policy/misc/loaded-scripts
//! @load-sigs frameworks/signatures/detect-windows-shells
//! redef Site::local_nets += { 10.0.0.0/8 };
//! "#;
//! let c = izanagi_kit::zeekconf::parse(b);
//! assert!(izanagi_kit::zeekconf::detect(b));
//! assert_eq!(c.loads, 3);
//! ```

const DIRECTIVES: &[&str] = &[
    "@load",
    "@load-plugin",
    "@load-sigs",
    "@load-file",
    "@unload",
    "@ifdef",
    "@ifndef",
    "@else",
    "@endif",
    "@prefixes",
];

fn is_load(t: &str) -> bool {
    t.starts_with("@load")
}

fn is_zeek_stmt(t: &str) -> bool {
    DIRECTIVES.iter().any(|d| t.starts_with(d))
        || t.starts_with("redef ")
        || t.starts_with("export ")
        || t.starts_with("global ")
        || t.starts_with("local ")
}

/// `b` が Zeek スクリプト/設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut loads = 0usize;
    let mut stmts = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with("//") {
            continue;
        }
        if is_load(tr) {
            loads += 1;
            stmts += 1;
        } else if is_zeek_stmt(tr) {
            stmts += 1;
        }
    }
    loads >= 1 && stmts >= 2
}

/// Zeek スクリプトの統計。
#[derive(Debug, Default, Clone)]
pub struct ZeekConf {
    /// `@load*` 行数。
    pub loads: usize,
    /// 宣言/ディレクティブ行数。
    pub statements: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Zeek スクリプトとして統計する。
pub fn parse(b: &[u8]) -> ZeekConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = ZeekConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if is_load(tr) {
            c.loads += 1;
            c.statements += 1;
        } else if is_zeek_stmt(tr) {
            c.statements += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"@load base/bif
@load policy/ftp
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.loads, 2);
    }

    #[test]
    fn detects_with_redef() {
        let b = br#"@load base/bif
redef Conn::in_orig = T;
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"@load base/bif\n"));
        assert!(!detect(b"load file\nredef x\n"));
        assert!(!detect(b"# @load a\n# @load b\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.loads, 0);
    }
}
