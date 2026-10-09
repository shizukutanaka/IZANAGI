//! Netlify `netlify.toml` サイト設定の認識と計数。
//!
//! `netlify.toml` は TOML で、`[build]`(コマンド・publish・functions・
//! `command`/`base`)、`[functions]`、`[[redirects]]`/`[[headers]]`/
//! `[[plugins]]`/`[[edge_functions]]`/`[[dev]]` の配列テーブル、
//! `[context.production]`/`[context.deploy-preview]`/`[context.branch-deploy]`
//! のコンテキスト別上書きを持つ。`_redirects` ファイル相当のルールは
//! `from`/`to`/`status`/`force`/`conditions` キーを取る。
//!
//! ```
//! let b = b"[build]\n  command = \"npm run build\"\n  publish = \"dist\"\n  functions = \"netlify/functions\"\n\n[build.environment]\n  NODE_VERSION = \"20\"\n\n[[redirects]]\n  from = \"/api/*\"\n  to = \"/.netlify/functions/:splat\"\n  status = 200\n\n[[headers]]\n  for = \"/*\"\n  [headers.values]\n    X-Frame-Options = \"DENY\"\n\n[[plugins]]\n  package = \"@netlify/plugin-lighthouse\"\n\n[context.deploy-preview]\n  command = \"npm run preview\"\n";
//! assert!(izanagi_kit::netlifyconf::detect(b));
//! let c = izanagi_kit::netlifyconf::parse(b).unwrap();
//! assert_eq!(c.assigns, 11);
//! assert_eq!(c.tables, 4); // [build] [build.environment] [headers.values] [context.deploy-preview]
//! assert_eq!(c.array_tables, 3); // redirects headers plugins
//! assert_eq!(c.contexts, 1); // context.deploy-preview
//! assert_eq!(c.redirect_rules, 1);
//! ```

use crate::textutil::strip_bom;
/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` 行の個数。
    pub assigns: usize,
    /// `[table]` 行の個数(`[[array]]` を除く)。
    pub tables: usize,
    /// `[[array-table]]` 行の個数。
    pub array_tables: usize,
    /// `[[redirects]]`/`[[rewrites]]` エントリの個数。
    pub redirect_rules: usize,
    /// `[context.<name>]` の個数。
    pub contexts: usize,
    /// `[[headers]]`/`[[plugins]]`/`[[edge_functions]]`/`[[dev]]`/`[[functions]]`
    /// 配列テーブルの個数。
    pub feature_tables: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const FEATURE_ARRAYS: &[&str] = &[
    "headers",
    "plugins",
    "edge_functions",
    "dev",
    "functions",
    "images",
];

/// `netlify.toml` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        for key in [
            "[build]",
            "[[redirects]]",
            "[[headers]]",
            "[[plugins]]",
            "[context.",
            "netlify",
            "command =",
            "publish =",
        ] {
            if s.starts_with(key) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        assigns: 0,
        tables: 0,
        array_tables: 0,
        redirect_rules: 0,
        contexts: 0,
        feature_tables: 0,
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
        if s.starts_with("[[") {
            c.array_tables += 1;
            let inner = s.trim_start_matches('[').trim_end_matches(']');
            if inner == "redirects" || inner == "rewrites" {
                c.redirect_rules += 1;
            } else if FEATURE_ARRAYS.contains(&inner) {
                c.feature_tables += 1;
            }
            continue;
        }
        if s.starts_with('[') {
            c.tables += 1;
            let inner = s.trim_start_matches('[').trim_end_matches(']');
            if inner.starts_with("context.") {
                c.contexts += 1;
            }
            continue;
        }
        let Some(eq) = s.find('=') else {
            continue;
        };
        let key = s[..eq]
            .trim_end()
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'))
            .next()
            .unwrap_or("");
        if !key.is_empty() {
            c.assigns += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_netlify() {
        assert!(detect(
            b"[build]\n  command = \"x\"\n[[redirects]]\n  from = \"/a\"\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"[build]\n  command = \"npm run build\"\n  publish = \"dist\"\n\n[[redirects]]\n  from = \"/a\"\n  to = \"/b\"\n[[redirects]]\n  from = \"/c\"\n  to = \"/d\"\n\n[context.branch-deploy]\n  command = \"x\"\n[[plugins]]\n  package = \"p\"\n";
        let c = parse(b).unwrap();
        assert_eq!(c.assigns, 8);
        assert_eq!(c.tables, 2);
        assert_eq!(c.array_tables, 3);
        assert_eq!(c.redirect_rules, 2);
        assert_eq!(c.contexts, 1);
        assert_eq!(c.feature_tables, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"[build]\n  command = \"x\"\n[[redirects]]\n  from = \"a\"\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
