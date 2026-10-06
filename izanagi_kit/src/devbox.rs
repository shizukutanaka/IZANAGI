//! Devbox(`devbox.json`)の検出と構造カウント。
//!
//! `packages` キーに加え、`shell`/`init_hook`/`scripts`/`env`/`nixpkgs`/
//! `$schema`(jetify-com/devbox)等の devbox 固有構造を要求し、
//! `composer.lock` 等の `"packages"` を持つだけの JSON と区別する。
//!
//! ```
//! let b = b"{\n  \"$schema\": \"https://raw.githubusercontent.com/jetify-com/devbox/0.13.0/.schema/devbox.schema.json\",\n  \"packages\": [\"nodejs@20\", \"go@latest\"],\n  \"shell\": {\n    \"init_hook\": [\"echo hi\"],\n    \"scripts\": {\"test\": \"go test ./...\"}\n  }\n}\n";
//! assert!(izanagi_kit::devbox::detect(b));
//! let c = izanagi_kit::devbox::parse(b).unwrap();
//! assert!(c.package_entries >= 2);
//! ```

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

/// `key` が `key:`/`"key":` 行アンカー形で存在するか(YAML サポート)。
fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| {
        let k = l
            .trim()
            .trim_start_matches(['"', '\''])
            .trim_end_matches(',');
        k.strip_prefix(key)
            .is_some_and(|r| r.trim_start().starts_with(':'))
    }) || jkey(t, key)
}

/// devbox 固有のトップレベルキー。
const DEVBOX_KEYS: &[&str] = &[
    "env",
    "exclude_from_paths",
    "init_hook",
    "install_post_hook",
    "install_pre_hook",
    "nixpkgs",
    "packages",
    "path",
    "scripts",
    "shell",
];

/// `devbox.json` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `packages` のエントリ数(配列要素 `"name@ver"`/`"name"`、
    /// オブジェクト形 `"name": "ver"` のいずれも)。
    pub package_entries: usize,
    /// devbox 固有キーの行数。
    pub devbox_keys: usize,
    /// `#`/`//` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が `devbox.json` に見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    has_key(t, "packages")
        && (t.contains("devbox")
            || t.contains("jetify")
            || has_key(t, "init_hook")
            || has_key(t, "nixpkgs"))
}

/// `b` を `devbox.json` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        package_entries: 0,
        devbox_keys: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_packages = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with("//") {
            c.comments += 1;
            continue;
        }
        let key_line = DEVBOX_KEYS.iter().any(|k| {
            tr.trim_start_matches('"').starts_with(*k)
                && tr.trim_start_matches('"')[k.len()..]
                    .trim_start_matches('"')
                    .trim_start()
                    .starts_with(':')
        });
        if tr.trim_start_matches('"').starts_with("packages") {
            c.devbox_keys += 1;
            // 同一行インライン配列/オブジェクトのエントリを数える。
            if let Some(colon) = tr.find(':') {
                let v = &tr[colon + 1..];
                c.package_entries += v.matches('"').count() / 2;
            }
            in_packages = !tr.contains(']');
            continue;
        }
        if in_packages {
            // `]`/`}` で packages ブロック終了
            if tr.starts_with(']') || tr.starts_with('}') {
                in_packages = false;
                continue;
            }
            // オブジェクト形 `"name": "ver"` は 1 行 1 エントリ、
            // 配列形は行内の引用符文字列数で数える。
            if tr.starts_with('"') {
                if tr.contains(':') {
                    c.package_entries += 1;
                } else {
                    c.package_entries += tr.matches('"').count() / 2;
                }
                continue;
            }
        }
        if key_line {
            c.devbox_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"$schema\": \"https://raw.githubusercontent.com/jetify-com/devbox/0.13.0/.schema/devbox.schema.json\",\n  \"packages\": [\"nodejs@20\", \"go@latest\"],\n  \"shell\": {\n    \"init_hook\": [\"echo hi\"],\n    \"scripts\": {\"test\": \"go test ./...\"}\n  }\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"{\n  \"packages\": [\"go\"],\n  \"nixpkgs\": {\"commit\": \"abc\"}\n}\n"
        ));
        // packages だけの JSON は検出しない。
        assert!(!detect(
            b"{\n  \"packages\": [{\"name\": \"x\"}],\n  \"packages-dev\": []\n}\n"
        ));
        assert!(!detect(b"{\n  \"name\": \"pkg\"\n}\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.package_entries, 2);
        assert!(c.devbox_keys >= 3);
        assert!(parse(b"{\n  \"name\": \"pkg\"\n}\n").is_none());
    }
}
