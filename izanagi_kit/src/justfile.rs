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
/// `set` に続く設定名。`set` 自体は AMPL 等の宣言語でもあるため、
/// just の設定名を伴う場合だけディレクティブとみなす。
const SETTINGS: &[&str] = &[
    "allow",
    "allow-duplicate-recipes",
    "dotenv-filename",
    "dotenv-load",
    "dotenv-path",
    "dotenv-required",
    "fallback",
    "ignore-comments",
    "positional-arguments",
    "quiet",
    "script-interpreter",
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

/// `name := expr` or `export name := expr` — a just assignment.
/// `:=` alone is not just-exclusive: Coq (`Definition x := …`), Lean,
/// Pascal/Ada and AMPL assign with `:=` too, so the left-hand side must
/// be exactly one identifier (optionally preceded by `export`).
fn is_assignment(t: &str) -> bool {
    if !t.contains(":=") {
        return false;
    }
    let lhs = t.split(":=").next().unwrap_or("").trim_end();
    let mut it = lhs.split_whitespace();
    let name = match (it.next(), it.next(), it.next()) {
        (Some(n), None, None) => n,
        (Some("export"), Some(n), None) | (Some("alias"), Some(n), None) => n,
        _ => return false,
    };
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A just directive line. The keyword alone is not evidence: `set`,
/// `import`, `mod`, `export` and `alias` are ordinary words in AMPL,
/// Coq/Lean, ASN.1 and shell configs — each must carry just-shaped
/// arguments.
fn is_directive(t: &str) -> bool {
    let mut it = t.split_whitespace();
    let kw = it.next();
    let arg = it.next().unwrap_or("");
    match kw {
        // `set <known-setting>` — a bare `set x :=` is AMPL, not just
        Some("set") => {
            let name = arg.split('=').next().unwrap_or("");
            SETTINGS.contains(&name.trim_end_matches(":=").trim_end())
        }
        // `import 'path'` / `import "path"` — unquoted imports are
        // Lean/Rust module statements, not just
        Some("import") | Some("mod") => arg.starts_with('\'') || arg.starts_with('"'),
        _ => false,
    }
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
            // `@name:`/`!name:` private/silent recipes are just-only;
            // `name arg:` params are not — `key word:` is an ordinary
            // shape in YAML lists, schema langs and Dockerfiles alike
            if t.starts_with('@') || t.starts_with('!') {
                distinctive += 1;
            }
        } else {
            last_was_recipe = false;
            if is_assignment(t) || is_directive(t) {
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
        } else if is_assignment(t) || is_directive(t) {
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
        // generic keyword lines and `name arg:` headers are not just —
        // YAML list items, schema declarations, AMPL/Coq `set`/`:=`
        // statements and unquoted `import`/`mod` must not count
        assert!(!detect(b"plugins:\n- name: tap-gitlab\n  variant: m\n"));
        assert!(!detect(b"schema Person:\n  name: str\n  age: int = 0\n"));
        assert!(!detect(b"set DIM := 1..3\nparam p{DIM} >= 0\n"));
        assert!(!detect(b"Cert ::= SEQUENCE {\n  version INTEGER\n}\n"));
        assert!(!detect(
            b"import Mathlib.Tactic\ntheorem t : True := by\n  trivial\n"
        ));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
