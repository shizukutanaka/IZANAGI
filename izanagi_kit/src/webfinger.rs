//! WebFinger — RFC 7033 JRD (JSON Resource Descriptor) returned by
//! `/.well-known/webfinger`: `{subject, aliases[], properties{},
//! links[{rel, type, href, titles{}, properties{}}]}`.
//!
//! ```
//! let d = izanagi_kit::webfinger::parse(br#"{"subject":"acct:u@e.x",
//!   "aliases":["https://e.x/u"],
//!   "links":[{"rel":"http://webfinger.net/rel/profile-page","href":"https://e.x/u"}]}"#)
//!   .unwrap();
//! assert_eq!(d.subject.as_deref(), Some("acct:u@e\x2ex"));
//! assert_eq!(d.links[0].href.as_deref(), Some("https://e\x2ex/u"));
//! ```

use crate::json::{self, Json};
use std::string::{String, ToString};
use std::vec::Vec;

/// One `links[]` entry.
#[derive(Clone, Debug)]
pub struct Link {
    /// `rel` link relation (URI or registered name).
    pub rel: String,
    /// `href` when present (absent for property-only links).
    pub href: Option<String>,
    /// `type` media type when present.
    pub media_type: Option<String>,
    /// `titles` map (lang → title).
    pub titles: Vec<(String, String)>,
}

/// A parsed WebFinger/JRD document.
#[derive(Clone, Debug)]
pub struct Webfinger {
    /// `subject` URI (e.g. `acct:user@host`) when present.
    pub subject: Option<String>,
    /// `aliases` list.
    pub aliases: Vec<String>,
    /// `properties` map.
    pub properties: Vec<(String, String)>,
    /// `links` entries.
    pub links: Vec<Link>,
}

fn s(v: &Json) -> Option<&str> {
    match v {
        Json::Str(t) => Some(t.as_str()),
        _ => None,
    }
}
fn obj(v: &Json) -> Option<&std::collections::BTreeMap<String, Json>> {
    match v {
        Json::Obj(m) => Some(m),
        _ => None,
    }
}
fn arr(v: &Json) -> Option<&[Json]> {
    match v {
        Json::Arr(a) => Some(a.as_slice()),
        _ => None,
    }
}
fn str_map(v: Option<&Json>) -> Vec<(String, String)> {
    v.and_then(obj).map_or(Vec::new(), |m| {
        m.iter()
            .filter_map(|(k, v)| s(v).map(|v| (k.clone(), v.to_string())))
            .collect()
    })
}
fn str_arr(v: Option<&Json>) -> Vec<String> {
    v.and_then(arr).map_or(Vec::new(), |a| {
        a.iter().filter_map(s).map(ToString::to_string).collect()
    })
}

/// Parse a JRD/WebFinger document; `None` unless the object carries
/// at least a `subject` or a `links` array.
pub fn parse(d: &[u8]) -> Option<Webfinger> {
    let v = json::parse(d).ok()?;
    let m = obj(&v)?;
    let subject = m.get("subject").and_then(s).map(ToString::to_string);
    let links: Vec<Link> = m.get("links").and_then(arr).map_or(Vec::new(), |a| {
        a.iter()
            .filter_map(|e| {
                let e = obj(e)?;
                Some(Link {
                    rel: s(e.get("rel")?)?.to_string(),
                    href: e.get("href").and_then(s).map(ToString::to_string),
                    media_type: e.get("type").and_then(s).map(ToString::to_string),
                    titles: str_map(e.get("titles")),
                })
            })
            .collect()
    });
    if subject.is_none() && links.is_empty() {
        return None;
    }
    Some(Webfinger {
        subject,
        aliases: str_arr(m.get("aliases")),
        properties: str_map(m.get("properties")),
        links,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full() {
        let d = parse(
            br#"{"subject":"acct:u@e.x",
            "aliases":["a1","a2"],
            "properties":{"http://k":"v"},
            "links":[{"rel":"self","href":"h","type":"j","titles":{"en":"T"}}]}"#,
        )
        .unwrap();
        assert_eq!(d.aliases.len(), 2);
        assert_eq!(d.properties.len(), 1);
        assert_eq!(d.links[0].rel, "self");
        assert_eq!(d.links[0].titles, [("en".to_string(), "T".to_string())]);
    }

    #[test]
    fn links_only() {
        let d = parse(br#"{"links":[{"rel":"r"}]}"#).unwrap();
        assert!(d.subject.is_none());
        assert_eq!(d.links[0].rel, "r");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"{}").is_none()); // neither subject nor links
        assert!(parse(br#"{"aliases":["a"]}"#).is_none());
        assert!(parse(b"[]").is_none());
    }
}
