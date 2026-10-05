//! Unity `Packages/manifest.json` の検出と構造カウント。
//!
//! `"dependencies": {"com.unity.*": "version"}` + `scopedRegistries`/`testables`
//! 等の既知キーを JSON キー走査で分類する。
//!
//! ```
//! let c = izanagi_kit::unitymanifest::parse(
//!     b"{\"dependencies\":{\"com.unity.textmeshpro\":\"3.0.6\",\"com.unity.ugui\":\"1.0.0\"}}").unwrap();
//! assert_eq!(c.dependencies, 2);
//! assert!(izanagi_kit::unitymanifest::detect(b"{\"dependencies\":{\"com.unity.x\":\"1\"}}"));
//! ```

/// 既知トップレベルキー。
const KEYS: &[&str] = &[
    "dependencies",
    "enableLockFile",
    "overrideRegistry",
    "registry",
    "resolutionStrategy",
    "scopedRegistries",
    "testables",
    "useSatSolver",
];

/// `"key"` トークンを順に走査して(名前, `:` の有無)を返す。
fn json_keys(text: &str) -> Vec<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len() {
            if bytes[j] == 0x5C {
                j += 1;
            } else if bytes[j] == b'"' {
                break;
            }
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }
        // 直後の非空白が `:` ならキー、それ以外は値文字列。
        let mut k = j + 1;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        if k < bytes.len() && bytes[k] == b':' {
            out.push((start, &text[start..j]));
        }
        i = j + 1;
    }
    out
}

/// Unity manifest 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `com.unity.*` 依存名。
    pub dependencies: usize,
    /// scopedRegistries 内エントリ(ダブルクォートキー全般)。
    pub registry_entries: usize,
    /// 既知トップキー。
    pub options: usize,
    /// その他キー/項目。
    pub misc: usize,
}

/// b が Unity manifest.json かどうか。
///
/// `com.unity.*` が JSON キー位置(`"key":`)にあることだけを見る —
/// 値文字列(例: scopedRegistries の `scopes`/`testables` 値)の中に
/// 同じ接頭辞があっても検出しない。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.contains('{')
        && json_keys(text)
            .iter()
            .any(|(_, k)| k.starts_with("com.unity."))
}

/// manifest.json の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    if !text.contains('{') {
        return None;
    }
    let mut c = Counts {
        dependencies: 0,
        registry_entries: 0,
        options: 0,
        misc: 0,
    };
    let mut depth = Vec::new();
    let bytes = text.as_bytes();
    let keys = json_keys(text);
    for (start, key) in &keys {
        // 深度を文字位置から推定: 直前までの { と } の差。
        let mut d = 0_i32;
        for &ch in &bytes[..*start] {
            if ch == b'{' {
                d += 1;
            } else if ch == b'}' {
                d -= 1;
            }
        }
        depth.push(d);
        if key.starts_with("com.unity.") || key.starts_with("org.nuget.") {
            c.dependencies += 1;
        } else if KEYS.contains(key) {
            c.options += 1;
        } else if d >= 2 {
            c.registry_entries += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.dependencies + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"dependencies\": {\n    \"com.unity.textmeshpro\": \"3.0.6\",\n    \"com.unity.ugui\": \"1.0.0\",\n    \"com.unity.timeline\": \"1.7.4\"\n  },\n  \"testables\": [\"com.unity.test-framework\"],\n  \"scopedRegistries\": [\n    {\n      \"name\": \"npm\",\n      \"url\": \"https://npm.example.com\",\n      \"scopes\": [\"com.example\"]\n    }\n  ],\n  \"enableLockFile\": true\n}\n";

    #[test]
    fn unitymanifest() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.dependencies, 3);
        assert_eq!(c.options, 4);
        assert!(c.registry_entries >= 3);
    }

    #[test]
    fn not_unity() {
        assert!(!detect(b"{\"name\":\"x\"}"));
        assert!(!detect(b"key = value\n"));
    }

    #[test]
    fn com_unity_in_value_does_not_detect() {
        // `com.unity.*` appearing only inside string values must not trigger.
        assert!(!detect(
            b"{\"testables\": [\"com.unity.test-framework\"]}\n"
        ));
        assert!(!detect(b"{\"note\": \"com.unity.modules\"}\n"));
        assert!(detect(b"{\"dependencies\":{\"com.unity.x\":\"1\"}}"));
    }
}
