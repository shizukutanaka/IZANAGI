//! Mbed OS `mbed_app.json` / `mbed_lib.json` アプリケーション設定の認識と
//! 計数。
//!
//! `mbed_app.json` は JSON で、`config`/`macros`/`target_overrides`/
//! `custom_targets`/`requires`/`features`/`target_lib`/`components` 等の
//! セクションを持つ。`config` には `<param>: { macro_name|value|help_text }`
//! のアプリ設定、`target_overrides` には `"*"`/`"TARGET_XXX"` キーの
//! ターゲット別上書きを書く。`mbed_lib.json` は同名構造のライブラリ側。
//!
//! ```
//! let b = b"{\n  \"config\": {\n    \"main-stack-size\": {\n      \"value\": \"0x4000\"\n    },\n    \"serial-baud\": {\n      \"macro_name\": \"MBED_CONF_BAUD\",\n      \"value\": 9600\n    }\n  },\n  \"macros\": [\"FOO\", \"BAR=1\"],\n  \"target_overrides\": {\n    \"*\": {\n      \"platform.stdio-baud-rate\": 9600,\n      \"platform.stdio-convert-newlines\": true\n    },\n    \"NUCLEO_F401RE\": {\n      \"network-default-interface-type\": \"ETHERNET\"\n    }\n  },\n  \"requires\": [\"mbed-os\"]\n}\n";
//! assert!(izanagi_kit::mbedapp::detect(b));
//! let c = izanagi_kit::mbedapp::parse(b).unwrap();
//! assert_eq!(c.keys, 14);
//! assert_eq!(c.sections, 4); // config macros target_overrides requires
//! assert_eq!(c.param_attrs, 3); // value/macro_name/value
//! assert_eq!(c.targets, 2); // "*", NUCLEO_F401RE
//! assert_eq!(c.config_params, 2); // main-stack-size, serial-baud
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"key":` の個数。
    pub keys: usize,
    /// `config`/`macros`/`target_overrides`/`custom_targets`/`requires`/
    /// `features`/`target_lib`/`components`/`artifact_name`/`name`/`tools`/
    /// `memory_*`/`bootloader*` 系トップキーの個数。
    pub sections: usize,
    /// `target_overrides` 直下のターゲットキー(`"*"`/`"TARGET_*"`/`"MCU_*"`)の個数。
    pub targets: usize,
    /// `config` 直下のパラメータ個数。
    pub config_params: usize,
    /// `"macro_name"`/`"value"`/`"help_text"`/`"defined_by"`/`"required"`/
    /// `"accepted"`/`"min_version"`/`"max_version"` パラメータ属性の個数。
    pub param_attrs: usize,
    /// `true`/`false` 値の個数。
    pub bools: usize,
    /// `//`/`/* */` コメント行の個数。
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "config",
    "macros",
    "target_overrides",
    "custom_targets",
    "requires",
    "features",
    "target_lib",
    "components",
    "artifact_name",
    "name",
    "tools",
    "has_config",
    "bootloader_supported",
    "application",
    "crypto_acceleration",
];

const PARAM_ATTRS: &[&str] = &[
    "macro_name",
    "value",
    "help_text",
    "defined_by",
    "required",
    "accepted",
    "min_version",
    "max_version",
    "display_name",
];

fn json_keys<'a>(s: &'a str, out: &mut std::vec::Vec<&'a str>) {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j < bytes.len() && s[j + 1..].trim_start().starts_with(':') {
                out.push(&s[i + 1..j]);
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

/// `mbed_app.json` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        for key in [
            "\"config\"",
            "\"macros\"",
            "\"target_overrides\"",
            "\"custom_targets\"",
            "\"requires\"",
            "\"macro_name\"",
            "\"defined_by\"",
            "\"help_text\"",
            "\"features\"",
            "\"target_lib\"",
        ] {
            if s.contains(key) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        keys: 0,
        sections: 0,
        targets: 0,
        config_params: 0,
        param_attrs: 0,
        bools: 0,
        comments: 0,
    };
    // -1=外, それ以外=セクション見出しの深度。
    let mut depth: i64 = 0;
    let mut in_overrides: i64 = -1;
    let mut in_config: i64 = -1;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with("//") || s.starts_with("/*") {
            c.comments += 1;
            continue;
        }
        let mut keys: std::vec::Vec<&str> = std::vec::Vec::new();
        json_keys(s, &mut keys);
        for (idx, &k) in keys.iter().enumerate() {
            c.keys += 1;
            if TOP_KEYS.contains(&k) {
                c.sections += 1;
            }
            if PARAM_ATTRS.contains(&k) {
                c.param_attrs += 1;
            }
            if k == "target_overrides" {
                in_overrides = depth + 1;
            }
            if k == "config" {
                in_config = depth + 1;
            }
            if in_overrides >= 0 && depth == in_overrides && idx == 0 {
                c.targets += 1;
            }
            if in_config >= 0 && depth == in_config {
                c.config_params += 1;
            }
            if idx == 0 && s.contains(':') {
                let after = s.rsplit(':').next().unwrap_or("");
                let v = after
                    .trim_start()
                    .trim_end_matches([',', '}', ']'])
                    .trim_end();
                if v == "true" || v == "false" {
                    c.bools += 1;
                }
            }
        }
        for ch in s.chars() {
            match ch {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
        }
        if in_overrides >= 0 && depth < in_overrides {
            in_overrides = -1;
        }
        if in_config >= 0 && depth < in_config {
            in_config = -1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(
            b"{\"config\": {}, \"macros\": [], \"target_overrides\": {}}"
        ));
        assert!(detect(
            b"{\"requires\": [\"mbed-os\"], \"defined_by\": \"x\"}"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"{\"name\": \"x\", \"version\": \"1\"}"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"{\n  \"config\": {\n    \"param-one\": {\n      \"value\": 1,\n      \"macro_name\": \"M\"\n    }\n  },\n  \"target_overrides\": {\n    \"*\": { \"a.b\": true }\n  }\n}\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.config_params, 1);
        assert_eq!(c.targets, 1);
        assert_eq!(c.param_attrs, 2); // value + macro_name
        assert_eq!(c.bools, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"{\"config\": {}}").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}
