//! Unreal Engine `.uproject` descriptor parser.
//!
//! JSON descriptor: `FileVersion`, `EngineAssociation`, `Category`,
//! `Description`, `Modules` (`Name`/`Type`/`LoadingPhase`), `Plugins`
//! (`Name`/`Enabled`).
//!
//! ```
//! use izanagi_kit::uproject::Uproject;
//! let src = br#"{"FileVersion":3,"EngineAssociation":"5\x2e3","Category":"","Description":"","Modules":[{"Name":"MyGame","Type":"Runtime","LoadingPhase":"Default"}],"Plugins":[{"Name":"ModelView","Enabled":true}]}"#;
//! assert!(izanagi_kit::uproject::detect(src));
//! let p = Uproject::parse(src).unwrap();
//! assert_eq!(p.file_version, 3);
//! assert_eq!(p.modules, 1);
//! assert_eq!(p.plugins, 1);
//! ```

/// Parsed census of a `.uproject` file.
#[derive(Debug, Clone)]
pub struct Uproject {
    /// `"FileVersion"`.
    pub file_version: u32,
    /// `"EngineAssociation"` value (e.g. `5.3`, GUID for custom builds).
    pub engine_association: String,
    /// `"Category"` value.
    pub category: String,
    /// Entries in `Modules` (one `"LoadingPhase"` per module).
    pub modules: usize,
    /// `"Type"` fields inside `Modules` (Runtime/Editor/...).
    pub module_types: usize,
    /// Entries in `Plugins`.
    pub plugins: usize,
    /// `Plugins` entries with `"Enabled": true`.
    pub enabled_plugins: usize,
    /// `"TargetPlatforms"` entries.
    pub target_platforms: usize,
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

/// Returns `true` when `b` looks like a `.uproject` descriptor.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    jkey(t, "EngineAssociation") && jkey(t, "FileVersion")
}

impl Uproject {
    /// Parses a `.uproject`; `None` without `EngineAssociation`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let engine_association = jstr(t, "EngineAssociation")?.to_string();
        let plugins_start = t.find("\"Plugins\"");
        let (mod_region, plug_region) = match plugins_start {
            Some(pi) => (&t[..pi], &t[pi..]),
            None => (t, ""),
        };
        let modules = mod_region.matches("\"LoadingPhase\"").count();
        let module_types = mod_region.matches("\"Type\"").count();
        let plugins = plug_region.matches("\"Name\"").count();
        let enabled_plugins = plug_region.matches("\"Enabled\": true").count()
            + plug_region.matches("\"Enabled\":true").count();
        Some(Self {
            file_version: jint(t, "FileVersion"),
            engine_association,
            category: jstr(t, "Category").unwrap_or("").to_string(),
            modules,
            module_types,
            plugins,
            enabled_plugins,
            target_platforms: t.matches("\"TargetPlatforms\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(
            b"{\"note\": \"EngineAssociation\", \"v\": \"FileVersion\"}\n"
        ));
    }

    #[test]
    fn detects_engine_assoc() {
        assert!(detect(
            br#"{"FileVersion":3,"EngineAssociation":"4\x2e27"}"#
        ));
        assert!(!detect(br#"{"FileVersion":1,"FriendlyName":"X"}"#));
    }

    #[test]
    fn counts_modules() {
        let src = br#"{"FileVersion":3,"EngineAssociation":"5\x2e0","Modules":[{"Name":"A","Type":"Runtime","LoadingPhase":"Default"},{"Name":"B","Type":"Editor","LoadingPhase":"PostEngineInit"}]}"#;
        let p = Uproject::parse(src).unwrap();
        assert_eq!(p.modules, 2);
        assert_eq!(p.module_types, 2);
    }
}
