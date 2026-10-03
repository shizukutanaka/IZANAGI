//! release-please `release-please-config.json` / `.release-please-manifest.json`
//! の検出・カウント。
//!
//! `packages`/`release-type`/`changelog-path`/`separate-major-releases`
//! 等 release-please 固有キーを持つ JSON 設定。
//!
//! ```
//! let cfg = br#"{"packages":{"pkg":{"release-type":"rust"}},"separate-major-releases":true}"#;
//! assert!(izanagi_kit::releaseplease::detect(cfg));
//! let c = izanagi_kit::releaseplease::parse(cfg).unwrap();
//! assert_eq!(c.packages, 2);
//! ```

/// リリース挙動系キー。
const RELEASE: &[&str] = &[
    "release-type",
    "separate-major-releases",
    "signoff",
    "draft",
    "draft-pull-request",
    "prerelease",
    "prerelease-branch",
    "tag-separator",
    "include-component-in-tag",
    "include-v-in-tag",
    "initial-version",
    "bump-patch-for-minor-pre-major",
    "sequential-calls",
    "always-update",
    "workspace-group",
    "linked-versions",
    "monorepo-tags",
    "pull-request-header",
    "pull-request-title-pattern",
    "pull-request-footer",
    "group-pull-request-title-pattern",
    "versioning",
    "snapshot-labels",
    "skip-github-pull-requests",
];

/// 変更履歴系キー。
const CHANGELOG: &[&str] = &[
    "changelog-path",
    "changelog-sections",
    "changelog-type",
    "changelog-host",
    "changelog-notes-type",
    "changelog-file",
];

/// メタ・バージョン追跡系キー。
const META: &[&str] = &[
    "$schema",
    "version",
    "bootstrap-sha",
    "last-release-sha",
    "release-labels",
    "skip-labeling",
    "labels",
    "plugins",
    "commit-search-depth",
    "tag-search-depth",
];

/// パッケージ内既知キー。
const PKG_KEYS: &[&str] = &[
    "release-type",
    "component",
    "version-file",
    "extra-files",
    "package-name",
    "exclude-paths",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// JSON キートークン総数。
    pub entries: usize,
    /// `packages` 配下の `release-type`/`component`/`version-file` 等パッケージ指定キー数。
    pub packages: usize,
    /// `separate-major-releases`/`draft`/`prerelease` 等リリース挙動系キー数。
    pub release: usize,
    /// `changelog-*` 系キー数。
    pub changelog: usize,
    /// `$schema`/`version`/`bootstrap-sha`/`labels` 等メタ系キー数。
    pub meta: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が release-please 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.packages + c.release + c.changelog + c.meta >= 2 && c.packages >= 1)
}

/// キートークンを走査 (`"name"` の直後が `:` のもの)。
fn keys(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            let mut esc = false;
            while j < bytes.len() {
                if esc {
                    esc = false;
                } else if bytes[j] == 0x5C {
                    esc = true;
                } else if bytes[j] == b'"' {
                    break;
                }
                j += 1;
            }
            let mut k = j + 1;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b':' && j > start {
                out.push(&text[start..j]);
            }
            i = k;
        } else {
            i += 1;
        }
    }
    out
}

/// `b` を release-please 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let keys = keys(text);
    if keys.is_empty() {
        return None;
    }
    let mut c = Counts {
        entries: keys.len(),
        packages: 0,
        release: 0,
        changelog: 0,
        meta: 0,
        misc: 0,
    };
    for k in keys {
        if k == "packages" || PKG_KEYS.contains(&k) {
            c.packages += 1;
        } else if CHANGELOG.contains(&k) {
            c.changelog += 1;
        } else if META.contains(&k) {
            c.meta += 1;
        } else if RELEASE.contains(&k) {
            c.release += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.packages >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn releaseplease() {
        let cfg = br#"{"$schema":"s","packages":{"crates/a":{"release-type":"rust","component":"a","version-file":"v.txt"}},"separate-major-releases":true,"changelog-path":"CHANGELOG.md","labels":["release"]}"#;
        let c = parse(cfg).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.packages, 4);
        assert_eq!(c.release, 1);
        assert_eq!(c.changelog, 1);
        assert_eq!(c.meta, 2);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_releaseplease() {
        assert!(parse(br#"{"a":1}"#).is_none());
    }
}
