//! lint-staged 設定(`.lintstagedrc`(JSON/YAML)/`lint-staged.config.js`/
//! `package.json` の `lint-staged` キー)の検出と構造カウント。
//!
//! 専用ファイルはファイルグロブ(`"*.{js,ts}"`/`"src/**"`/…)→コマンド列の
//! マッピング構造を持つ。埋め込み形は `lint-staged` キー名で識別する。
//!
//! ```
//! let c = izanagi_kit::lintstaged::parse(
//!     b"{\n  \"*.{js,ts}\": [\"eslint --fix\"],\n  \"*.md\": \"prettier --write\"\n}\n").unwrap();
//! assert_eq!(c.globs, 2);
//! assert!(izanagi_kit::lintstaged::detect(
//!     b"{\n  \"*.rs\": \"cargo fmt\"\n}\n"));
//! ```

/// 行頭がファイルグロブ(`*.ext`/`*.{a,b}`/`src/**`/`foo/*.rs`…)のキー行かどうか。
fn is_glob_key(tr: &str) -> bool {
    let t = tr.trim_start_matches('"').trim_start_matches('\'');
    let Some(colon) = t.find(':') else {
        return false;
    };
    let k = &t[..colon];
    k.contains('*') && (k.starts_with('*') || k.contains('/'))
}

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

/// lint-staged 設定の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// グロブキー行数。
    pub globs: usize,
    /// コマンド文字列要素数(配列要素 + スカラー値)。
    pub commands: usize,
    /// `lint-staged` キー行数。
    pub lint_staged_keys: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が lint-staged 設定に見えるかを判定する。
///
/// `lint-staged` キー名、またはグロブ→コマンドのマッピング構造
/// (グロブキー行 1 以上)を要求する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    if jkey(t, "lint-staged")
        || t.lines().any(|l| {
            let tr = l.trim().trim_start_matches(['"', '\'']);
            tr.starts_with("lint-staged")
        })
    {
        return true;
    }
    t.lines().filter(|l| is_glob_key(l.trim())).count() >= 1 && (t.contains('{') || t.contains(':'))
}

/// `b` を lint-staged 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        globs: 0,
        commands: 0,
        lint_staged_keys: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr == "{" || tr == "}" || tr == "]" || tr == "[" {
            continue;
        }
        if is_glob_key(tr) {
            c.globs += 1;
            // 値側の引用符付き文字列をコマンドとして数える
            // (スカラー `"cmd"` でも配列 `["a", "b"]` でも同一行上なら)。
            if let Some(colon) = tr.find(':') {
                let v = &tr[colon + 1..];
                c.commands += v.matches('"').count() / 2;
            }
            continue;
        }
        if tr
            .trim_start_matches(['"', '\''])
            .starts_with("lint-staged")
        {
            c.lint_staged_keys += 1;
            continue;
        }
        // 配列中のコマンド文字列要素 `"cmd",` / `"cmd"`
        let s = tr.trim_end_matches(',');
        if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
            if s.len() > 2 {
                c.commands += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"*.{js,ts,tsx}\": [\"eslint --fix\", \"prettier --write\"],\n  \"*.md\": \"prettier --write\",\n  \"src/**/*.rs\": \"cargo fmt\"\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(b"{\"lint-staged\": {\"*.js\": \"eslint\"}}\n"));
        assert!(detect(b"*.rs: cargo fmt\n"));
        assert!(!detect(b"{\"name\": \"x\", \"version\": \"1\"}\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.globs, 3);
        assert!(c.commands >= 4);
        assert!(parse(b"{\"name\": \"x\"}\n").is_none());
    }
}
