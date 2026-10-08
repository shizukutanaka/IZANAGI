//! mise(`mise.toml`/`.mise.toml`/`config.toml`)の検出と構造カウント。
//!
//! `[tools]` セクション配下の `tool = "version"` エントリを中核に、
//! `[env]`/`[tasks]`/`[plugins]`/`[settings]`/`[alias]`/`[hooks]`/
//! `[watch_files]`/`[vars]` 等の mise セクションを識別する。
//! 一般 TOML との誤検出を避けるため、`[tools]` セクションと既知ツール名
//! (または `[plugins]`/`[alias]`/`[watch_files]`/`[vars]`/`[hooks]`
//! セクション)の組合せを要求する。
//!
//! ```
//! let b = b"[tools]\nnode = \"20\"\npython = \"3.12\"\n\n[env]\nDEBUG = \"1\"\n\n[tasks.test]\nrun = \"cargo test\"\n";
//! assert!(izanagi_kit::mise::detect(b));
//! let c = izanagi_kit::mise::parse(b).unwrap();
//! assert_eq!(c.tools, 2);
//! ```

/// TOML セクションヘッダ `[name]`(配列表 `[[name]]` は別扱い)。
fn section_of(tr: &str) -> Option<&str> {
    tr.strip_prefix('[')?
        .strip_suffix(']')
        .map(|s| s.trim())
        .filter(|s| !s.starts_with('[') && !s.is_empty())
}

/// `key = value` 行のキー部(引用符なし前提)。
fn toml_key(tr: &str) -> Option<&str> {
    tr.split_once('=')
        .map(|(k, _)| k.trim())
        .filter(|k| !k.is_empty())
}

/// mise の `[tools]` で一般的なツール名。
const TOOL_NAMES: &[&str] = &[
    "bun",
    "cmake",
    "deno",
    "docker",
    "dotnet",
    "elixir",
    "erlang",
    "fd",
    "go",
    "golang",
    "gradle",
    "helm",
    "java",
    "jq",
    "kotlin",
    "kubectl",
    "maven",
    "minikube",
    "ninja",
    "node",
    "nodejs",
    "perl",
    "php",
    "poetry",
    "postgres",
    "python",
    "ruby",
    "rust",
    "scala",
    "swift",
    "terraform",
    "uv",
    "yarn",
    "yq",
    "zig",
];

/// `[tools]` 以外の mise 固有セクション名。
const MISE_SECTIONS: &[&str] = &[
    "alias",
    "env",
    "hooks",
    "plugins",
    "settings",
    "tasks",
    "vars",
    "watch_files",
];

/// `mise.toml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `[tools]` セクション内の `tool = version` エントリ数。
    pub tools: usize,
    /// 認識したセクションヘッダ数(`[tools]`/`[env]`/`[tasks]`/…)。
    pub sections: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `b` が `mise.toml` に見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let t = strip_bom(t);
    let mut in_tools = false;
    let mut has_tools_section = false;
    let mut has_known_tool = false;
    let mut has_mise_section = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if let Some(s) = section_of(tr) {
            let top = s.split('.').next().unwrap_or(s);
            in_tools = top == "tools";
            has_tools_section |= in_tools;
            has_mise_section |= MISE_SECTIONS.contains(&top);
            continue;
        }
        if in_tools {
            if let Some(k) = toml_key(tr) {
                has_known_tool |= TOOL_NAMES.contains(&k);
            }
        }
    }
    has_tools_section && (has_known_tool || has_mise_section)
}

/// `b` を `mise.toml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let t = strip_bom(t);
    let mut c = Counts {
        tools: 0,
        sections: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_tools = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if let Some(s) = section_of(tr) {
            let top = s.split('.').next().unwrap_or(s);
            in_tools = top == "tools";
            if in_tools || MISE_SECTIONS.contains(&top) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if in_tools && toml_key(tr).is_some() {
            c.tools += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] =
        b"[tools]\nnode = \"20\"\npython = \"3.12\"\n\n[env]\nDEBUG = \"1\"\n\n[tasks.test]\nrun = \"cargo test\"\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"[tools]\nnode = \"lts\"\n[alias]\nnode20 = \"node:20\"\n"
        ));
        // `[tools]` だけで既知ツールも mise セクションもなければ検出しない。
        assert!(!detect(b"[tools]\nhammer = \"1\"\n"));
        assert!(!detect(b"[package]\nname = \"x\"\nversion = \"1\"\n"));
        assert!(!detect(b"[dependencies]\nserde = \"1\"\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tools, 2);
        assert_eq!(c.sections, 3);
        assert!(parse(b"[package]\nname = \"x\"\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
