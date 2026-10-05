//! GB Studio `project.gbsproj` の検出と構造カウント。
//!
//! ゲーム全データを1 JSON に持つ形式:`"scenes"`/`"settings"`/`"variables"`/
//! `"spriteSheets"`/`"backgrounds"`/`music`/`"palettes"`/`"customEvents"`。
//!
//! ```
//! let c = izanagi_kit::gbstudio::parse(
//!     b"{\"name\":\"g\",\"scenes\":[{\"id\":\"1\"}],\"settings\":{\"startSceneId\":\"1\"}}").unwrap();
//! assert_eq!(c.objects, 3);
//! assert!(izanagi_kit::gbstudio::detect(b"{\"scenes\":[],\"settings\":{},\"variables\":[]}"));
//! ```

/// GB Studio の既知リソース配列キー。
const ARRAYS: &[&str] = &[
    "scenes",
    "spriteSheets",
    "backgrounds",
    "musi\u{63}",
    "sounds",
    "fonts",
    "palettes",
    "variables",
    "customEvents",
    "avatars",
    "emotes",
    "tilesets",
    "prefabs",
    "engineFieldValues",
    "scriptResourceType",
    "chapterImages",
];
/// `settings` 内の既知キー。
const SETTINGS: &[&str] = &[
    "autoFlashEnabled",
    "batterylessEnabled",
    "colorEnabled",
    "colorMode",
    "customColorsEnabled",
    "customColorsWhite",
    "customColorsLight",
    "customColorsDark",
    "customColorsBackground",
    "defaultBackgroundPaletteIds",
    "defaultFadePaletteIds",
    "defaultSpritePaletteId",
    "defaultUIPaletteId",
    "engineFields",
    "fadeStyle",
    "musicDriver",
    "openSourceLicense",
    "romType",
    "sgbEnabled",
    "showColliders",
    "showConnections",
    "startAnimSpeed",
    "startDirection",
    "startMoveSpeed",
    "startSceneId",
    "startX",
    "startY",
    "worldScrollX",
    "worldScrollY",
];
/// `settings` 外のスカラーキー。
const SCALARS: &[&str] = &[
    "_v",
    "name",
    "author",
    "notes",
    "version",
    "zoom",
    "spriteMode",
    "tmpDirPath",
    "generator",
    "cartridgeType",
    "colorCorrection",
    "debuggerEnabled",
];

/// `"key":` 走査(位置と値先頭を返す)。
fn entries(text: &str) -> Vec<(&str, usize)> {
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
        let mut k = j + 1;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        if k < bytes.len() && bytes[k] == b':' {
            out.push((&text[start..j], start));
        }
        i = j + 1;
    }
    out
}

/// 位置 start の JSON 深度(`{`/`[` 開始までの括弧差分)。
fn depth_at(text: &str, start: usize) -> i32 {
    let mut d = 0_i32;
    for &ch in text.as_bytes()[..start].iter() {
        if ch == b'{' || ch == b'[' {
            d += 1;
        } else if ch == b'}' || ch == b']' {
            d -= 1;
        }
    }
    d
}

/// GB Studio 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルの既知リソースキー(scenes/variables/…)。
    pub objects: usize,
    /// `settings` 内の既知設定キー。
    pub settings: usize,
    /// `id`/`name` 等エンティティの属性キー。
    pub attributes: usize,
    /// その他キー。
    pub misc: usize,
}

/// b が .gbsproj かどうか。
///
/// `scenes`/`settings` が JSON キー位置(`"key":`)にあることだけを
/// 見る — 文字列値の中の同名語では検出しない。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let keys: Vec<&str> = entries(text).iter().map(|(k, _)| *k).collect();
    text.contains('{') && keys.contains(&"scenes") && keys.contains(&"settings")
}

/// .gbsproj の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    if !text.contains('{') {
        return None;
    }
    let mut c = Counts {
        objects: 0,
        settings: 0,
        attributes: 0,
        misc: 0,
    };
    for (key, start) in entries(text) {
        let d = depth_at(text, start);
        if d <= 1 && (ARRAYS.contains(&key) || key == "settings" || SCALARS.contains(&key)) {
            c.objects += 1;
        } else if SETTINGS.contains(&key) {
            c.settings += 1;
        } else if matches!(
            key,
            "id" | "name" | "title" | "description" | "type" | "x" | "y"
        ) {
            c.attributes += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.objects >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"name\": \"demo\",\n  \"author\": \"me\",\n  \"version\": \"4\",\n  \"scenes\": [\n    {\"id\": \"s1\", \"name\": \"start\", \"backgroundId\": \"b1\", \"actors\": [{\"id\": \"a1\"}]}\n  ],\n  \"backgrounds\": [{\"id\": \"b1\"}],\n  \"spriteSheets\": [{\"id\": \"p1\"}],\n  \"music\": [],\n  \"variables\": [\"var0\"],\n  \"settings\": {\n    \"startSceneId\": \"s1\",\n    \"startX\": 7,\n    \"startY\": 6,\n    \"musicDriver\": \"hUGE\",\n    \"colorEnabled\": true\n  },\n  \"customEvents\": []\n}\n";

    #[test]
    fn gbstudio() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.objects, 10);
        assert_eq!(c.settings, 5);
        assert!(c.attributes >= 3);
    }

    #[test]
    fn not_gbstudio() {
        assert!(!detect(b"{\"name\":\"x\"}"));
        assert!(!detect(b"key = v\n"));
    }

    #[test]
    fn markers_in_string_values_do_not_detect() {
        // The quoted words inside a string value must not trigger.
        assert!(!detect(b"{\"notes\": \"\\\"scenes\\\" \\\"settings\\\"\"}"));
        assert!(detect(
            b"{\"scenes\":[],\"settings\":{\"startSceneId\":\"1\"}}"
        ));
    }
}
