//! host-meta — RFC 6415 `.well-known/host-meta`: an XRD document
//! `<XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0">` with
//! optional `<Host>`, `<Expires>` and `<Link rel=… href=|template=…>`
//! entries. Namespace prefixes on element names are tolerated.
//!
//! ```
//! let s = "<XRD xmlns=\"http://docs.oasis-open.org/ns/xri/xrd-1.0\">\
//!          <Host>e\x2ex</Host>\
//!          <Link rel=\"lrdd\" template=\"https://e\x2ex/{uri}\"/></XRD>";
//! let d = izanagi_kit::hostmeta::parse(s).unwrap();
//! assert_eq!(d.host.as_deref(), Some("e\x2ex"));
//! assert_eq!(d.links[0].rel, "lrdd");
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// One `<Link>` entry from host-meta.
#[derive(Clone, Debug)]
pub struct Link {
    /// `rel` attribute (e.g. `lrdd`, `author`, `hub`).
    pub rel: String,
    /// `href` when the link is a fixed URL.
    pub href: Option<String>,
    /// `template` when the link is a URI template (`{uri}`).
    pub template: Option<String>,
    /// `type` (media type) when present.
    pub media_type: Option<String>,
}

/// A parsed host-meta document.
#[derive(Clone, Debug)]
pub struct Hostmeta {
    /// `<Host>` text when present.
    pub host: Option<String>,
    /// `<Expires>` text when present.
    pub expires: Option<String>,
    /// `<Link>` entries in document order.
    pub links: Vec<Link>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    for q in ['"', '\''] {
        let pat = format!("{key}={q}");
        if let Some(at) = tag.find(&pat) {
            let at = at + pat.len();
            if let Some(end) = tag[at..].find(q) {
                return Some(tag[at..at + end].to_string());
            }
        }
    }
    None
}

/// Find `local` element open tags; each entry is the raw tag text and
/// the absolute offset just past its `>`.
fn open_tags<'a>(src: &'a str, local: &str) -> Vec<(&'a str, usize)> {
    let mut out = Vec::new();
    let mut rest = src;
    let mut base = 0usize;
    while let Some(a) = rest.find('<') {
        let gt = match rest[a..].find('>') {
            Some(g) => a + g,
            None => break,
        };
        let tag = &rest[a..=gt];
        if !tag.starts_with("</") && !tag.starts_with("<!") && !tag.starts_with("<?") {
            let inner = tag[1..tag.len() - 1].trim_end_matches('/');
            let name_end = inner
                .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
                .unwrap_or(inner.len());
            let name = &inner[..name_end];
            let local_name = name.rsplit_once(':').map(|(_, l)| l).unwrap_or(name);
            if local_name == local {
                out.push((tag, base + gt + 1));
            }
        }
        base += gt + 1;
        rest = &rest[gt + 1..];
    }
    out
}

fn close_tag<'a>(src: &'a str, from: usize, local: &str) -> Option<&'a str> {
    let body = &src[from..];
    for (i, _) in body.match_indices("</") {
        let tail = &body[i..];
        let gt = tail.find('>')?;
        let name = tail[2..gt].trim();
        let name = name
            .find(|c: char| c.is_whitespace())
            .map_or(name, |e| &name[..e]);
        let local_name = name.rsplit_once(':').map(|(_, l)| l).unwrap_or(name);
        if local_name == local {
            return Some(&body[..i]);
        }
    }
    None
}

/// Parse an XRD host-meta document; `None` without an `<XRD>` root
/// whose namespace is `…/xri/xrd-1.0` (or an unprefixed `<XRD>` when
/// no xmlns is given — lenient for minimal files).
pub fn parse(s: &str) -> Option<Hostmeta> {
    let ids = open_tags(s, "XRD");
    let (root, _) = ids.first()?;
    if let Some(ns) = attr(root, "xmlns") {
        // `xmlns` may carry a prefix-less XRD namespace only.
        if !ns.contains("xrd") {
            return None;
        }
    }
    // `xmlns:xrd`-style prefixed namespaces are also accepted.
    let mut host = None;
    for (_, at) in open_tags(s, "Host") {
        if let Some(body) = close_tag(s, at, "Host") {
            host = Some(body.trim().to_string());
        }
    }
    let mut expires = None;
    for (_, at) in open_tags(s, "Expires") {
        if let Some(body) = close_tag(s, at, "Expires") {
            expires = Some(body.trim().to_string());
        }
    }
    let links = open_tags(s, "Link")
        .iter()
        .filter_map(|(t, _)| {
            Some(Link {
                rel: attr(t, "rel")?,
                href: attr(t, "href"),
                template: attr(t, "template"),
                media_type: attr(t, "type"),
            })
        })
        .collect();
    Some(Hostmeta {
        host,
        expires,
        links,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let s = "<XRD xmlns=\"http://docs.oasis-open.org/ns/xri/xrd-1.0\">\
                 <Host>e\x2ex</Host><Expires>2030</Expires>\
                 <Link rel=\"lrdd\" template=\"https://e\x2ex/{uri}\"/>\
                 <Link rel=\"author\" href=\"https://e\x2ex/a\" type=\"text/html\"/></XRD>";
        let d = parse(s).unwrap();
        assert_eq!(d.host.as_deref(), Some("e\x2ex"));
        assert_eq!(d.expires.as_deref(), Some("2030"));
        assert_eq!(d.links.len(), 2);
        assert_eq!(d.links[0].template.as_deref(), Some("https://e\x2ex/{uri}"));
        assert_eq!(d.links[1].media_type.as_deref(), Some("text/html"));
    }

    #[test]
    fn prefixed() {
        let s = "<xrd:XRD xmlns:xrd=\"x\"><xrd:Link rel=\"r\" href=\"h\"/></xrd:XRD>";
        let d = parse(s).unwrap();
        assert_eq!(d.links[0].href.as_deref(), Some("h"));
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("<html></html>").is_none());
        assert!(parse("<XRD xmlns=\"urn:other\"></XRD>").is_none());
    }
}
