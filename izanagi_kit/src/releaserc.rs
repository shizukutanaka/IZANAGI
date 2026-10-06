//! semantic-release(`.releaserc`/`.releaserc.json`/`.releaserc.yaml`/
//! `release.config.js`)の検出と構造カウント。
//!
//! `branches`/`plugins` キーに加え、`tagFormat`/`preset`/`extends`/
//! `repositoryUrl`/`dryRun`/`ci`/`verifyConditions`/`verifyRelease`/
//! `analyzeCommits`/`generateNotes`/`prepare`/`publish`/`success`/`fail`
//! 等の semantic-release 固有キーか `@semantic-release/` プラグイン参照を
//! 要求し、一般 JSON/YAML との誤検出を避ける。
//!
//! ```
//! let b = b"{\n  \"branches\": [\"main\", \"next\"],\n  \"plugins\": [\"@semantic-release/commit-analyzer\", \"@semantic-release/release-notes-generator\"]\n}\n";
//! assert!(izanagi_kit::releaserc::detect(b));
//! let c = izanagi_kit::releaserc::parse(b).unwrap();
//! assert!(c.plugin_refs >= 2);
//! ```

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `"key"` が JSON オブジェクトのキー位置に現れるか(値側の言及は無視)。
fn jkey(t: &str, key: &str) -> bool {
    let pat = format!("\"{key}\"");
    t.find(&pat).is_some_and(|_| {
        t.match_indices(&pat).any(|(i, _)| {
            let rest = t[i + pat.len()..].trim_start();
            rest.starts_with(':')
        })
    })
}

/// semantic-release 固有の設定キー。
const SR_KEYS: &[&str] = &[
    "analyzeCommits",
    "branches",
    "ci",
    "debug",
    "dryRun",
    "extends",
    "fail",
    "generateNotes",
    "plugins",
    "prepare",
    "preset",
    "publish",
    "repositoryUrl",
    "success",
    "tagFormat",
    "verifyConditions",
    "verifyRelease",
];

/// `.releaserc` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// semantic-release 固有キーの行数。
    pub config_keys: usize,
    /// `@semantic-release/…` プラグイン参照の出現数。
    pub plugin_refs: usize,
    /// `#`/`//` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// キーが行アンカー形(`key:`/`"key":`)で存在するか。
fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim(), key)) || jkey(t, key)
}

/// `b` が semantic-release 設定に見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    if !(has_key(t, "branches") && has_key(t, "plugins")) {
        return false;
    }
    t.contains("@semantic-release/")
        || has_key(t, "tagFormat")
        || has_key(t, "preset")
        || has_key(t, "repositoryUrl")
        || has_key(t, "verifyConditions")
        || has_key(t, "extends")
}

/// `b` を semantic-release 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        config_keys: 0,
        plugin_refs: t.matches("@semantic-release/").count(),
        comments: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if SR_KEYS.iter().any(|k| is_key(tr, k)) {
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

    const SAMPLE: &[u8] = b"{\n  \"branches\": [\"main\", \"next\"],\n  \"plugins\": [\"@semantic-release/commit-analyzer\", \"@semantic-release/release-notes-generator\", \"@semantic-release/npm\"]\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"branches:\n  - main\nplugins:\n  - '@semantic-release/commit-analyzer'\ntagFormat: v${version}\n"
        ));
        // branches+plugins だけでは検出しない(固有キーが必要)。
        assert!(!detect(
            b"{\n  \"branches\": [\"main\"],\n  \"plugins\": [\"x\"]\n}\n"
        ));
        assert!(!detect(b"{\n  \"name\": \"pkg\"\n}\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.plugin_refs, 3);
        assert!(c.config_keys >= 2);
        assert!(parse(b"{\n  \"name\": \"pkg\"\n}\n").is_none());
    }
}
