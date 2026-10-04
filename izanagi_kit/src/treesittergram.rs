//! tree-sitter grammar definition (`grammar.js`/`grammar.json`) parser.
//!
//! Detects tree-sitter grammars by `grammar({`/`module.exports = grammar`
//! with `name:`/`rules:`/`extras:`/`externals:`/`conflicts:`/`precedences:`/
//! `supertypes:`/`inline:`/`word:`/`reserved:` blocks and DSL calls
//! (`$ =>`/`seq(`/`choice(`/`repeat(`/`repeat1(`/`optional(`/`token(`/
//! `prec(`/`prec.left(`/`prec.right(`/`prec.dynamic(`/`field(`/`alias(`/
//! `sym(`/`blank(`/`commaSep(`/`sep1(`).
//!
//! ```
//! let b = b"module.exports = grammar({\n  name: 'demo',\n  rules: {\n    source_file: $ => repeat($._stmt),\n    _stmt: $ => choice($.expr, $.decl),\n  },\n  extras: $ => [$.comment],\n});\n";
//! assert!(izanagi_kit::treesittergram::detect(b));
//! let c = izanagi_kit::treesittergram::Tsgram::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed tree-sitter grammar summary.
#[derive(Debug, Clone)]
pub struct Tsgram {
    /// Recognized key/call occurrences.
    pub keys: usize,
    /// Grammar meta keys (`grammar(`/`module.exports`/`name:`/`rules:`/`extras:`/`externals:`/`conflicts:`/`precedences:`/`supertypes:`/`inline:`/`word:`/`reserved:`/`precedences:`).
    pub meta_keys: usize,
    /// DSL call sites (`$ =>`/`seq(`/`choice(`/`repeat(`/`repeat1(`/`optional(`/`token(`/`token.immediate(`/`prec(`/`prec.left(`/`prec.right(`/`prec.dynamic(`/`field(`/`alias(`/`sym(`/`blank(`/`commaSep(`/`sep1(`/`paren(`).
    pub rule_keys: usize,
    /// JSON-form keys (`"name"`/`"type"`/`"value"`/`"content"`/`"members"`/`"rules"`/`"arguments"`/`"named"`/`"immediate"` — `.json` grammars counted via these tokens).
    pub json_keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

/// Meta keys.
const META_KEYS: &[&str] = &[
    "grammar(",
    "module.exports",
    "name:",
    "rules:",
    "extras:",
    "externals:",
    "conflicts:",
    "precedences:",
    "supertypes:",
    "inline:",
    "word:",
    "reserved:",
];

/// DSL calls.
const RULE_KEYS: &[&str] = &[
    "$ =>",
    "seq(",
    "choice(",
    "repeat(",
    "repeat1(",
    "optional(",
    "token(",
    "token.immediate(",
    "prec(",
    "prec.left(",
    "prec.right(",
    "prec.dynamic(",
    "field(",
    "alias(",
    "sym(",
    "blank(",
    "commaSep(",
    "sep1(",
];

/// JSON-form keys.
const JSON_KEYS: &[&str] = &[
    "\"name\"",
    "\"type\"",
    "\"value\"",
    "\"content\"",
    "\"members\"",
    "\"rules\"",
    "\"arguments\"",
    "\"named\"",
    "\"immediate\"",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["grammar(", "$ =>", "module.exports"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "grammar(",
    "module.exports = grammar",
    "rules:",
    "$ =>",
    "seq(",
    "choice(",
    "repeat(",
    "field(",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a tree-sitter grammar.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Tsgram {
    /// Count categories in a grammar.js. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            meta_keys: 0,
            rule_keys: 0,
            json_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
            }
        }
        for k in META_KEYS {
            c.meta_keys += t.matches(k).count();
        }
        for k in RULE_KEYS {
            c.rule_keys += t.matches(k).count();
        }
        for k in JSON_KEYS {
            c.json_keys += t.matches(k).count();
        }
        c.keys = c.meta_keys + c.rule_keys + c.json_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"// grammar\nmodule.exports = grammar({\n  name: 'demo',\n  rules: {\n    source_file: $ => repeat($._stmt),\n    _stmt: $ => choice($.expr, $.decl),\n    expr: $ => seq($.ident, field('op', $.oper)),\n  },\n  extras: $ => [$.comment, /\\s/],\n  conflicts: $ => [[$.a, $.b]],\n  inline: $ => [$._stmt],\n});\n";
        assert!(detect(b));
        let c = Tsgram::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.meta_keys >= 4);
        assert!(c.rule_keys >= 6);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_js() {
        assert!(!detect(b"function f() { return 1; }\n"));
        assert!(Tsgram::parse(b"a = b\n").is_none());
    }
}
