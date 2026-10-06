//! commitlint 設定(`commitlint.config.js`/`commitlint.config.cjs`/
//! `.commitlintrc`(JSON/YAML)/`package.json` の `commitlint` キー)の
//! 検出と構造カウント。
//!
//! `extends:`(`@commitlint/config-*`)/`rules:`(`type-enum`/`subject-empty`/
//! `header-max-length`/`scope-enum`/`body-max-line-length`/`footer-*`/`case`/
//! `references-empty`/`signed-off-by`/`type-empty` 等のルールキー)を識別する。
//!
//! ```
//! let c = izanagi_kit::commitlint::parse(
//!     b"module.exports = {\n  extends: ['@commitlint/config-conventional'],\n  rules: {\n    'type-enum': [2, 'always', ['feat','fix']],\n    'subject-empty': [2, 'never']\n  }\n}\n").unwrap();
//! assert_eq!(c.extends, 1);
//! assert_eq!(c.rules, 2);
//! assert!(izanagi_kit::commitlint::detect(
//!     b"extends: ['@commitlint/config-conventional']\nrules:\n  type-empty: [2, 'never']\n"));
//! ```

/// `"key"` が値位置ではなくキー位置(直後が `:`)にあるかを確認。
fn jkey(t: &str, key: &str) -> bool {
    let pat = format!("\"{key}\"");
    let mut rest = t;
    while let Some(i) = rest.find(&pat) {
        rest = &rest[i + pat.len()..];
        if rest.trim_start().starts_with(':') {
            return true;
        }
    }
    false
}

/// 行頭が `key:`(JS/YAML 共通、引用符任意)かどうか。
fn is_key_line(tr: &str, key: &str) -> bool {
    let t = tr.trim_start_matches(['"', '\'']);
    t.strip_prefix(key).is_some_and(|r| {
        r.trim_start_matches(['"', '\''])
            .trim_start()
            .starts_with(':')
    })
}

/// commitlint ルール名(サフィックス比較用)。
const RULE_SUFFIXES: &[&str] = &[
    "-enum",
    "-empty",
    "-case",
    "-length",
    "-full-stop",
    "-leading-blank",
    "-max-length",
    "-min-length",
    "-signed-off-by",
];

/// `tr` がルールキー行(`'type-enum':`/`"subject-empty":`/`type-enum:`)か。
fn is_rule_key(tr: &str) -> bool {
    let t = tr.trim_end_matches(',');
    let Some(colon) = t.find(':') else {
        return false;
    };
    let k = t[..colon].trim().trim_matches('"').trim_matches('\'');
    RULE_SUFFIXES.iter().any(|s| k.ends_with(s))
}

/// commitlint 設定の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `extends:` 参照数(`@commitlint/*`/`./path` 要素)。
    pub extends: usize,
    /// ルールキー行数。
    pub rules: usize,
    /// `ignores:`/`prompt:`/`helpUrl` 等の補助キー行数。
    pub option_keys: usize,
    /// `//`/`#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が commitlint 設定に見えるかを判定する。
///
/// `@commitlint` 参照、または `extends`/`rules` キーと commitlint 固有の
/// ルールキー(`type-enum`/`subject-empty`/…)を要求する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    if t.contains("@commitlint") || jkey(t, "commitlint") {
        return true;
    }
    let rules_hit = t.lines().filter(|l| is_rule_key(l.trim())).count();
    rules_hit >= 1
        && (jkey(t, "extends")
            || jkey(t, "rules")
            || t.lines()
                .any(|l| is_key_line(l.trim(), "extends") || is_key_line(l.trim(), "rules")))
}

/// `b` を commitlint 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        extends: t.matches("@commitlint/").count(),
        rules: 0,
        option_keys: 0,
        comments: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
            c.comments += 1;
            continue;
        }
        if is_rule_key(tr) {
            c.rules += 1;
            continue;
        }
        let k = tr.trim_start_matches(['"', '\'']);
        if k.starts_with("extends")
            || k.starts_with("rules")
            || k.starts_with("ignores")
            || k.starts_with("prompt")
            || k.starts_with("helpUrl")
            || k.starts_with("formatter")
            || k.starts_with("defaultIgnores")
        {
            c.option_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JS: &[u8] = b"module.exports = {\n  extends: ['@commitlint/config-conventional'],\n  rules: {\n    'type-enum': [2, 'always', ['feat','fix']],\n    'subject-empty': [2, 'never'],\n    'header-max-length': [2, 'always', 100]\n  }\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE_JS));
        assert!(detect(
            b"{\n  \"extends\": [\"@commitlint/config-conventional\"],\n  \"rules\": {}\n}\n"
        ));
        assert!(detect(
            b"extends: ['@commitlint/config-conventional']\nrules:\n  type-enum: [2, 'always', [feat]]\n"
        ));
        assert!(!detect(b"extends: [\"base\"]\nrules:\n  indent: [2, 4]\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE_JS).unwrap();
        assert_eq!(c.extends, 1);
        assert_eq!(c.rules, 3);
        assert!(parse(b"{\"name\": \"x\"}\n").is_none());
    }
}
