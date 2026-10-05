//! Unity `ProjectSettings.asset`(および兄弟アセット)の検出と構造カウント。
//!
//! Unity YAML:`%TAG !u!` ディレクティブ・`--- !u!<class> &id` ドキュメント・
//! `PlayerSettings:` 等の既知ルート + `m_*`/キャメルキーを分類する。
//!
//! ```
//! let c = izanagi_kit::unitysettings::parse(
//!     b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  m_Name: game\n  productName: Game\n").unwrap();
//! assert_eq!(c.documents, 3);
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::unitysettings::detect(b"%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n"));
//! ```

/// 既知 Unity ルートキー(*Settings/*Manager 等)。
const ROOTS: &[&str] = &[
    "Activity",
    "Android",
    "AudioManager",
    "BuildPlayer",
    "ClusterInputManager",
    "CloudSettings",
    "DynamicsManager",
    "EditorSettings",
    "EditorUserBuildSettings",
    "GarbageCollector",
    "GraphicsSettings",
    "InputManager",
    "LightmapSettings",
    "MemorySettings",
    "NavMeshAreas",
    "NavMeshSettings",
    "NetworkManager",
    "PackageManagerSettings",
    "Physics2DSettings",
    "PhysicsManager",
    "PlayerSettings",
    "PresetManager",
    "QualitySettings",
    "RenderSettings",
    "ShaderGraph",
    "StreamingManager",
    "TagManager",
    "TerrainSettings",
    "TimeManager",
    "UiElements",
    "UnityAdsSettings",
    "UnityConnectSettings",
    "VFXManager",
    "VideoManager",
    "VisualScriptingSettings",
    "XRManagerSettings",
    "XRSettings",
];
/// PlayerSettings 内の代表キー(先頭数文字で広く拾うため境界マッチに利用)。
const PLAYER_KEYS: &[&str] = &[
    "accelerometerFrequency",
    "activeInputHandler",
    "androidBanner",
    "androidKeystoreName",
    "apiCompatibilityLevel",
    "applicationIdentifier",
    "assetBundleVersion",
    "bundleVersion",
    "buildNumber",
    "companyName",
    "cpuArchitecture",
    "defaultCursor",
    "defaultScreenHeight",
    "defaultScreenWidth",
    "displayResolutionDialog",
    "firstStreamedLevelWithResources",
    "fullscreenMode",
    "gpuSkinning",
    "internal",
    "ios",
    "ipadHighResPortrait",
    "keystorePass",
    "licenseType",
    "locationUsageDescription",
    "m_ActiveColorSpace",
    "m_AllowedOrientationLandscapeLeft",
    "m_AllowedOrientationLandscapeRight",
    "m_AllowedOrientationPortrait",
    "m_AllowedOrientationPortraitUpsideDown",
    "m_BuildTargetIcons",
    "m_BuildTargetPlatformIcons",
    "m_Name",
    "m_StackTraceTypes",
    "microphoneUsageDescription",
    "mobileRenderPath",
    "muteOtherAudioSources",
    "preloadedAssets",
    "productGUID",
    "productName",
    "resizableWindow",
    "runInBackground",
    "scriptingBackend",
    "splashes",
    "supportedAspectRatios",
    "targetDevice",
    "usePlayerLog",
    "visibleInBackground",
    "vulkanEnableSetSRGBWrite",
    "webGLTemplate",
];

/// Unity settings 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `%YAML`/`%TAG`/`---` ドキュメント行。
    pub documents: usize,
    /// 既知ルートキー行(PlayerSettings: 等)。
    pub sections: usize,
    /// `key: value` の既知/一般オプション行。
    pub options: usize,
    /// `- ` リスト項目。
    pub items: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が Unity ProjectSettings アセットかどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    // `!u!`/`unity3d.com` は YAML タグ宣言やドキュメント行に限り
    // シグネチャとする — コメント/値の中の文字列では検出しない。
    let has_tag = text.lines().any(|l| {
        let t = l.trim_start();
        (t.starts_with("---") || t.starts_with("%TAG") || t.starts_with("%YAML"))
            && (t.contains("!u!") || t.contains("unity3d.com"))
    });
    let mut roots = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.find(':').is_some_and(|p| ROOTS.contains(&&t[..p])) {
            roots += 1;
        }
    }
    has_tag && roots >= 1
}

/// Unity settings アセットの構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        documents: 0,
        sections: 0,
        options: 0,
        items: 0,
        comments: 0,
        misc: 0,
    };
    let mut known_root = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('%') || t.starts_with("---") {
            c.documents += 1;
            continue;
        }
        if t.starts_with('-') {
            c.items += 1;
            continue;
        }
        let Some(colon) = t.find(':') else {
            c.misc += 1;
            continue;
        };
        let key = &t[..colon];
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            if ROOTS.contains(&key) {
                c.sections += 1;
                known_root = true;
            } else {
                c.misc += 1;
                known_root = false;
            }
        } else if known_root || PLAYER_KEYS.contains(&key) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.documents + c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 26\n  productGUID: abcdef0123456789\n  companyName: MyCompany\n  productName: MyGame\n  defaultScreenWidth: 1920\n  defaultScreenHeight: 1080\n  m_ActiveColorSpace: 1\n  m_StackTraceTypes: 01000000\n  apiCompatibilityLevel: 3\n  bundleVersion: 1.0.0\n--- !u!78 &2\nTagManager:\n  serializedVersion: 2\n  tags:\n  - Enemy\n  - Ally\n  layers:\n  - Default\n";

    #[test]
    fn unitysettings() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.documents, 4);
        assert_eq!(c.sections, 2);
        assert_eq!(c.options, 14);
        assert_eq!(c.items, 3);
    }

    #[test]
    fn not_unity() {
        assert!(!detect(b"key: value\nother: 1\n"));
        assert!(!detect(b"hello\n"));
    }

    #[test]
    fn tag_marker_must_be_a_directive() {
        // `!u!` in a comment or plain line is not a Unity signature.
        assert!(!detect(b"# !u!129 &1\nPlayerSettings:\n  productName: x\n"));
        assert!(!detect(
            b"notes: contains !u! text\nPlayerSettings:\n  x: 1\n"
        ));
        // The real `%TAG`/`--- !u!` directives still detect.
        assert!(detect(
            b"%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n"
        ));
    }
}
