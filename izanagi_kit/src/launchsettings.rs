//! Visual Studio `Properties/launchSettings.json` census.
//!
//! `iisSettings`/`profiles` top keys; profile keys `commandName`,
//! `applicationUrl`, `launchUrl`, `launchBrowser`, `launchMode`,
//! `environmentVariables`, `dotnetRunMessages`, `inspectUri`,
//! `sslPort`, `httpPort`, `useSSL`, `windowsAuthentication`,
//! `anonymousAuthentication`, `iisExpress`, `hotReloadEnabled`,
//! `nativeDebugging`, `sqlDebugging`, `publishAllPorts`,
//! `use64Bit` …
//!
//! ```rust
//! let l = br#"{
//!   "iisSettings": {"windowsAuthentication": false, "anonymousAuthentication": true},
//!   "profiles": {
//!     "web": {
//!       "commandName": "Project",
//!       "launchBrowser": true,
//!       "applicationUrl": "http://localhost:5000",
//!       "environmentVariables": {"ASPNETCORE_ENVIRONMENT": "Development"}
//!     }
//!   }
//! }"#;
//! assert!(izanagi_kit::launchsettings::detect(l));
//! ```

/// launchSettings.json census.
#[derive(Debug, Clone)]
pub struct Launchsettings {
    /// `"key":` lines matching known launch-settings keys.
    pub keys: usize,
}

/// launchSettings keys.
const KEYS: &[&str] = &[
    "$schema",
    "anonymousAuthentication",
    "applicationUrl",
    "ASPNETCORE_ENVIRONMENT",
    "commandArgs",
    "commandName",
    "cssEnabled",
    "dotnetRunMessages",
    "environmentVariables",
    "executablePath",
    "externalUrlConfiguration",
    "hotReloadEnabled",
    "httpPort",
    "iisExpress",
    "iisSettings",
    "inspectUri",
    "jsdebug",
    "launchBrowser",
    "launchMode",
    "launchUrl",
    "nativeDebugging",
    "publishAllPorts",
    "remoteDebugEnabled",
    "remoteDebugMachine",
    "sslPort",
    "sqlDebugging",
    "use64Bit",
    "useSSL",
    "webBrowser",
    "windowsAuthentication",
    "workingDirectory",
];

fn jkey(l: &str) -> Option<&str> {
    let t = l.trim_start();
    let t = t.strip_prefix('"')?;
    let end = t.find('"')?;
    let key = &t[..end];
    if key.is_empty() {
        return None;
    }
    let rest = t[end + 1..].trim_start();
    if rest.starts_with(':') {
        Some(key)
    } else {
        None
    }
}

/// Detect a `launchSettings.json` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Launchsettings {
    /// Count keys. Returns `None` when the input does not look like a
    /// `launchSettings.json`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self { keys: 0 };
        for l in t.lines() {
            if let Some(k) = jkey(l) {
                if KEYS.contains(&k) {
                    c.keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"{
    "$schema": "https://json.schemastore.org/launchsettings.json",
    "iisSettings": {
        "windowsAuthentication": false,
        "anonymousAuthentication": true,
        "iisExpress": {
            "applicationUrl": "http://localhost:12345",
            "sslPort": 44312
        }
    },
    "profiles": {
        "web": {
            "commandName": "Project",
            "launchBrowser": true,
            "launchUrl": "swagger",
            "applicationUrl": "https://localhost:7001;http://localhost:5001",
            "dotnetRunMessages": true,
            "environmentVariables": {
                "ASPNETCORE_ENVIRONMENT": "Development"
            }
        },
        "IIS Express": {
            "commandName": "IISExpress",
            "launchBrowser": true,
            "environmentVariables": {
                "ASPNETCORE_ENVIRONMENT": "Development"
            }
        }
    }
}
"#;
        assert!(detect(b));
        let c = Launchsettings::parse(b).unwrap();
        assert!(c.keys >= 12);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(br#"{"name":"x","profiles":{"a":{}}}"#));
        assert!(Launchsettings::parse(b"").is_none());
    }
}
