//! CocoaPods `Podfile` census.
//!
//! Podfile is Ruby DSL: `platform :ios, '17.0'`,
//! `use_frameworks!`/`use_modular_headers!`/`use_frameworks! :linkage => :static`,
//! `pod 'Alamofire'`/`pod 'X', '~> 1.0'`/`pod 'X', :path => 'y'`,
//! `target 'App' do … end`, `abstract_target`, `post_install do |installer|`,
//! `pre_install`, `source 'https://…'`, `workspace`, `project`,
//! `plugin`, `inhibit_all_warnings!`, `install! 'cocoapods'`,
//! `podspec`, `script_phases`, `supports_swift_versions`,
//! `inherit! :search_paths`, `def` helper blocks, `flutter_*`,
//! `prepare_command`, `config`, `generate_podfile`.
//!
//! ```rust
//! let k = b"platform :ios, '17.0'\nuse_frameworks!\ntarget 'App' do\n  pod 'Alamofire'\n  pod 'X', '~> 1.0'\nend\n";
//! assert!(izanagi_kit::podfile::detect(k));
//! ```

/// Podfile census.
#[derive(Debug, Clone)]
pub struct Podfile {
    /// `pod`/`target`/`*!` DSL lines.
    pub directives: usize,
    /// `do`/`end` block markers.
    pub blocks: usize,
    /// recognised DSL calls present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const CALLS: &[&str] = &[
    "platform",
    "use_frameworks!",
    "use_modular_headers!",
    "pod ",
    "podspec",
    "target ",
    "abstract_target",
    "post_install",
    "pre_install",
    "post_integrate",
    "source ",
    "workspace",
    "project",
    "plugin",
    "inhibit_all_warnings!",
    "install!",
    "script_phases",
    "supports_swift_versions",
    "inherit!",
    "prepare_command",
    "generate_podfile",
    "test_spec",
    "app_spec",
    "def ",
    "flutter_",
];

fn call(line: &str) -> Option<&'static str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    CALLS.iter().find(|&&c| s.starts_with(c)).copied()
}

/// Detect a CocoaPods `Podfile`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `use_frameworks!`/`pod 'X'`/`platform :ios`/`abstract_target`
    // are CocoaPods-only DSL calls.
    let mut n = 0usize;
    for line in t.lines() {
        if call(line).is_some() {
            n += 1;
        }
    }
    n >= 2
}

impl Podfile {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            directives: 0,
            blocks: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.ends_with(" do") || s.contains(" do |") || s == "end" {
                c.blocks += 1;
            }
            if call(line).is_some() {
                c.directives += 1;
                c.keys += 1;
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
        let b = b"platform :ios, '17.0'\nuse_frameworks!\ntarget 'App' do\n  pod 'Alamofire'\n  pod 'X', '~> 1.0'\nend\n";
        assert!(detect(b));
        let c = Podfile::parse(b).unwrap();
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"x = 1\ny = 2\n"));
        assert!(!detect(b"# pod 'x'\n# platform :ios\nz = 1\n"));
    }
}
