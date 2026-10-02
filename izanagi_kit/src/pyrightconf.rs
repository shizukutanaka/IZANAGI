//! Pyright/Pylance `pyrightconfig.json` の検出と構造カウント。
//!
//! `typeCheckingMode`/`pythonVersion`/`report*`/`include`/`exclude`/`extraPaths`/
//! `executionEnvironments` 等の既知キーを JSON 行走査で分類する。
//!
//! ```
//! let c = izanagi_kit::pyrightconf::parse(
//!     b"{\n \"typeCheckingMode\": \"strict\",\n \"pythonVersion\": \"3.12\"\n}\n").unwrap();
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::pyrightconf::detect(
//!     b"{\"typeCheckingMode\": \"basic\", \"pythonVersion\": \"3.12\"}"));
//! ```

/// 診断ルール(`report*`)。
const REPORT_PREFIX: &str = "report";
/// 既知トップキー。
const KEYS: &[&str] = &[
    "analyzeUnannotatedFunctions",
    "autoFormatPythonImports",
    "defineConstant",
    "deprecateTypingAliases",
    "diagnosticLevel",
    "diagnosticMode",
    "disableBytesTypePromotions",
    "enableExperimentalFeatures",
    "exclude",
    "executionEnvironments",
    "extraPaths",
    "extends",
    "failOnWarnings",
    "importFormat",
    "include",
    "inlayHints",
    "jsonOutput",
    "logPath",
    "nodeModules",
    "openFilesOnly",
    "outputjson",
    "pythonPlatform",
    "pythonVersion",
    "regenerationCount",
    "reportAmbiguousUri",
    "skipUnannotatedFunction",
    "stats",
    "strict",
    "stubPath",
    "typeCheckingMode",
    "typeshedPath",
    "useLibraryCodeForTypes",
    "venv",
    "venvPath",
    "verboseOutput",
    "watchman",
    "workspace",
];

/// Pyright 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップキー(診断ルール含む)。
    pub options: usize,
    /// `report*` 診断ルールキー数。
    pub reports: usize,
    /// `executionEnvironments` ブロック内オブジェクト推定行。
    pub environments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// `"key"` の抽出(JSON 行頭)。
fn json_key(t: &str) -> Option<&str> {
    let rest = t.strip_prefix('"')?;
    let end = rest.find('"')?;
    let key = &rest[..end];
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

/// 行内の全 `"key":` 出現を列挙(1行複数キーの JSON に対応)。
fn keys_in_line<'a>(t: &'a str, out: &mut Vec<&'a str>) {
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j < bytes.len() && j > start {
                let mut k = j + 1;
                while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b':' {
                    out.push(&t[start..j]);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

/// b が pyrightconfig.json かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    let mut reports = 0;
    let mut keys = Vec::new();
    for line in text.lines() {
        keys.clear();
        keys_in_line(line.trim(), &mut keys);
        for k in &keys {
            if k.starts_with(REPORT_PREFIX) {
                reports += 1;
            } else if KEYS.contains(k) {
                hits += 1;
            }
        }
    }
    hits >= 2 || (reports >= 1 && hits >= 1) || reports >= 3
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        reports: 0,
        environments: 0,
        misc: 0,
    };
    let mut in_env = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t == "{" || t == "}" || t == "}," || t == "]" || t == "]," {
            if in_env && (t == "}," || t == "]" || t == "],") {
                in_env = false;
            }
            continue;
        }
        let Some(key) = json_key(t) else {
            if in_env {
                c.environments += 1;
            }
            continue;
        };
        if key == "executionEnvironments" {
            in_env = true;
            c.options += 1;
        } else if key.starts_with(REPORT_PREFIX) {
            c.reports += 1;
            c.options += 1;
        } else if KEYS.contains(&key) {
            c.options += 1;
        } else if in_env {
            c.environments += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"typeCheckingMode\": \"strict\",\n  \"pythonVersion\": \"3.12\",\n  \"pythonPlatform\": \"Linux\",\n  \"include\": [\"src\"],\n  \"exclude\": [\"tests\"],\n  \"extraPaths\": [\"third_party\"],\n  \"useLibraryCodeForTypes\": true,\n  \"reportMissingImports\": \"error\",\n  \"reportUnknownVariableType\": \"warning\",\n  \"reportGeneralTypeIssues\": \"error\",\n  \"executionEnvironments\": [\n    {\n      \"root\": \"src\",\n      \"pythonVersion\": \"3.12\"\n    }\n  ],\n  \"verboseOutput\": false\n}\n";

    #[test]
    fn pyrightconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 13);
        assert_eq!(c.reports, 3);
        assert_eq!(c.environments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_pyright() {
        assert!(!detect(b"{\"name\": \"x\", \"version\": \"1\"}"));
        assert!(!detect(b"hello\n"));
    }
}
