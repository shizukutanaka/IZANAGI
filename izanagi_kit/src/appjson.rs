//! Census of an Expo `app.json` / `app.config.json` (`expo` block).
//!
//! `{ "expo": { … } }` with `name`, `slug`, `version`, `orientation`,
//! `icon`, `scheme`, `userInterfaceStyle`, `backgroundColor`, `splash`
//! (`image`/`resizeMode`/`backgroundColor`), `assetBundlePatterns`,
//! `updates` (`fallbackToCacheTimeout`/`url`/`enabled`/`checkAutomatically`/
//! `codeSigningCertificate`), `ios` (`bundleIdentifier`/`buildNumber`/
//! `supportsTablet`/`infoPlist`/`entitlements`/`associatedDomains`/
//! `config.googleMapsApiKey`), `android` (`package`/`versionCode`/
//! `adaptiveIcon`/`permissions`/`intentFilters`/`googleServicesFile`/
//! `config.googleMaps.apiKey`/`softwareKeyboardLayoutMode`), `web`
//! (`bundler`/`output`/`favicon`/`pwa`/`build.babel`), `plugins` array,
//! `extra`, `owner`, `runtimeVersion`, `jsEngine`, `locales`,
//! `notification` (`icon`/`color`/`androidMode`/`iosDisplayInForeground`),
//! `androidStatusBar`, `androidNavigationBar`, `sdkVersion`(legacy),
//! `experiments` (`tsconfigPaths`/`baseUrl`/`turboModules`/`newArchEnabled`),
//! `doctor`/`developmentClient`/`entryPoint`/`packagerOpts`.
//!
//! ```rust
//! let a = izanagi_kit::appjson::AppJson::parse(
//!     b"{ \"expo\": { \"name\": \"app\", \"slug\": \"app\", \"version\": \"1\" } }",
//! ).unwrap();
//! assert!(a.expo_keys >= 3);
//! ```
#![forbid(unsafe_code)]

/// Expo `app.json` census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppJson {
    /// Keys inside the `expo` object.
    pub expo_keys: usize,
    /// `ios`/`android`/`web`/`splash`/`updates`/`notification`/`experiments`
    ////`extra`/`locales`/`androidStatusBar`/`androidNavigationBar` section keys.
    pub sections: usize,
    /// `plugins:`/`assetBundlePatterns`/`permissions`/`locales` array items
    /// (quoted entries).
    pub list_items: usize,
    /// `"key":` keys total (expo scope + nested).
    pub keys: usize,
}

/// Section keys inside `expo`.
const SECTION_KEYS: &[&str] = &[
    "ios",
    "android",
    "web",
    "splash",
    "updates",
    "notification",
    "experiments",
    "extra",
    "locales",
    "androidStatusBar",
    "androidNavigationBar",
    "doctor",
    "developmentClient",
];

/// True if `b` looks like an Expo app.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"expo\"") && t.contains('{')
}

impl AppJson {
    /// Count the census fields; `None` unless [`detect`] holds.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            expo_keys: 0,
            sections: 0,
            list_items: 0,
            keys: 0,
        };
        let mut in_expo = false;
        let mut depth = 0usize;
        let mut expo_depth = 0usize;
        for raw in t.lines() {
            let l = raw.trim();
            if l.is_empty() {
                continue;
            }
            if l.contains("\"expo\"") && l.contains('{') {
                in_expo = true;
                depth += l.matches('{').count();
                expo_depth = depth;
            }
            // count "key": patterns
            let bytes = l.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'"' {
                    i += 1;
                    let start = i;
                    while i < bytes.len() && bytes[i] != b'"' {
                        i += 1;
                    }
                    let s = &l[start..i];
                    i += 1;
                    if i < bytes.len() && bytes[i] == b':' {
                        c.keys += 1;
                        if in_expo && depth == expo_depth {
                            c.expo_keys += 1;
                        }
                        if SECTION_KEYS.contains(&s) {
                            c.sections += 1;
                        }
                    } else {
                        c.list_items += 1;
                    }
                } else {
                    i += 1;
                }
            }
            let closes = l.matches('}').count();
            if in_expo && closes > 0 {
                depth = depth.saturating_sub(closes);
                if depth < expo_depth {
                    in_expo = false;
                }
            }
        }
        if c.keys == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br##"{
  "expo": {
    "name": "my-app",
    "slug": "my-app",
    "version": "1.0.0",
    "orientation": "portrait",
    "icon": "./assets/icon.png",
    "scheme": "myapp",
    "splash": {
      "image": "./assets/splash.png",
      "resizeMode": "contain",
      "backgroundColor": "#ffffff"
    },
    "ios": {
      "bundleIdentifier": "com.example.app",
      "buildNumber": "1"
    },
    "android": {
      "package": "com.example.app",
      "versionCode": 1
    },
    "plugins": ["expo-router", ["expo-notifications", {"icon": "./icon.png"}]],
    "extra": { "eas": { "projectId": "abc" } }
  }
}"##;

    #[test]
    fn detects() {
        assert!(detect(SAMPLE));
        assert!(!detect(br##"{"name": "x"}"##));
    }

    #[test]
    fn parses() {
        let c = AppJson::parse(SAMPLE).unwrap();
        assert!(c.expo_keys >= 7);
        assert!(c.sections >= 4);
        assert!(c.keys >= 15);
        assert!(c.list_items >= 1);
    }

    #[test]
    fn rejects() {
        assert!(AppJson::parse(b"").is_none());
        assert!(AppJson::parse(b"{\"expo\"}").is_none());
    }
}
