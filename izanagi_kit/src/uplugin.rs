//! Unreal Engine `.uplugin` plugin descriptor parser.
//!
//! JSON descriptor: `FileVersion`, `Version`, `VersionName`,
//! `FriendlyName`, `Description`, `Category`, `CanContainContent`,
//! `Modules`, `Plugins` (dependency list).
//!
//! ```
//! use izanagi_kit::uplugin::Uplugin;
//! let src = br#"{"FileVersion":3,"Version":1,"VersionName":"1\x2e0","FriendlyName":"Demo","CanContainContent":true,"Modules":[{"Name":"Demo","Type":"Runtime"}],"Plugins":[{"Name":"Engine","Enabled":true}]}"#;
//! assert!(izanagi_kit::uplugin::detect(src));
//! let p = Uplugin::parse(src).unwrap();
//! assert_eq!(p.friendly_name, "Demo");
//! assert!(p.can_contain_content);
//! assert_eq!(p.modules, 1);
//! assert_eq!(p.plugins, 1);
//! ```

/// Parsed census of a `.uplugin` file.
#[derive(Debug, Clone)]
pub struct Uplugin {
    /// `"FileVersion"`.
    pub file_version: u32,
    /// `"Version"`.
    pub version: u32,
    /// `"VersionName"` value.
    pub version_name: String,
    /// `"FriendlyName"` value.
    pub friendly_name: String,
    /// `"Category"` value.
    pub category: String,
    /// `"CanContainContent"`.
    pub can_contain_content: bool,
    /// Entries in `Modules`.
    pub modules: usize,
    /// Entries in `Plugins` (dependencies).
    pub plugins: usize,
    /// `"SupportedTargetPlatforms"` entries.
    pub supported_platforms: usize,
}

fn jstr<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\"", key);
    let i = t.find(&pat)?;
    let s = t[i + pat.len()..].trim_start();
    let s = s.strip_prefix(':')?.trim_start();
    if !s.starts_with('"') {
        return None;
    }
    let s = &s[1..];
    let end = s.find('"')?;
    Some(&s[..end])
}

fn jint(t: &str, key: &str) -> u32 {
    let pat = format!("\"{}\"", key);
    let Some(i) = t.find(&pat) else { return 0 };
    let s = t[i + pat.len()..].trim_start();
    let Some(s) = s.strip_prefix(':') else {
        return 0;
    };
    let s = s.trim_start();
    let digits: usize = s.bytes().take_while(u8::is_ascii_digit).count();
    s[..digits].parse().unwrap_or(0)
}
fn jkey(t: &str, key: &str) -> bool {
    // `"key"` が値位置ではなくキー位置(直後が `:`)にあるかを確認。
    let pat = format!("\"{key}\"");
    let mut rest = t;
    while let Some(i) = rest.find(&pat) {
        rest = &rest[i + pat.len()..];
        if rest.trim_start().starts_with(':') {
            return true;
        }
    }
    false
}

/// Returns `true` when `b` looks like a `.uplugin` descriptor.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    jkey(t, "FriendlyName") && jkey(t, "VersionName")
}

impl Uplugin {
    /// Parses a `.uplugin`; `None` without `FriendlyName`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let friendly_name = jstr(t, "FriendlyName")?.to_string();
        let modules = match t.find("\"Modules\"") {
            Some(mi) => {
                let end = t.find("\"Plugins\"").filter(|p| *p > mi).unwrap_or(t.len());
                let region = &t[mi..end];
                region.matches("\"Name\"").count()
            }
            None => 0,
        };
        let plugins = match t.find("\"Plugins\"") {
            Some(pi) => t[pi..].matches("\"Name\"").count(),
            None => 0,
        };
        Some(Self {
            file_version: jint(t, "FileVersion"),
            version: jint(t, "Version"),
            version_name: jstr(t, "VersionName").unwrap_or("").to_string(),
            friendly_name,
            category: jstr(t, "Category").unwrap_or("").to_string(),
            can_contain_content: t.contains("\"CanContainContent\": true")
                || t.contains("\"CanContainContent\":true"),
            modules,
            plugins,
            supported_platforms: t.matches("\"SupportedTargetPlatforms\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(
            b"{\"note\": \"FriendlyName\", \"v\": \"VersionName\"}\n"
        ));
    }

    #[test]
    fn detects_friendly() {
        assert!(detect(br#"{"FriendlyName":"X","VersionName":"1\x2e0"}"#));
        assert!(!detect(br#"{"EngineAssociation":"5\x2e3"}"#));
    }

    #[test]
    fn parse_none_on_project() {
        assert!(Uplugin::parse(br#"{"FileVersion":3,"EngineAssociation":"5"}"#).is_none());
    }
}
