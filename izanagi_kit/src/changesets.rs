//! Changesets 設定(`.changeset/config.json`)とチェンジセットファイル
//! (`.changeset/*.md` の frontmatter `"pkg": patch|minor|major`)の
//! 検出と構造カウント。
//!
//! `config.json` は `changelog`/`baseBranch`/`commit`/`access`/`fixed`/
//! `linked`/`updateInternalDependencies`/`ignore`/`snapshot`/`privatePackages`
//! キーを持つ。チェンジセット md は `---` 区切りの semver 指定を持つ。
//!
//! ```
//! let c = izanagi_kit::changesets::parse(
//!     b"{\n  \"changelog\": \"@changesets/cli/changelog\",\n  \"commit\": false,\n  \"baseBranch\": \"main\",\n  \"access\": \"restricted\"\n}\n").unwrap();
//! assert!(c.config_keys >= 4);
//! assert!(izanagi_kit::changesets::detect(
//!     b"---\n\"my-pkg\": patch\n---\n\nFix the bug.\n"));
//! ```

use crate::textutil::strip_bom;
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

/// `tr` が `"pkg": patch|minor|major` 形式の frontmatter 行かどうか。
fn is_semver_line(tr: &str) -> bool {
    let Some(colon) = tr.find(':') else {
        return false;
    };
    let k = tr[..colon].trim().trim_matches('"').trim_matches('\'');
    let v = tr[colon + 1..]
        .trim()
        .trim_end_matches(',')
        .trim_matches('"')
        .trim_matches('\'');
    !k.is_empty()
        && matches!(
            v,
            "patch" | "minor" | "major" | "prepatch" | "preminor" | "premajor"
        )
}

/// Changesets の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `config.json` 既知キー行数。
    pub config_keys: usize,
    /// frontmatter の semver 指定行数(パッケージ数)。
    pub semver_entries: usize,
    /// `---` 区切り行数。
    pub fences: usize,
    /// `//`/`#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// 既知 config キー。
const CONFIG_KEYS: &[&str] = &[
    "access",
    "baseBranch",
    "changelog",
    "commit",
    "fixed",
    "ignore",
    "linked",
    "privatePackages",
    "snapshot",
    "updateInternalDependencies",
];

/// `b` が Changesets 関連ファイルに見えるかを判定する。
///
/// `config.json` 判定は `changelog`+`baseBranch`+他キーの組合せ。
/// チェンジセット md は `---` ブロック内の semver 指定行で判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let t = strip_bom(t);
    if jkey(t, "changelog") && jkey(t, "baseBranch") {
        return true;
    }
    // `---` で始まる frontmatter + semver 指定行。
    t.trim_start().starts_with("---") && t.lines().any(|l| is_semver_line(l.trim()))
}

/// `b` を Changesets 関連ファイルとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let t = strip_bom(t);
    let mut c = Counts {
        config_keys: 0,
        semver_entries: 0,
        fences: 0,
        comments: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with("//") || tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("---") {
            c.fences += 1;
            continue;
        }
        if is_semver_line(tr) {
            c.semver_entries += 1;
            continue;
        }
        if CONFIG_KEYS.iter().any(|k| jkey(tr, k)) {
            c.config_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &[u8] = b"{\n  \"changelog\": \"@changesets/cli/changelog\",\n  \"commit\": false,\n  \"fixed\": [],\n  \"linked\": [],\n  \"access\": \"restricted\",\n  \"baseBranch\": \"main\",\n  \"updateInternalDependencies\": \"patch\",\n  \"ignore\": []\n}\n";

    const CHANGESET: &[u8] =
        b"---\n\"my-pkg\": patch\n\"other-pkg\": minor\n---\n\nFixed a bug in the thing.\n";

    #[test]
    fn detect_works() {
        assert!(detect(CONFIG));
        assert!(detect(CHANGESET));
        assert!(!detect(b"{\"name\": \"x\", \"baseBranch\": \"main\"}\n"));
        assert!(!detect(b"---\ntitle: doc\n---\n"));
    }

    #[test]
    fn parses_config() {
        let c = parse(CONFIG).unwrap();
        assert!(c.config_keys >= 6);
    }

    #[test]
    fn parses_changeset() {
        let c = parse(CHANGESET).unwrap();
        assert_eq!(c.semver_entries, 2);
        assert_eq!(c.fences, 2);
        assert!(parse(b"plain markdown\n---\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
