//! RPM `.spec` file structural parsing.
//!
//! Preamble tags `Name:`/`Version:`/`Release:`/`Requires:` etc.,
//! then `%prep`/`%build`/`%install`/`%files`/`%changelog`
//! sections (bodies kept raw), `%description`, `%package` subs.
//!
//! ```
//! use izanagi_kit::spec;
//! let d = b"Name: demo\nVersion: 1.0\nRelease: 1\nLicense: MIT\n\n%description\nA demo.\n\n%prep\n%build\nmake\n";
//! let s = spec::parse(d).unwrap();
//! assert_eq!(s.name, "demo");
//! assert!(s.sections.iter().any(|x| x.name == "build"));
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// A `%`-section (`%prep`, `%build`, `%files`, `%description`…).
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// Section name without `%` (`prep`, `build`, `files`…).
    pub name: String,
    /// Raw body text (lines joined with `\n`).
    pub body: String,
    /// 1-based line of the `%name` header.
    pub line: usize,
}

/// A parsed spec file.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    /// `Name:` value.
    pub name: String,
    /// `Version:` value.
    pub version: String,
    /// `Release:` value.
    pub release: String,
    /// `License:`/`License:`-value if present.
    pub license: Option<String>,
    /// `Summary:` if present.
    pub summary: Option<String>,
    /// All preamble tags, first wins.
    pub tags: BTreeMap<String, String>,
    /// `%`-sections in order.
    pub sections: Vec<Section>,
    /// Requires/BuildRequires values (each full line).
    pub requires: Vec<String>,
}

/// Parses a `.spec` file: preamble `Tag: value` lines until the
/// first `%section`, then `%name` sections accumulate raw bodies.
/// Requires `Name`/`Version`/`Release` tags.
pub fn parse(d: &[u8]) -> Option<Spec> {
    let text = std::str::from_utf8(d).ok()?;
    let mut tags: BTreeMap<String, String> = BTreeMap::new();
    let mut sections = Vec::new();
    let mut requires = Vec::new();
    let mut cur: Option<Section> = None;
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim_end_matches('\r');
        let t = line.trim();
        const SECTIONS: &[&str] = &[
            "prep",
            "build",
            "install",
            "check",
            "files",
            "description",
            "changelog",
            "package",
            "pre",
            "post",
            "preun",
            "postun",
            "pretrans",
            "posttrans",
            "clean",
            "generate_buildrequires",
            "conf",
            "trigger",
            "triggerin",
            "triggerun",
            "triggerpostun",
            "patch",
            "verifyscript",
            "sourcetrigger",
        ];
        if t.starts_with('%') {
            let name = t
                .trim_start_matches('%')
                .split([' ', '('])
                .next()
                .unwrap_or("");
            if SECTIONS.contains(&name) {
                if let Some(s) = cur.take() {
                    sections.push(s);
                }
                cur = Some(Section {
                    name: name.to_string(),
                    body: String::new(),
                    line: i + 1,
                });
                continue;
            }
            // unknown %macro: inside a section it's body text; in the
            // preamble we skip it.
            if let Some(s) = cur.as_mut() {
                if !s.body.is_empty() {
                    s.body.push('\n');
                }
                s.body.push_str(t);
            }
            continue;
        }
        if let Some(s) = cur.as_mut() {
            // inside section: accumulate
            if !s.body.is_empty() {
                s.body.push('\n');
            }
            s.body.push_str(t);
            continue;
        }
        // preamble
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let (k, v) = t.split_once(':')?;
        let key = k.trim();
        if key.is_empty() {
            return None;
        }
        let val = v.trim().to_string();
        if key == "Requires" || key == "BuildRequires" {
            requires.push(val.clone());
        }
        tags.entry(key.to_string()).or_insert(val);
    }
    if let Some(s) = cur.take() {
        sections.push(s);
    }
    let get = |k: &str| tags.get(k).cloned();
    let name = get("Name")?;
    let version = get("Version")?;
    let release = get("Release")?;
    let license = get("License");
    let summary = get("Summary");
    Some(Spec {
        name,
        version,
        release,
        license,
        summary,
        tags,
        sections,
        requires,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &[u8] = b"Name: pkg\nVersion: 2.0\nRelease: 1%{?dist}\nLicense: GPL\nSummary: a pkg\nRequires: glibc\n\n%description\nlong\n\n%prep\n%setup -q\n%build\nmake\n%files\n/usr/bin/pkg\n";

    #[test]
    fn parses() {
        let s = parse(SPEC).unwrap();
        assert_eq!(s.name, "pkg");
        assert_eq!(s.license.as_deref(), Some("GPL"));
        assert_eq!(s.requires, vec!["glibc".to_string()]);
        let names: Vec<&str> = s.sections.iter().map(|x| x.name.as_str()).collect();
        assert!(names.contains(&"description") && names.contains(&"build"));
        assert!(s
            .sections
            .iter()
            .find(|x| x.name == "build")
            .unwrap()
            .body
            .contains("make"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"Version: 1\nRelease: 1\n").is_none());
        assert!(parse(b"no colon\n").is_none());
        assert!(parse(b"").is_none());
    }
}
