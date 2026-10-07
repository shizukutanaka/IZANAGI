//! Vyper smart-contract source (`.vy`) parser.
//!
//! Detects Vyper by `# @version` or `@external`/`@internal`-style
//! decorators on `def` functions, and counts decorators, events,
//! structs, interfaces, `log` calls and `#` comments.
//!
//! ```
//! use izanagi_kit::vyper::Vyper;
//! let src = b"# @version ^0\x2e3\n\n@external\ndef f():\n    pass\n";
//! assert!(izanagi_kit::vyper::detect(src));
//! let v = Vyper::parse(src).unwrap();
//! assert_eq!(v.functions, 1);
//! assert_eq!(v.externals, 1);
//! ```

/// Parsed census of a Vyper source file.
#[derive(Debug, Clone)]
pub struct Vyper {
    /// `# @version` constraint text.
    pub version: String,
    /// `def` function definitions.
    pub functions: usize,
    /// `@external` decorators.
    pub externals: usize,
    /// `@internal` decorators.
    pub internals: usize,
    /// `@view` decorators.
    pub views: usize,
    /// `@pure` decorators.
    pub pures: usize,
    /// `@payable` decorators.
    pub payables: usize,
    /// `@nonreentrant`/`@nonpayable` decorators.
    pub guards: usize,
    /// `event` declarations.
    pub events: usize,
    /// `struct` declarations.
    pub structs: usize,
    /// `interface` declarations.
    pub interfaces: usize,
    /// `implements:` lines.
    pub implements: usize,
    /// `import`/`from ... import` statements.
    pub imports: usize,
    /// `log` calls.
    pub logs: usize,
    /// `#` comment lines.
    pub comments: usize,
}
fn code_has(t: &str, needle: &str) -> bool {
    // `#` コメント行内の言及は証拠にしない。
    t.lines()
        .any(|l| !l.trim_start().starts_with('#') && l.contains(needle))
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Returns `true` when `b` looks like Vyper source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let t = strip_bom(t);
    t.contains("# @version")
        || (code_has(t, "def ")
            && ["@external", "@internal", "@view", "@pure", "@payable"]
                .iter()
                .any(|d| t.lines().any(|l| l.trim_start().starts_with(d))))
}

impl Vyper {
    /// Counts Vyper constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut v = Self {
            version: String::new(),
            functions: 0,
            externals: t.matches("@external").count(),
            internals: t.matches("@internal").count(),
            views: t.matches("@view").count(),
            pures: t.matches("@pure").count(),
            payables: t.matches("@payable").count(),
            guards: t.matches("@nonreentrant").count() + t.matches("@nonpayable").count(),
            events: 0,
            structs: 0,
            interfaces: 0,
            implements: 0,
            imports: 0,
            logs: t.matches("log ").count(),
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim_start();
            if l.starts_with('#') {
                v.comments += 1;
                if let Some(rest) = l.strip_prefix("# @version") {
                    if v.version.is_empty() {
                        v.version = rest.trim().to_string();
                    }
                }
                continue;
            }
            match l.split_whitespace().next().unwrap_or("") {
                "def" => v.functions += 1,
                "event" => v.events += 1,
                "struct" => v.structs += 1,
                "interface" => v.interfaces += 1,
                "implements:" => v.implements += 1,
                "import" => v.imports += 1,
                "from" if l.contains(" import ") => v.imports += 1,
                _ => {}
            }
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# def f():\n# @external\n"));
    }

    const SRC: &[u8] = b"# @version ^0\x2e3\n\n@external\ndef f():\n    pass\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"@view\ndef g():\n    pass"));
        assert!(!detect(b"def f():\n    pass"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let v = Vyper::parse(SRC).unwrap();
        assert_eq!(v.version, "^0\x2e3");
        assert_eq!(v.functions, 1);
        assert_eq!(v.externals, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"# @version ^0\x2e3\nimport a\nfrom b import c\nevent E:\n    x: uint256\nstruct S:\n    y: uint256\ninterface I:\n    def z() -> uint256: view\nimplements: I\n@internal\n@pure\ndef h():\n    pass\n@payable\n@nonreentrant(\"k\")\ndef p():\n    log E()\n";
        let v = Vyper::parse(s).unwrap();
        assert_eq!(v.imports, 2);
        assert_eq!(v.events, 1);
        assert_eq!(v.structs, 1);
        assert_eq!(v.interfaces, 1);
        assert_eq!(v.implements, 1);
        assert_eq!(v.internals, 1);
        assert_eq!(v.pures, 1);
        assert_eq!(v.payables, 1);
        assert_eq!(v.guards, 1);
        assert_eq!(v.logs, 1);
        assert!(v.comments >= 1);
    }

    #[test]
    fn rejects() {
        assert!(Vyper::parse(b"plain text").is_none());
        assert!(Vyper::parse(b"").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
