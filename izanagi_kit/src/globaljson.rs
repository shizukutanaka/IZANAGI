//! .NET `global.json` census.
//!
//! `{"sdk": {"version": …, "rollForward": …, "allowPrerelease": …},
//! "tools": {"dotnet": …}, "msbuild-sdks": {"SDK.Name": "ver"},
//! "errorMessage": …, "paths": […]}`.
//!
//! ```rust
//! let g = br#"{
//!   "sdk": {
//!     "version": "8.0.100",
//!     "rollForward": "latestFeature",
//!     "allowPrerelease": false
//!   },
//!   "msbuild-sdks": { "Microsoft.Build.Traversal": "4.1.0" }
//! }"#;
//! assert!(izanagi_kit::globaljson::detect(g));
//! ```

/// global.json census.
#[derive(Debug, Clone)]
pub struct Globaljson {
    /// `"key":` lines matching a known global.json key.
    pub keys: usize,
}

/// global.json keys.
const KEYS: &[&str] = &[
    "allowPrerelease",
    "dotnet",
    "errorMessage",
    "msbuild-sdks",
    "paths",
    "rollForward",
    "sdk",
    "tools",
    "version",
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

/// Detect a `global.json` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    let mut strong = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                n += 1;
            }
            if matches!(
                k,
                "rollForward" | "allowPrerelease" | "msbuild-sdks" | "sdk"
            ) {
                strong += 1;
            }
        }
    }
    strong >= 1 || n >= 3
}

impl Globaljson {
    /// Count keys. Returns `None` when the input does not look like a
    /// `global.json`.
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
    "sdk": {
        "version": "8.0.100",
        "rollForward": "latestFeature",
        "allowPrerelease": false,
        "errorMessage": "The .NET SDK could not be found"
    },
    "tools": {
        "dotnet": "8.0.100"
    },
    "msbuild-sdks": {
        "Microsoft.Build.Traversal": "4.1.0"
    }
}
"#;
        assert!(detect(b));
        let c = Globaljson::parse(b).unwrap();
        assert_eq!(c.keys, 8);
    }

    #[test]
    fn rejects_package_json() {
        assert!(!detect(br#"{"name":"x","version":"1.0.0"}"#));
        assert!(Globaljson::parse(b"").is_none());
    }
}
