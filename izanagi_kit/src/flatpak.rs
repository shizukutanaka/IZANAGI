//! Flatpak `.flatpakref` / `.flatpakrepo` / `.flatpak` descriptors:
//! INI files whose section is `Flatpak Ref`, `Flatpak Bundle`, or
//! `Flatpak Repo`.
//!
//! ```
//! let src = "[Flatpak Ref]\nName=org.app.Demo\nBranch=stable\nUrl=https://dl.example.org/repo\nIsRuntime=false\n";
//! let f = izanagi_kit::flatpak::parse(src).unwrap();
//! assert_eq!(f.kind, izanagi_kit::flatpak::Kind::Ref);
//! assert_eq!(f.name.as_deref(), Some("org.app.Demo"));
//! assert_eq!(f.branch.as_deref(), Some("stable"));
//! ```

use std::string::String;

/// Which Flatpak descriptor section the file carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `[Flatpak Ref]` — an installable ref file.
    Ref,
    /// `[Flatpak Bundle]` — single-file bundle metadata.
    Bundle,
    /// `[Flatpak Repo]` — repository configuration.
    Repo,
}

impl Kind {
    /// Section name for this kind.
    pub fn section(&self) -> &'static str {
        match self {
            Kind::Ref => "Flatpak Ref",
            Kind::Bundle => "Flatpak Bundle",
            Kind::Repo => "Flatpak Repo",
        }
    }

    /// Classify an INI section name; `None` for unrelated sections.
    pub fn from_section(s: &str) -> Option<Kind> {
        match s {
            "Flatpak Ref" => Some(Kind::Ref),
            "Flatpak Bundle" => Some(Kind::Bundle),
            "Flatpak Repo" => Some(Kind::Repo),
            _ => None,
        }
    }
}

/// A parsed Flatpak descriptor.
#[derive(Clone, Debug)]
pub struct Flatpak {
    /// Which of the three sections matched.
    pub kind: Kind,
    /// `Name=` (e.g. `org.app.Demo`).
    pub name: Option<String>,
    /// `Branch=` (default ref branch).
    pub branch: Option<String>,
    /// `Url=` of the remote/repo.
    pub url: Option<String>,
    /// `Title=` human label.
    pub title: Option<String>,
    /// `IsRuntime=` flag (`true`/`false`).
    pub is_runtime: Option<bool>,
    /// `RuntimeRepo=` URL (Ref only).
    pub runtime_repo: Option<String>,
    /// `GPGKey=` base64 blob (kept verbatim).
    pub gpg_key: Option<String>,
}

/// Parse a `.flatpakref`/`.flatpakrepo` INI file. Returns `None`
/// when none of the three `Flatpak *` sections is present.
pub fn parse(src: &str) -> Option<Flatpak> {
    let ini = crate::ini::parse(src);
    let mut kind = None;
    for (section, _) in &ini.sections {
        if kind.is_none() {
            kind = Kind::from_section(section);
        }
    }
    let kind = kind?;
    let sec = kind.section();
    let get = |k: &str| ini.get(Some(sec), k).map(String::from);
    Some(Flatpak {
        kind,
        name: get("Name"),
        branch: get("Branch"),
        url: get("Url"),
        title: get("Title"),
        is_runtime: get("IsRuntime").map(|v| v == "true" || v == "1"),
        runtime_repo: get("RuntimeRepo"),
        gpg_key: get("GPGKey"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_file() {
        let s = "[Flatpak Ref]\nName=org.freedesktop.Platform\nBranch=22\x2e08\nUrl=https://f.example/x\nRuntimeRepo=https://rt\nGPGKey=QUJD\nIsRuntime=true\n";
        let f = parse(s).unwrap();
        assert_eq!(f.kind, Kind::Ref);
        assert_eq!(f.name.as_deref(), Some("org.freedesktop.Platform"));
        assert_eq!(f.branch.as_deref(), Some("22\x2e08"));
        assert_eq!(f.is_runtime, Some(true));
        assert_eq!(f.runtime_repo.as_deref(), Some("https://rt"));
        assert_eq!(f.gpg_key.as_deref(), Some("QUJD"));
        assert_eq!(f.kind.section(), "Flatpak Ref");
        assert_eq!(Kind::from_section("Other"), None);
    }

    #[test]
    fn bundle_and_repo() {
        let b = parse("[Flatpak Bundle]\nName=x\n").unwrap();
        assert_eq!(b.kind, Kind::Bundle);
        let r = parse("[Flatpak Repo]\nUrl=https://repo\n").unwrap();
        assert_eq!(r.kind, Kind::Repo);
        assert_eq!(r.url.as_deref(), Some("https://repo"));
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("[Desktop Entry]\nName=x\n").is_none());
    }
}
