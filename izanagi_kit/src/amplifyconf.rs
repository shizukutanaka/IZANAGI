//! AWS Amplify Hosting `amplify.yml` ビルド仕様の認識と計数。
//!
//! Amplify のビルド仕様は YAML で、`version:` に続き `frontend:` /
//! `backend:` / `test:` / `customHeaders:` / `cache:` / `env:` セクションを
//! 持つ。各セクション配下の `phases:` は `preBuild` / `build` / `postBuild`
//! (`test:` では `preTest`/`postTest`) に分かれ、`commands:` リストに
//! シェルコマンドを並べる。`artifacts:` には `baseDirectory:` と `files:` を
//! 指定する。
//!
//! ```
//! let b = b"version: 1\nfrontend:\n  phases:\n    preBuild:\n      commands:\n        - npm ci\n    build:\n      commands:\n        - npm run build\n  artifacts:\n    baseDirectory: build\n    files:\n      - '**/*'\ncustomHeaders:\n  - pattern: '**'\n";
//! assert!(izanagi_kit::amplifyconf::detect(b));
//! let c = izanagi_kit::amplifyconf::parse(b).unwrap();
//! assert_eq!(c.keys, 11); // version/frontend/phases/preBuild/commands×2/build/artifacts/baseDirectory/files/customHeaders
//! assert_eq!(c.blocks, 2); // frontend, customHeaders
//! assert_eq!(c.phases, 2); // preBuild, build
//! assert_eq!(c.commands, 2);
//! assert_eq!(c.list_items, 4);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key:` を持つ行の個数(`key: value` を含む)。
    pub keys: usize,
    /// トップレベル `frontend:`/`backend:`/`test:`/`customHeaders:`/`cache:`/`env:` ブロックの個数。
    pub blocks: usize,
    /// `preBuild:`/`build:`/`postBuild:`/`preTest:`/`postTest:` フェーズの個数。
    pub phases: usize,
    /// `commands:` 配下の `-` リスト項目の個数。
    pub commands: usize,
    /// `-` 始まりのリスト項目の個数。
    pub list_items: usize,
    /// `artifacts:`/`cache:`/`headers:`/`files:`/`paths:`/`variables:` 系
    /// ネストサブセクションの個数。
    pub subsections: usize,
    /// `#` で始まる行の個数。
    pub comments: usize,
}

const TOP_BLOCKS: &[&str] = &[
    "frontend",
    "backend",
    "test",
    "customHeaders",
    "cache",
    "env",
    "monorepo",
    "applications",
];

const SUB_SECTIONS: &[&str] = &[
    "artifacts",
    "headers",
    "files",
    "paths",
    "variables",
    "phases",
];

const PHASES: &[&str] = &["preBuild", "build", "postBuild", "preTest", "postTest"];

/// 行の `key:` からキー名を取る。
fn line_key(s: &str) -> Option<&str> {
    let colon = s.find(':')?;
    let k = s[..colon]
        .trim_end()
        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
        .next()
        .unwrap_or("");
    if k.is_empty() {
        None
    } else {
        Some(k)
    }
}

fn is_key(s: &str, key: &str) -> bool {
    // `key :` (コロン前の空白)も YAML では合法。
    s.strip_prefix(key)
        .is_some_and(|r| r.trim_start().starts_with(':'))
}

/// `amplify.yml` らしさを返す。`version:` と `frontend:`/`backend:`/`phases:`
/// 等ビルド構造キーの組合せで判定。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    let mut version = false;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if is_key(s, "version") {
            version = true;
        }
        for key in [
            "frontend:",
            "backend:",
            "phases:",
            "commands:",
            "artifacts:",
            "customHeaders:",
        ] {
            if line_key(s) == Some(key.trim_end_matches(':')) {
                hits += 1;
            }
        }
    }
    version && hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        keys: 0,
        blocks: 0,
        phases: 0,
        commands: 0,
        list_items: 0,
        subsections: 0,
        comments: 0,
    };
    let mut in_commands = false;
    for l in t.lines() {
        if l.trim().is_empty() {
            continue;
        }
        let indent = l.len() - l.trim_start().len();
        let s = l.trim();
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('-') {
            c.list_items += 1;
            if in_commands {
                c.commands += 1;
            }
            continue;
        }
        let Some(key) = line_key(s) else {
            continue;
        };
        c.keys += 1;
        if indent == 0 && TOP_BLOCKS.contains(&key) {
            c.blocks += 1;
        } else if SUB_SECTIONS.contains(&key) {
            c.subsections += 1;
        }
        if PHASES.contains(&key) {
            c.phases += 1;
        }
        in_commands = key == "commands";
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_space_before_colon() {
        // YAML では `key :` も合法。
        let b = b"version : 1\nfrontend:\n  phases:\n    build:\n      commands:\n        - x\n  artifacts:\n    baseDirectory: build\n";
        assert!(detect(b));
    }

    #[test]
    fn detects_amplify() {
        let b = b"version: 1\nfrontend:\n  phases:\n    build:\n      commands:\n        - npm run build\n  artifacts:\n    baseDirectory: build\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"name: x\nkind: y\n"));
        assert!(!detect(b"version: 1\nfoo: bar\n"));
        assert!(!detect(&[0xff, 0x00]));
    }

    #[test]
    fn counts() {
        let b = b"version: 1\nfrontend:\n  phases:\n    preBuild:\n      commands:\n        - npm ci\n    build:\n      commands:\n        - npm run build\n  artifacts:\n    baseDirectory: build\n    files:\n      - '**/*'\ncustomHeaders:\n  - pattern: '**'\n";
        let c = parse(b).unwrap();
        assert_eq!(c.keys, 11);
        assert_eq!(c.subsections, 3); // phases + artifacts + files
        assert_eq!(c.blocks, 2);
        assert_eq!(c.phases, 2);
        assert_eq!(c.commands, 2);
        assert_eq!(c.list_items, 4);
    }

    #[test]
    fn copy_eq() {
        let c =
            parse(b"version: 1\nfrontend:\n  phases:\n    build:\n      commands:\n        - x\n")
                .unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}
