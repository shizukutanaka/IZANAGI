//! Zeek スクリプト (`local.zeek`, `*.zeek`, `*.bro`) パーサ。
//!
//! `@load`/`@if`/`@ifdef`/`@endif`/`@load-sigs` 等 `\@` ディレクティブと
//! `module`/`export`/`redef`/`event`/`hook`/`function` 宣言を計数する。
//!
//! ```
//! use izanagi_kit::zeekscript;
//! let sc = b"@load base/frameworks/cluster\n@load base/protocols/conn\nmodule Local;\n\nexport {\n    redef enum Log::ID += { LOG };\n}\n\nevent zeek_init() {\n    print \"hi\";\n}\n";
//! assert!(zeekscript::detect(sc));
//! let c = zeekscript::parse(sc).unwrap();
//! assert_eq!(c.at_directives, 2);
//! assert_eq!(c.events, 1);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `@` ディレクティブ行数。
    pub at_directives: usize,
    /// `module` 宣言数。
    pub modules: usize,
    /// `export` ブロック数。
    pub exports: usize,
    /// `redef` 文数。
    pub redefs: usize,
    /// `event` 宣言数。
    pub events: usize,
    /// `function` 宣言数。
    pub functions: usize,
    /// `hook` 宣言数。
    pub hooks: usize,
    /// 宣言系ステートメント総数。
    pub statements: usize,
}

const DECL_WORDS: &[&str] = &[
    "module",
    "export",
    "redef",
    "event",
    "function",
    "hook",
    "type",
    "global",
    "const",
    "option",
    "if",
    "for",
    "while",
    "switch",
    "return",
    "local",
    "schedule",
    "print",
    "delete",
    "add",
    "next",
    "break",
    "continue",
    "fallthrough",
    "default",
    "when",
];

/// 簡易判定 (`@` ディレクティブ or Zeek 固有宣言語彙)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.at_directives >= 2
        || (c.at_directives >= 1 && (c.redefs + c.exports + c.events) >= 1)
        || (c.statements >= 3 && (c.redefs + c.exports + c.events + c.modules) >= 2)
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        at_directives: 0,
        modules: 0,
        exports: 0,
        redefs: 0,
        events: 0,
        functions: 0,
        hooks: 0,
        statements: 0,
    };
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('@') {
            c.at_directives += 1;
            continue;
        }
        let Some(first) = t.split([' ', '(', ';', '{', '\t']).next() else {
            continue;
        };
        match first {
            "module" => c.modules += 1,
            "export" => c.exports += 1,
            "redef" => c.redefs += 1,
            "event" => c.events += 1,
            "function" => c.functions += 1,
            "hook" => c.hooks += 1,
            _ => {}
        }
        if DECL_WORDS.contains(&first) {
            c.statements += 1;
        }
    }
    (c.statements + c.at_directives > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"##! Site policy loader\n@load base/frameworks/notice\n@load protocols/conn/dhcp\n@load-sigs framework/signatures/detect-windows-shells.sig\nmodule Site;\n\nexport {\n    const local_subnet = 192.168.0.0/16;\n}\n\nredef Site::local_nets += { 10.0.0.0/8 };\nredef PacketFilter::default_capture_filter = \"ip\";\n\nevent zeek_init()\n    {\n    print \"init\";\n    }\n\nevent http_stats(c: connection, stats: http_stats_rec)\n    {\n    print \"stats\";\n    }\n";

    #[test]
    fn detects_zeekscript() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.at_directives, 3);
        assert_eq!(c.modules, 1);
        assert_eq!(c.exports, 1);
        assert_eq!(c.redefs, 2);
        assert_eq!(c.events, 2);
        assert_eq!(c.statements, 9);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"#!/bin/sh\nls -la\n"));
        assert!(!detect(b"int main() { return 0; }\n"));
    }
}
