//! Arch Linux PKGBUILD field parsing.
//!
//! Bash-style `name=value` / `name=(a b c)` arrays, `#` comments,
//! continuation inside arrays; functions (`prepare() {`) noted
//! but not executed. Required fields: `pkgname`, `pkgver`, `pkgrel`.
//!
//! ```
//! use izanagi_kit::pkgbuild;
//! let d = b"pkgname=demo\npkgver=1.0\npkgrel=1\narch=('x86_64')\ndepends=('glibc' 'zlib')\n";
//! let p = pkgbuild::parse(d).unwrap();
//! assert_eq!(p.pkgname, "demo");
//! assert_eq!(p.arrays["arch"], vec!["x86_64".to_string()]);
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// A parsed PKGBUILD.
#[derive(Clone, Debug, PartialEq)]
pub struct Pkgbuild {
    /// `pkgname` value.
    pub pkgname: String,
    /// `pkgver` value.
    pub pkgver: String,
    /// `pkgrel` value.
    pub pkgrel: String,
    /// `pkgdesc` if present.
    pub pkgdesc: Option<String>,
    /// Scalar fields (`url`, `license`-as-string, `epoch`…).
    pub scalars: BTreeMap<String, String>,
    /// Array fields (`arch`, `depends`, `makedepends`, `source`…).
    pub arrays: BTreeMap<String, Vec<String>>,
    /// Function names seen (`prepare`, `build`, `package`…).
    pub functions: Vec<String>,
}

/// Parses a PKGBUILD: `name=value` scalars, `name=(…)` arrays
/// (possibly spanning lines), shell identifiers only; requires
/// `pkgname`, `pkgver`, `pkgrel`.
pub fn parse(d: &[u8]) -> Option<Pkgbuild> {
    let text = std::str::from_utf8(d).ok()?;
    let mut scalars: BTreeMap<String, String> = BTreeMap::new();
    let mut arrays: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut functions = Vec::new();
    let mut fn_depth = 0u32;
    let mut it = text.split('\n').peekable();
    while let Some(raw) = it.next() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if fn_depth > 0 {
            // inside a function body: count braces and skip
            fn_depth = fn_depth.saturating_add(line.bytes().filter(|&b| b == b'{').count() as u32);
            fn_depth = fn_depth.saturating_sub(line.bytes().filter(|&b| b == b'}').count() as u32);
            continue;
        }
        // function definition?
        if let Some(p) = line.find('(') {
            if line[..p]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_')
                && line[p + 1..].trim_start().starts_with(')')
            {
                functions.push(line[..p].to_string());
                fn_depth = line.bytes().filter(|&b| b == b'{').count() as u32;
                fn_depth =
                    fn_depth.saturating_sub(line.bytes().filter(|&b| b == b'}').count() as u32);
                continue;
            }
        }
        let (name, val) = line.split_once('=')?;
        let name = name.trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
        {
            return None;
        }
        let val = val.trim();
        if let Some(open) = val.strip_prefix('(') {
            // array: may continue across lines until ')'
            let mut body = open.to_string();
            while !body.contains(')') {
                let n = it.next()?;
                body.push(' ');
                body.push_str(n.trim());
            }
            let close = body.find(')')?;
            let inner = &body[..close];
            let items: Vec<String> = inner
                .split_whitespace()
                .map(|w| w.trim_matches(|c| c == '\'' || c == '"').to_string())
                .filter(|w| !w.is_empty())
                .collect();
            arrays.insert(name.to_string(), items);
        } else {
            scalars.insert(
                name.to_string(),
                val.trim_matches(|c| c == '\'' || c == '"').to_string(),
            );
        }
    }
    let get = |k: &str| scalars.get(k).cloned();
    let pkgname = get("pkgname")?;
    let pkgver = get("pkgver")?;
    let pkgrel = get("pkgrel")?;
    let pkgdesc = get("pkgdesc");
    Some(Pkgbuild {
        pkgname,
        pkgver,
        pkgrel,
        pkgdesc,
        scalars,
        arrays,
        functions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PKGBUILD: &[u8] = b"# Maintainer\npkgname=demo\npkgver=1.2\npkgrel=3\npkgdesc=\"A demo\"\narch=('x86_64' 'aarch64')\ndepends=('glibc'\n         'zlib')\nsource=(\"$pkgname.tar.gz\")\nbuild() {\n  make\n}\npackage() { :; }\n";

    #[test]
    fn parses() {
        let p = parse(PKGBUILD).unwrap();
        assert_eq!(p.pkgname, "demo");
        assert_eq!(p.pkgrel, "3");
        assert_eq!(p.pkgdesc.as_deref(), Some("A demo"));
        assert_eq!(
            p.arrays.get("arch").unwrap(),
            &vec!["x86_64".to_string(), "aarch64".to_string()]
        );
        assert_eq!(
            p.arrays.get("depends").unwrap(),
            &vec!["glibc".to_string(), "zlib".to_string()]
        );
        assert!(p.functions.contains(&"build".to_string()));
        assert!(p.functions.contains(&"package".to_string()));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"pkgver=1\npkgrel=1\n").is_none()); // no pkgname
        assert!(parse(b"pkgname=x\npkgver=1\npkgrel=1\ndepends=(unclosed\n").is_none());
        assert!(parse(b"pkgname=x\npkgver=1\npkgrel=1\nBAD LINE\n").is_none());
        assert!(parse(b"").is_none());
    }
}
