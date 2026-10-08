//! AppVeyor 設定(`appveyor.yml`)の検出と構造カウント。
//!
//! `version`/`image`/`build`/`build_script`/`test`/`test_script`/`install`/
//! `environment`/`matrix`/`configuration`/`platform`/`artifacts`/`deploy`/
//! `notifications`/`cache`/`branches`/`services` 等のトップキーと、
//! `for:` 条件ブロックと `matrix:` 内 `only:`/`except:` を識別する。
//!
//! ```
//! let c = izanagi_kit::appveyor::parse(
//!     b"version: 1.0.{build}\nimage: Visual Studio 2022\nbuild_script:\n  - cargo build\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert!(izanagi_kit::appveyor::detect(b"image: VS2022\nbuild_script:\n  - cargo build\ntest_script:\n  - cargo test\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "after_build",
    "after_deploy",
    "after_test",
    "artifacts",
    "assembly_info",
    "before_build",
    "before_deploy",
    "before_package",
    "before_test",
    "branches",
    "build",
    "build_cloud",
    "build_script",
    "cache",
    "clone_depth",
    "clone_folder",
    "configuration",
    "deploy",
    "deploy_script",
    "dotnet_csproj",
    "environment",
    "for",
    "hosts",
    "image",
    "init",
    "install",
    "matrix",
    "max_jobs",
    "notifications",
    "nuget",
    "off",
    "on_finish",
    "on_failure",
    "on_success",
    "package",
    "patch_fastly",
    "platform",
    "pull_requests",
    "services",
    "shallow_clone",
    "skip_branch_with_pr",
    "skip_commits",
    "skip_non_tags",
    "skip_tags",
    "source_directory",
    "stack",
    "test",
    "test_assemblies",
    "test_categories",
    "test_script",
    "verbosity",
    "version",
];

/// `for:` ブロック内の既知条件キー。
const FOR_KEYS: &[&str] = &[
    "branches",
    "commit_message",
    "debug",
    "pull_requests",
    "release",
    "repository",
    "skip_tags",
    "tags",
];

/// appveyor 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルキー行。
    pub sections: usize,
    /// `for:` 条件ブロック内のキー行。
    pub for_keys: usize,
    /// その他のネストした `key:` 行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行(リスト要素・継続スカラ・未知キー)。
    pub misc: usize,
}

/// `key:` 先頭のキー名。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が appveyor.yml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with('#')
                && (l.len() - l.trim_start().len()) == 0
                && yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k))
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
        sections: 0,
        for_keys: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_for = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_for = t == "for:";
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let item = t.strip_prefix("- ").map_or(t, |s| s);
        if let Some(k) = yaml_key(item) {
            if in_for && FOR_KEYS.contains(&k) {
                c.for_keys += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# appveyor\nversion: 1.0.{build}\nimage: Visual Studio 2022\nplatform:\n  - x64\nconfiguration:\n  - Release\ninstall:\n  - cinst rust-ms\nbuild_script:\n  - cargo build --release\ntest_script:\n  - cargo test\nfor:\n  branches:\n    only:\n      - main\n  notifications: []\n";

    #[test]
    fn appveyor() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 8);
        assert_eq!(c.for_keys, 1);
        assert!(c.options >= 1);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 6);
    }

    #[test]
    fn not_appveyor() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"image: none\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
