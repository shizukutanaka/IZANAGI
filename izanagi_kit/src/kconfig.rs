//! Kconfig 言語ファイル(Linux カーネル・Zephyr・Buildroot)の認識と計数。
//!
//! `Kconfig` 系ファイルは `config NAME` / `menuconfig NAME` / `comment "…"` /
//! `menu "…"` / `if …` / `choice` / `source "…"` のブロックで構成され、
//! 各シンボルの中に `bool`/`tristate`/`int`/`hex`/`string` 型行、
//! `prompt`/`default`/`depends on`/`select`/`imply`/`range`/`help`/`---help---`/
//! `option`/`visible if`/`optional` 属性を持つ。`menu`/`endmenu`、
//! `choice`/`endchoice`、`if`/`endif` は対応する。
//!
//! ```
//! let b = b"mainmenu \"My project\"\n\nconfig FOO\n    bool \"Enable foo\"\n    default y\n    depends on BAR\n    select BAZ\n    help\n      Foo feature help text.\n\nmenuconfig BAR\n    bool \"Bar\"\n    default n\n\nmenu \"Extras\"\n\nconfig BAZ\n    tristate \"Baz driver\"\n    range 0 100\n\nendmenu\n\nif BAR\nsource \"drivers/Kconfig\"\nendif\n";
//! assert!(izanagi_kit::kconfig::detect(b));
//! let c = izanagi_kit::kconfig::parse(b).unwrap();
//! assert_eq!(c.symbols, 3); // FOO, BAR(menuconfig), BAZ
//! assert_eq!(c.types, 3); // bool, bool, tristate
//! assert_eq!(c.defaults, 2);
//! assert_eq!(c.deps, 1);
//! assert_eq!(c.menus, 1);
//! assert_eq!(c.sources, 1);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `config NAME`/`menuconfig NAME` シンボルの個数。
    pub symbols: usize,
    /// `menuconfig` 個数(symbols に含まれる)。
    pub menuconfigs: usize,
    /// `bool`/`tristate`/`int`/`hex`/`string` 型行の個数。
    pub types: usize,
    /// `default` 行の個数。
    pub defaults: usize,
    /// `depends on` 行の個数。
    pub deps: usize,
    /// `select`/`imply` 行の個数。
    pub selects: usize,
    /// `range`/`option`/`visible if`/`optional`/`def_bool`/`def_tristate`
    /// 属性行の個数。
    pub attrs: usize,
    /// `menu`/`endmenu` ペアの `menu` 見出し個数。
    pub menus: usize,
    /// `choice`/`endchoice` の `choice` 個数。
    pub choices: usize,
    /// `if`/`endif` の `if` 個数。
    pub ifs: usize,
    /// `source`/`rsource`/`osource`/`orsource`/`ksource` 行の個数。
    pub sources: usize,
    /// `help`/`---help---` 行の個数。
    pub helps: usize,
    /// `comment` 行の個数。
    pub comment_stmts: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const TYPE_WORDS: &[&str] = &["bool", "boolean", "tristate", "int", "hex", "string"];

const ATTR_PREFIX: &[&str] = &[
    "range ",
    "option ",
    "visible if",
    "optional",
    "def_bool",
    "def_tristate",
];

/// Kconfig らしさを返す。`config`/`menuconfig` シンボルと属性の組合せ。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut syms = 0usize;
    let mut attrs = 0usize;
    for l in t.lines() {
        let s = l.trim();
        let head = s.split_whitespace().next().unwrap_or("");
        match head {
            "config" | "menuconfig" | "choice" | "menu" | "if" | "comment" | "source"
            | "rsource" | "osource" | "orsource" | "ksource" | "mainmenu" | "endmenu"
            | "endchoice" | "endif" => syms += 1,
            "bool" | "boolean" | "tristate" | "int" | "hex" | "string" | "default" | "depends"
            | "select" | "imply" | "range" | "help" | "---help---" | "option" | "visible"
            | "optional" | "def_bool" | "def_tristate" => {
                attrs += 1;
            }
            _ => {}
        }
    }
    syms >= 1 && attrs >= 1 || syms >= 4
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        symbols: 0,
        menuconfigs: 0,
        types: 0,
        defaults: 0,
        deps: 0,
        selects: 0,
        attrs: 0,
        menus: 0,
        choices: 0,
        ifs: 0,
        sources: 0,
        helps: 0,
        comment_stmts: 0,
        comments: 0,
    };
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let head = s.split_whitespace().next().unwrap_or("");
        match head {
            "config" => c.symbols += 1,
            "menuconfig" => {
                c.symbols += 1;
                c.menuconfigs += 1;
            }
            "bool" | "boolean" | "tristate" | "int" | "hex" | "string" => c.types += 1,
            "default" => c.defaults += 1,
            "depends" => {
                if s[7..].trim_start().starts_with("on") {
                    c.deps += 1;
                }
            }
            "select" | "imply" => c.selects += 1,
            "menu" => c.menus += 1,
            "choice" => c.choices += 1,
            "if" => c.ifs += 1,
            "source" | "rsource" | "osource" | "orsource" | "ksource" => c.sources += 1,
            "help" | "---help---" => c.helps += 1,
            "comment" => c.comment_stmts += 1,
            _ => {
                if ATTR_PREFIX.iter().any(|p| s.starts_with(p)) || TYPE_WORDS.contains(&head) {
                    c.attrs += 1;
                }
            }
        }
    }
    if !t.trim().is_empty()
        && c.symbols
            + c.menuconfigs
            + c.types
            + c.defaults
            + c.deps
            + c.selects
            + c.attrs
            + c.menus
            + c.choices
            + c.ifs
            + c.sources
            + c.helps
            + c.comment_stmts
            + c.comments
            == 0
    {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(b"config FOO\n    bool \"x\"\n    default y\n"));
        assert!(detect(
            b"menu \"A\"\nendmenu\nchoice\nendchoice\nif X\nendif\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"FOO=bar\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"config FOO\n    bool \"x\"\n    default y\n    depends on BAR\n    select BAZ\nmenu \"M\"\nconfig BAZ\n    int \"n\"\n    range 0 9\nendmenu\nif X\nsource \"k\"\nendif\nhelp\n    text\n";
        let c = parse(b).unwrap();
        assert_eq!(c.symbols, 2);
        assert_eq!(c.types, 2);
        assert_eq!(c.defaults, 1);
        assert_eq!(c.deps, 1);
        assert_eq!(c.selects, 1);
        assert_eq!(c.attrs, 1); // range
        assert_eq!(c.menus, 1);
        assert_eq!(c.ifs, 1);
        assert_eq!(c.sources, 1);
        assert_eq!(c.helps, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"config A\n    bool \"x\"\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a config file at all\n").is_none());
    }
}
