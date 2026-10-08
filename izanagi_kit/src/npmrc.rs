//! npm 設定ファイル(`.npmrc`)の検出と構造カウント。
//!
//! `key=value` 既知キー(`registry`/`prefix`/`cache`/`save-exact`/`audit`/
//! `engine-strict`/`package-lock`/`strict-ssl`/`tag`/`fund`/`userconfig` 等)と
//! スコープ設定(`//host/path/:key=value`・`@scope:key=value`)を識別する。
//!
//! ```
//! let c = izanagi_kit::npmrc::parse(
//!     b"registry=https://registry.npmjs.org/\nsave-exact=true\n//reg.example.com/:_authToken=x\n").unwrap();
//! assert_eq!(c.options, 2);
//! assert_eq!(c.scoped, 1);
//! assert!(izanagi_kit::npmrc::detect(b"engine-strict=true\npackage-lock=false\n"));
//! ```

/// 既知 npm 設定キー(抜粋)。
const KEYS: &[&str] = &[
    "access",
    "audit",
    "audit-level",
    "auth-type",
    "before",
    "bin-links",
    "ca",
    "cache",
    "cache-lock-retries",
    "cache-lock-stale",
    "cache-lock-wait",
    "cache-min",
    "cafile",
    "call",
    "cert",
    "ci-name",
    "cidr",
    "color",
    "commit-hooks",
    "depth",
    "description",
    "dev",
    "dry-run",
    "editor",
    "engine-strict",
    "fetch-retries",
    "fetch-retry-factor",
    "fetch-retry-maxtimeout",
    "fetch-retry-mintimeout",
    "fetch-timeout",
    "force",
    "foreground-scripts",
    "format-package-lock",
    "fund",
    "git",
    "git-tag-version",
    "global",
    "globalconfig",
    "global-style",
    "group",
    "heading",
    "https-proxy",
    "ignore-scripts",
    "include",
    "init-author-email",
    "init-author-name",
    "init-author-url",
    "init-license",
    "init-module",
    "init-version",
    "json",
    "key",
    "legacy-bundling",
    "link",
    "local-address",
    "location",
    "loglevel",
    "logs-dir",
    "logs-max",
    "long",
    "maxsockets",
    "message",
    "metrics-registry",
    "node-options",
    "noproxy",
    "offline",
    "omit",
    "omit-lockfile-registry-resolved",
    "only",
    "optional",
    "otp",
    "package-lock",
    "package-lock-only",
    "parseable",
    "prefix",
    "preid",
    "production",
    "progress",
    "provenance",
    "proxy",
    "read-only",
    "rebuild-bundle",
    "registry",
    "replace-registry-host",
    "save",
    "save-bundle",
    "save-dev",
    "save-exact",
    "save-optional",
    "save-peer",
    "save-prefix",
    "save-prod",
    "scope",
    "script-shell",
    "searchexclude",
    "searchlimit",
    "searchopts",
    "shell",
    "shrinkwrap",
    "sign-git-commit",
    "sign-git-tag",
    "strict-peer-deps",
    "strict-ssl",
    "tag",
    "tag-version-prefix",
    "timing",
    "tmp",
    "umask",
    "unicode",
    "update-notifier",
    "usage",
    "user-agent",
    "userconfig",
    "version",
    "versions",
    "viewer",
    "which",
    "workspace",
    "workspaces",
    "yes",
];

/// npmrc 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知 `key=value` 代入。
    pub options: usize,
    /// スコープ設定行(`//host/:key=`・`@scope:key=`)。
    pub scoped: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 未知 `key=` 代入/分類不能行。
    pub misc: usize,
}

/// 行が `key=value` か(キー部分を返す)。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find('=')?;
    let k = t[..p].trim();
    if k.is_empty() {
        None
    } else {
        Some(k)
    }
}

/// スコープ設定キーか(`//…/:` または `@scope:`)。
fn is_scoped_key(k: &str) -> bool {
    (k.starts_with("//") && k.contains("/:")) || (k.starts_with('@') && k.contains(':'))
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が .npmrc かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with('#')
                && !t.starts_with(';')
                && kv_key(t).is_some_and(|k| {
                    is_scoped_key(k)
                        || KEYS.contains(&k)
                        || k.strip_prefix("//").is_some_and(|s| s.contains(':'))
                })
        })
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        scoped: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if let Some(k) = kv_key(t) {
            if is_scoped_key(k) {
                c.scoped += 1;
            } else if KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.options + c.scoped >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# npmrc\nregistry=https://registry.npmjs.org/\n@myorg:registry=https://npm.example.com/\n//npm.example.com/:_authToken=abc\n//npm.example.com/:always-auth=true\nsave-exact=true\nengine-strict=true\npackage-lock=false\nstrict-ssl=true\nfund=false\naudit-level=moderate\n";

    #[test]
    fn npmrc() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 7);
        assert_eq!(c.scoped, 3);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_npmrc() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"[section]\nkey = value\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
