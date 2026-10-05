//! Cocos2d-x/Cocos Creator `project.json`/`.cocos-project.json` の検出と構造カウント。
//!
//! `engine_version`/`project_type`/`isNative` 等のシグネチャキーを
//! JSON キー走査で分類する。
//!
//! ```
//! let c = izanagi_kit::cocosproj::parse(
//!     b"{\"project_type\":\"lua\",\"engine_version\":\"cocos2d-x-3.17\",\"isNative\":true}").unwrap();
//! assert_eq!(c.signatures, 3);
//! assert!(izanagi_kit::cocosproj::detect(b"{\"project_type\":\"js\"}"));
//! ```

/// Cocos プロジェクトの既知キー。
const KEYS: &[&str] = &[
    "android_aapt_skip_compile",
    "android_apilevel",
    "android_instant",
    "app_name",
    "build_cfg",
    "build_script",
    "buildNative",
    "custom_step_script",
    "design_height",
    "design_width",
    "engine_dir",
    "engine_type",
    "engine_version",
    "isNative",
    "is_wp8",
    "jsBundled",
    "language",
    "lua_encrypt",
    "main_module",
    "package_name",
    "platform",
    "portrait",
    "preCompiledEngine",
    "project_type",
    "quick_version",
    "resolution_policy",
    "simulator_screen_size",
    "start_scene",
    "step",
    "target",
    "templates",
    "useSource",
    "version",
];

/// `"key"` 走査(`:` 後続のみ)。
fn json_keys(text: &str) -> Vec<&str> {
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
            out.push(&text[start..j]);
        }
        i = j + 1;
    }
    out
}

/// Cocos プロジェクト構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// エンジン/型シグネチャキー(project_type/engine_*/isNative)。
    pub signatures: usize,
    /// その他既知キー。
    pub options: usize,
    /// 未知キー。
    pub misc: usize,
}

/// b が Cocos project.json かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    (text.contains("project_type") || text.contains("engine_version")) && text.contains('{')
}

/// project.json の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    if !text.contains('{') {
        return None;
    }
    let mut c = Counts {
        signatures: 0,
        options: 0,
        misc: 0,
    };
    for key in json_keys(text) {
        if key.starts_with("engine_") || key == "project_type" || key == "isNative" {
            c.signatures += 1;
        } else if KEYS.contains(&key) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.signatures + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"project_type\": \"lua\",\n  \"engine_version\": \"cocos2d-x-3.17.2\",\n  \"engine_type\": \"prebuilt\",\n  \"isNative\": true,\n  \"main_module\": \"src/main.lua\",\n  \"package_name\": \"com.example.game\",\n  \"portrait\": false,\n  \"custom\": \"x\"\n}\n";

    #[test]
    fn cocosproj() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.signatures, 4);
        assert_eq!(c.options, 3);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_cocos() {
        assert!(!detect(b"{\"name\":\"x\"}"));
        assert!(!detect(b"key = v\n"));
    }
}
