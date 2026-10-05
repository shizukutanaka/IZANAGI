//! RPGツクール MV/MZ `js/plugins.js`(プラグイン JSON)の検出と構造カウント。
//!
//! `[{"name":..,"status":true,"description":..,"parameters":{..}}]` 配列を
//! キー走査で分類する。
//!
//! ```
//! let c = izanagi_kit::rpgmakerconf::parse(
//!     b"[{\"name\":\"TestPlugin\",\"status\":true,\"description\":\"d\",\"parameters\":{\"x\":\"1\"}}]").unwrap();
//! assert_eq!(c.plugins, 1);
//! assert_eq!(c.enabled, 1);
//! assert!(izanagi_kit::rpgmakerconf::detect(b"[{\"name\":\"X\",\"status\":false,\"parameters\":{}}]"));
//! ```

/// `"key":` トークンの走査(値 `"true"`/`"false"` 検出のため位置も返す)。
fn entries(text: &str) -> Vec<(&str, usize, usize)> {
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
            // 値先頭(非空白)位置を拾う。
            let mut v = k + 1;
            while v < bytes.len() && bytes[v].is_ascii_whitespace() {
                v += 1;
            }
            out.push((&text[start..j], v, j + 1));
        }
        i = j + 1;
    }
    out
}

/// plugins.js 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"name"` プラグインオブジェクト数。
    pub plugins: usize,
    /// `"status": true` のプラグイン数。
    pub enabled: usize,
    /// `"parameters"` オブジェクト数。
    pub parameter_blocks: usize,
    /// その他キー。
    pub misc: usize,
}

/// b が plugins.js かどうか。
///
/// `name`/`status`/`parameters` が JSON キー位置(`"key":`)にあること
/// だけを見る — 文字列値の中の同名語では検出しない。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let keys: Vec<&str> = entries(text).iter().map(|(k, _, _)| *k).collect();
    keys.contains(&"name") && keys.contains(&"status") && keys.contains(&"parameters")
}

/// plugins.js の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        plugins: 0,
        enabled: 0,
        parameter_blocks: 0,
        misc: 0,
    };
    for (key, val, _) in entries(text) {
        match key {
            "name" => c.plugins += 1,
            "status" => {
                if text[val..].starts_with("true") {
                    c.enabled += 1;
                }
            }
            "parameters" => c.parameter_blocks += 1,
            _ => c.misc += 1,
        }
    }
    (c.plugins >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[\n{\"name\":\"Core\",\"status\":true,\"description\":\"core\",\"parameters\":{\"debug\":\"1\"}},\n{\"name\":\"Battle\",\"status\":true,\"description\":\"bt\",\"parameters\":{\"mode\":\"2\"}},\n{\"name\":\"Menu\",\"status\":false,\"description\":\"mn\",\"parameters\":{}}\n]\n";

    #[test]
    fn rpgmakerconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.plugins, 3);
        assert_eq!(c.enabled, 2);
        assert_eq!(c.parameter_blocks, 3);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn not_rpgmaker() {
        assert!(!detect(b"{\"name\":\"x\"}"));
        assert!(!detect(b"key = v\n"));
    }

    #[test]
    fn markers_in_string_values_do_not_detect() {
        // The quoted words inside string values must not trigger.
        assert!(!detect(
            b"[{\"note\": \"\\\"name\\\" \\\"status\\\" \\\"parameters\\\"\"}]"
        ));
        assert!(detect(
            b"[{\"name\":\"X\",\"status\":false,\"parameters\":{}}]"
        ));
    }
}
