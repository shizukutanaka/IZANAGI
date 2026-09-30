//! NuGet `.nupkg`: a ZIP holding `[Content_Types].xml`, `lib/` or
//! `tools/` payloads, and exactly one root `*.nuspec` manifest.
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored("[Content_Types].xml", b"<Types/>");
//! zw.add_stored("lib/net45/x.dll", b"MZ...");
//! zw.add_stored(
//!     "demo.nuspec",
//!     b"<?xml version=\"1\x2e0\"?><package><metadata><id>Demo</id><version>1\x2e2\x2e3</version></metadata></package>",
//! );
//! let n = izanagi_kit::nuget::parse(&zw.finish()).unwrap();
//! assert_eq!(n.id.as_deref(), Some("Demo"));
//! assert_eq!(n.version.as_deref(), Some("1\x2e2\x2e3"));
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected NuGet package.
#[derive(Clone, Debug)]
pub struct Nuget {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// The `.nuspec` member path (rooted at `/` or bare).
    pub nuspec: String,
    /// `<id>` element text when present in the manifest.
    pub id: Option<String>,
    /// `<version>` element text.
    pub version: Option<String>,
    /// Payload file count under `lib/`, `tools/`, `content/`,
    /// `contentFiles/`, or `build/`.
    pub payload_files: usize,
}

fn element(src: &[u8], tag: &[u8]) -> Option<String> {
    let open = [&b"<"[..], tag, &b">"[..]].concat();
    let close = [&b"</"[..], tag, &b">"[..]].concat();
    let start = src
        .windows(open.len())
        .position(|w| w == open.as_slice())?
        .checked_add(open.len())?;
    let rel_end = src
        .get(start..)?
        .windows(close.len())
        .position(|w| w == close.as_slice())?;
    let body = src.get(start..start + rel_end)?;
    String::from(std::str::from_utf8(body).ok()?).into()
}

/// Parse a `.nupkg`: ZIP member list must include
/// `[Content_Types].xml` and a root-level `.nuspec` file, which is
/// then read for `<id>`/`<version>`. Returns `None` otherwise.
pub fn parse(d: &[u8]) -> Option<Nuget> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut nuspec = None;
    let mut content_types = false;
    let mut payload_files = 0usize;
    for e in &entries {
        let name = e.name.clone();
        let bare = name.trim_start_matches('/');
        if bare == "[Content_Types].xml" {
            content_types = true;
        }
        if !bare.contains('/') && bare.ends_with(".nuspec") {
            nuspec = Some(name.clone());
        }
        if bare.starts_with("lib/")
            || bare.starts_with("tools/")
            || bare.starts_with("content/")
            || bare.starts_with("contentFiles/")
            || bare.starts_with("build/")
        {
            payload_files += 1;
        }
        names.push(name);
    }
    if !content_types {
        return None;
    }
    let nuspec = nuspec?;
    let body = crate::zip::extract(d, &nuspec)?;
    Some(Nuget {
        entries: names,
        nuspec,
        id: element(&body, b"id"),
        version: element(&body, b"version"),
        payload_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<Types/>");
        zw.add_stored("lib/net45/x.dll", b"MZ");
        zw.add_stored("tools/install.ps1", b"echo hi");
        zw.add_stored(
            "demo.nuspec",
            b"<package><metadata><id>Demo</id><version>2\x2e0</version></metadata></package>",
        );
        zw.finish()
    }

    #[test]
    fn manifest_extracted() {
        let n = parse(&fixture()).unwrap();
        assert_eq!(n.id.as_deref(), Some("Demo"));
        assert_eq!(n.version.as_deref(), Some("2\x2e0"));
        assert_eq!(n.payload_files, 2);
        assert_eq!(n.nuspec, "demo.nuspec");
    }

    #[test]
    fn rejects_plain_zip() {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("readme.txt", b"hi");
        assert!(parse(&zw.finish()).is_none());
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("[Content_Types].xml", b"<>");
        assert!(parse(&zw2.finish()).is_none()); // no nuspec
        assert!(parse(b"").is_none());
    }
}
