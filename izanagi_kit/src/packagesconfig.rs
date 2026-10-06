//! NuGet `packages.config` XML census.
//!
//! `<packages>` root + `<package id="…" version="…" targetFramework="…"/>`
//! (optionally `developmentDependency`/`allowedVersions`/`userInstalled`)
//! lines.
//!
//! ```rust
//! let p = br#"<packages>
//! <package id="Newtonsoft.Json" version="13.0.3" targetFramework="net48"/>
//! <package id="Serilog" version="4.0.0" targetFramework="net48"/>
//! </packages>"#;
//! assert!(izanagi_kit::packagesconfig::detect(p));
//! let c = izanagi_kit::packagesconfig::Packagesconfig::parse(p).unwrap();
//! assert_eq!(c.packages, 2);
//! ```

/// packages.config census.
#[derive(Debug, Clone)]
pub struct Packagesconfig {
    /// `<package` element lines.
    pub packages: usize,
    /// `<packages` root seen.
    pub has_root: bool,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn package_line(tr: &str) -> bool {
    tr.starts_with("<package ") && (tr.contains("id=") && tr.contains("version="))
}

/// Detect a `packages.config` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut pkgs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<packages") {
            root = true;
            continue;
        }
        if package_line(tr) {
            pkgs += 1;
        }
    }
    root || pkgs >= 2
}

impl Packagesconfig {
    /// Count packages. Returns `None` when the input does not look like
    /// a `packages.config`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            packages: 0,
            has_root: false,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("<packages") {
                c.has_root = true;
                continue;
            }
            if package_line(tr) {
                c.packages += 1;
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
        let b = br#"<?xml version="1.0" encoding="utf-8"?>
<packages>
    <package id="Newtonsoft.Json" version="13.0.3" targetFramework="net48"/>
    <package id="Serilog" version="4.0.0" targetFramework="net48" developmentDependency="false"/>
    <package id="Serilog.Sinks.File" version="6.0.0" targetFramework="net48"/>
    <package id="xunit" version="2.6.0" targetFramework="net48" userInstalled="true"/>
</packages>
"#;
        assert!(detect(b));
        let c = Packagesconfig::parse(b).unwrap();
        assert_eq!(c.packages, 4);
        assert!(c.has_root);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<html></html>"));
        assert!(!detect(b"key=value\n"));
        assert!(Packagesconfig::parse(b"").is_none());
    }
}
