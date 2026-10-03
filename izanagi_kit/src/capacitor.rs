//! Census of a Capacitor `capacitor.config.ts` / `capacitor.config.json`.
//!
//! `const config: CapacitorConfig = {…}` + `export default config` (TS) or
//! plain JSON. Keys: `appId`, `appName`, `webDir`, `server`
//! (`url`/`cleartext`/`androidScheme`/`iosScheme`/`hostname`/`allowNavigation`),
//! `plugins` (`SplashScreen`/`PushNotifications`/`Keyboard`/`LocalNotifications`/
//! `CapacitorHttp`/`CapacitorCookies`/`CapacitorUpdater`/`CapacitorGoogleAuth`),
//! `android` (`path`/`webDir`/`buildOptions`/`includePlugins`/`flavor`/
//! `allowMixedContent`/`captureInput`/`webContentsDebuggingEnabled`),
//! `ios` (`path`/`scheme`/`contentInset`/`scrollEnabled`/`allowsLinkPreview`/
//! `limitsNavigationsToAppBoundDomains`/`preferredContentMode`/`backgroundColor`),
//! `bundledWebRuntime`, `cordova.staticPlugins`, `includePlugins`,
//! `linuxAndroidStudioPath`, `overrideUserAgent`, `appendUserAgent`,
//! `backgroundColor`, `loggingBehavior`, `webContentsDebuggingEnabled`,
//! `hideLogs`.
//!
//! ```rust
//! let c = izanagi_kit::capacitor::Capacitor::parse(
//!     b"const config: CapacitorConfig = { appId: 'com.example.app', appName: 'app', webDir: 'dist' };\nexport default config;",
//! ).unwrap();
//! assert!(c.keys >= 2);
//! ```
#![forbid(unsafe_code)]

/// capacitor.config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacitor {
    /// `key:` / `"key":` keys total.
    pub keys: usize,
    /// `appId`/`appName`/`webDir` identity keys found.
    pub identity: usize,
    /// `plugins` object entries (nested `Name:` keys inside `plugins`).
    pub plugins: usize,
    /// `server`/`android`/`ios`/`cordova`/`windows` section objects.
    pub sections: usize,
    /// `allowNavigation` entries.
    pub navigations: usize,
}

/// Identity keys.
const ID_KEYS: &[&str] = &["appId", "appName", "webDir"];
/// Section keys.
const SECTION_KEYS: &[&str] = &["server", "android", "ios", "cordova", "windows", "electron"];

/// True if `b` looks like a capacitor config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("CapacitorConfig")
        || t.contains("capacitor")
        || (t.contains("appId") && t.contains("webDir")))
        && t.contains('{')
}

impl Capacitor {
    /// Count the census fields; `None` unless [`detect`] holds.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            identity: 0,
            plugins: 0,
            sections: 0,
            navigations: 0,
        };
        let mut in_plugins = false;
        let mut plugins_depth = 0usize;
        let mut depth = 0usize;
        for raw in t.lines() {
            let l = raw.trim();
            if l.is_empty() || l.starts_with("//") || l.starts_with("import ") || l.starts_with('*')
            {
                continue;
            }
            depth += l.matches('{').count();
            if l.contains("plugins") && l.contains('{') && !in_plugins {
                in_plugins = true;
                plugins_depth = depth;
            }
            if l.starts_with('}') && in_plugins && depth <= plugins_depth {
                in_plugins = false;
            }
            for seg in l.split(',') {
                let s = seg.trim();
                if let Some(colon) = s.find(':') {
                    let key = s[..colon]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("");
                    if key.is_empty()
                        || !key
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
                    {
                        continue;
                    }
                    c.keys += 1;
                    if ID_KEYS.contains(&key) {
                        c.identity += 1;
                    }
                    if SECTION_KEYS.contains(&key) && s[colon + 1..].contains('{') {
                        c.sections += 1;
                    }
                    if in_plugins && depth > plugins_depth {
                        c.plugins += 1;
                    }
                    if key == "allowNavigation" {
                        c.navigations += 1;
                    }
                } else if in_plugins
                    && depth > plugins_depth
                    && s.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '\'' || ch == '"')
                {
                    c.plugins += 1;
                }
            }
            depth = depth.saturating_sub(l.matches('}').count());
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
  appId: 'com.example.app',
  appName: 'my-app',
  webDir: 'dist',
  server: {
    androidScheme: 'https',
    allowNavigation: ['api.example.com'],
  },
  plugins: {
    SplashScreen: {
      launchShowDuration: 3000,
      backgroundColor: '#ffffffff',
    },
    PushNotifications: {
      presentationOptions: ['badge', 'sound', 'alert'],
    },
  },
  android: {
    allowMixedContent: true,
  },
};

export default config;";

    #[test]
    fn detects_config() {
        assert!(detect(SAMPLE));
        assert!(!detect(b"const x = {};"));
    }

    #[test]
    fn parses() {
        let c = Capacitor::parse(SAMPLE).unwrap();
        assert_eq!(c.identity, 3);
        assert!(c.sections >= 2);
        assert!(c.keys >= 8);
        assert_eq!(c.navigations, 1);
    }

    #[test]
    fn rejects() {
        assert!(Capacitor::parse(b"").is_none());
        assert!(Capacitor::parse(b"{\"name\": \"x\"}").is_none());
    }
}
