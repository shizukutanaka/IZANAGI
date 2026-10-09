//! just の `justfile`/`Justfile` の検出と構造カウント。
//!
//! `name:`/`name args: dep` レシピ定義、`x := "v"` 変数代入、
//! `set`/`alias`/`import`/`export`/`mod`/`dotenv-*` ディレクティブ、
//! `#` コメントを識別する。
//!
//! ```
//! let c = izanagi_kit::justfile::parse(
//!     b"set dotenv-load\nname := \"x\"\n\nbuild: deps\n    cargo build\n\n@lint:\n    cargo clippy\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::justfile::detect(b"@run:\n    echo hi\n"));
//! ```

use crate::textutil::strip_bom;
/// 先頭ディレクティブ。
const DIRECTIVES: &[&str] = &[
    "alias",
    "dotenv-filename",
    "dotenv-load",
    "dotenv-path",
    "dotenv-required",
    "export",
    "fallback",
    "import",
    "mod",
    "positional-arguments",
    "script-interpreter",
    "set",
    "shell",
    "tempdir",
    "unstable",
    "windows-powershell",
    "windows-shell",
    "working-directory",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// レシピ定義行数。
    pub sections: usize,
    /// ディレクティブ + `:=` 代入行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数(レシピ本体等)。
    pub misc: usize,
}

fn recipe_header(t: &str) -> bool {
    let t = t.strip_prefix('@').unwrap_or(t);
    let t = t.strip_prefix('!').unwrap_or(t);
    let colon = match t.find(':') {
        Some(c) => c,
        None => return false,
    };
    if t[colon + 1..].starts_with('=') {
        return false;
    }
    let mut it = t[..colon].split_whitespace();
    let name = match it.next() {
        Some(n) => n,
        None => return false,
    };
    if name.contains('=')
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return false;
    }
    // パラメータは `*`/`+`/`$`/`=`/引用符/通常トークンを許容。
    it.all(|p| {
        p.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '-' | '_' | '*' | '+' | '$' | '=' | '\'' | '"' | '/')
        })
    })
}

/// justfile らしさを判定する。
///
/// `name: value` 行は YAML と同形なので、レシピ本体(次行のインデント)または
/// `:=`/`set` 等の just 固有構文を要求する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut recipes = 0usize;
    let mut just_syntax = 0usize;
    let mut distinctive = 0usize;
    let mut saw_body = false;
    let mut last_was_recipe = false;
    for line in text.lines() {
        if line.starts_with(char::is_whitespace) {
            if last_was_recipe && !line.trim().is_empty() && !line.trim().starts_with('#') {
                saw_body = true;
                // `{{var}}` interpolation is just-only syntax
                if line.contains("{{") {
                    distinctive += 1;
                }
            }
            continue;
        }
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            last_was_recipe = false;
            continue;
        }
        if recipe_header(t) {
            recipes += 1;
            last_was_recipe = true;
            // `@name:`/`!name:` private/silent recipes and
            // `name arg="x":` parameter defaults don't exist in YAML
            if t.starts_with('@') || t.starts_with('!') {
                distinctive += 1;
            } else {
                let head = t.strip_prefix('@').unwrap_or(t);
                if let Some(c) = head.find(':') {
                    if head[..c].split_whitespace().nth(1).is_some() {
                        distinctive += 1;
                    }
                }
            }
        } else {
            last_was_recipe = false;
            if t.contains(":=")
                || DIRECTIVES
                    .iter()
                    .any(|d| t == *d || t.starts_with(&format!("{d} ")))
            {
                just_syntax += 1;
            }
        }
    }
    // `name:` + indented body is exactly the YAML mapping shape — a
    // recipe list only counts when just-exclusive syntax is present
    recipes >= 1 && (just_syntax >= 1 || (saw_body && distinctive >= 1))
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            c.misc += 1;
            continue;
        }
        if recipe_header(t) {
            c.sections += 1;
        } else if t.contains(":=")
            || DIRECTIVES
                .iter()
                .any(|d| t == *d || t.starts_with(&format!("{d} ")))
        {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# sample\nset dotenv-load\nset shell := [\"bash\", \"-c\"]\nname := \"x\"\nexport VER := \"1\"\n\nbuild target=\"all\": deps lint\n    cargo build {{target}}\n\ndeps:\n    cargo fetch\n\n@lint:\n    cargo clippy\n\ntest: build\n    cargo test\n";

    #[test]
    fn justfile() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.options, 4);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 4);
    }

    #[test]
    fn not_justfile() {
        assert!(!detect(b"key: value\nother: 1\n"));
        // `name:` + indented body is the YAML mapping shape — no
        // just-exclusive syntax means not a justfile
        assert!(!detect(b"build:\n  cargo build\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
