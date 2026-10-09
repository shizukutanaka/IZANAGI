//! API Blueprint (`FORMAT: 1A`) Markdown — `.apib` files with
//! `## Group`/`### Resource [METHOD /path]`/`+ Request`/`+ Response` structure.

use crate::textutil::strip_xml_comments;
use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of an API Blueprint document.
pub struct Apib {
    /// `FORMAT:` version string.
    pub format: String,
    /// API name (`# <name>` first heading).
    pub name: String,
    /// `## Group <name>` sections.
    pub groups: usize,
    /// `### <name> [METHOD /path]` action headings.
    pub resources: usize,
    /// `+ Request` blocks.
    pub requests: usize,
    /// `+ Response` blocks.
    pub responses: usize,
    /// `## Data Structures` entries (`### name` inside it).
    pub data_structures: usize,
    /// `+ Parameters` sections.
    pub parameters: usize,
    /// `+ Attributes` sections.
    pub attributes: usize,
    /// `+ Headers` sections.
    pub headers: usize,
    /// `+ Body` sections.
    pub bodies: usize,
    /// `+ Schema` sections.
    pub schemas: usize,
    /// `## Resource`/`## <name> [/path]` resource group headings.
    pub resource_groups: usize,
    /// `<!--`/`+ Note:`/`+ Description:` docs.
    pub docs: usize,
}

fn heading(l: &str, hashes: usize) -> Option<&str> {
    l.trim()
        .strip_prefix(&"#".repeat(hashes))
        .filter(|r| r.starts_with(' '))
        .map(str::trim)
}

/// `true` when the text looks like an API Blueprint document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    let t = strip_xml_comments(t);
    let has_format = t.lines().take(4).any(|l| l.trim() == "FORMAT: 1A");
    let has_actions = t
        .lines()
        .any(|l| l.trim().starts_with("+ Request") || l.trim().starts_with("+ Response"));
    has_format || (has_actions && t.contains("### "))
}

impl Apib {
    #[must_use]
    /// Parses `b` into `Apib`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = strip_xml_comments(from_utf8(b).ok()?);
        if !detect(b) {
            return None;
        }
        let mut r = Self {
            format: t
                .lines()
                .find_map(|l| l.trim().strip_prefix("FORMAT:"))
                .unwrap_or("")
                .trim()
                .to_string(),
            name: t
                .lines()
                .find_map(|l| heading(l, 1))
                .filter(|h| !h.starts_with('#'))
                .unwrap_or("")
                .to_string(),
            groups: 0,
            resources: 0,
            requests: 0,
            responses: 0,
            data_structures: 0,
            parameters: 0,
            attributes: 0,
            headers: 0,
            bodies: 0,
            schemas: 0,
            resource_groups: 0,
            docs: 0,
        };
        let mut in_ds = false;
        for l in t.lines() {
            let tr = l.trim();
            if let Some(h2) = heading(tr, 2) {
                in_ds = h2 == "Data Structures";
                if h2.starts_with("Group") {
                    r.groups += 1;
                } else if !in_ds && h2.contains('[') && h2.contains(']') {
                    r.resource_groups += 1;
                }
                continue;
            }
            if let Some(h3) = heading(tr, 3) {
                if in_ds {
                    r.data_structures += 1;
                } else if h3.contains('[') && h3.contains(']') {
                    r.resources += 1;
                }
                continue;
            }
            if tr.starts_with("###") || tr.starts_with('#') {
                continue;
            }
            for (pat, f) in [
                ("+ Request", &mut r.requests),
                ("+ Response", &mut r.responses),
                ("+ Parameters", &mut r.parameters),
                ("+ Attributes", &mut r.attributes),
                ("+ Headers", &mut r.headers),
                ("+ Body", &mut r.bodies),
                ("+ Schema", &mut r.schemas),
            ] {
                if tr.starts_with(pat) {
                    *f += 1;
                    break;
                }
            }
            if tr.starts_with("<!--")
                || tr.starts_with("+ Note:")
                || tr.starts_with("+ Description:")
            {
                r.docs += 1;
            }
        }
        Some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"FORMAT: 1A\n# Shop API\n\n## Group Users\n\n### List users [GET /users]\n+ Response 200 (application/json)\n\n### Create user [POST /users]\n+ Request Create (application/json)\n    + Headers\n\n            X: 1\n+ Response 201\n\n## Data Structures\n\n### User\n+ Attributes\n    + id: 1 (number)\n";

    #[test]
    fn detects_apib() {
        assert!(detect(FIX));
        assert!(!detect(b"# plain markdown"));
    }

    #[test]
    fn parses_apib() {
        let a = Apib::parse(FIX).unwrap();
        assert_eq!(a.format, "1A");
        assert_eq!(a.name, "Shop API");
        assert_eq!(a.groups, 1);
        assert_eq!(a.resources, 2);
        assert_eq!(a.requests, 1);
        assert_eq!(a.responses, 2);
        assert_eq!(a.data_structures, 1);
        assert_eq!(a.headers, 1);
        assert_eq!(a.attributes, 1);
        assert!(Apib::parse(b"").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_xml_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_xml_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
