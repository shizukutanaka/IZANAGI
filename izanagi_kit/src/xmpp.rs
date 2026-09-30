//! XMPP stanzas (RFC 6120): top-level `<message>`, `<presence>`, and
//! `<iq>` elements with `to`/`from`/`id`/`type` attributes and a
//! `<body>` payload for messages. Self-closing stanzas are accepted.
//!
//! ```
//! let s = b"<message to='a@b' from='c@d' type='chat'><body>hi</body></message>";
//! let x = izanagi_kit::xmpp::parse(s).unwrap();
//! assert_eq!(x.kind, izanagi_kit::xmpp::Kind::Message);
//! assert_eq!(x.to.as_deref(), Some("a@b"));
//! assert_eq!(x.body.as_deref(), Some("hi"));
//! ```

use std::string::String;

/// Stanza kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `<message>` — instant messaging.
    Message,
    /// `<presence>` — availability broadcast.
    Presence,
    /// `<iq>` — info/query request-response.
    Iq,
}

/// A parsed XMPP stanza.
#[derive(Clone, Debug)]
pub struct Xmpp {
    /// Stanza kind.
    pub kind: Kind,
    /// `to` JID attribute.
    pub to: Option<String>,
    /// `from` JID attribute.
    pub from: Option<String>,
    /// `id` attribute.
    pub id: Option<String>,
    /// `type` attribute (`chat`, `available`, `get`, `set`, …).
    pub stanza_type: Option<String>,
    /// First `<body>` text for messages.
    pub body: Option<String>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    for q in ['"', '\''] {
        let pat = format!("{key}={q}");
        if let Some(i) = tag.find(&pat) {
            let rest = &tag[i + pat.len()..];
            if let Some(e) = rest.find(q) {
                return Some(rest[..e].to_string());
            }
        }
    }
    None
}

fn text_between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    let j = s[i..].find(close)? + i;
    Some(&s[i..j])
}

/// Parse a single XMPP stanza; `None` when it isn't a `message`,
/// `presence`, or `iq` element.
pub fn parse(d: &[u8]) -> Option<Xmpp> {
    let s = std::str::from_utf8(d).ok()?.trim();
    let lt = s.find('<')?;
    let gt = s[lt..].find('>')? + lt;
    let head = &s[lt + 1..gt];
    let head = head.trim_start_matches('/'); // never a close tag
    let name_end = head
        .find(|c: char| c.is_whitespace() || c == '/')
        .unwrap_or(head.len());
    let kind = match &head[..name_end] {
        "message" => Kind::Message,
        "presence" => Kind::Presence,
        "iq" => Kind::Iq,
        _ => return None,
    };
    let name = &head[..name_end];
    let self_closing = head.ends_with('/');
    if !self_closing {
        // require the matching close tag after the open tag
        let close = format!("</{name}>");
        if !s[gt..].contains(&close) {
            return None;
        }
    }
    Some(Xmpp {
        kind,
        to: attr(head, "to"),
        from: attr(head, "from"),
        id: attr(head, "id"),
        stanza_type: attr(head, "type"),
        body: text_between(s, "<body>", "</body>").map(|b| b.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message() {
        let x = parse(
            b"<message id='1' to='romeo@ex' from='jul@ex' type='chat'><body>Art thou?</body></message>",
        )
        .unwrap();
        assert_eq!(x.kind, Kind::Message);
        assert_eq!(x.id.as_deref(), Some("1"));
        assert_eq!(x.stanza_type.as_deref(), Some("chat"));
        assert_eq!(x.body.as_deref(), Some("Art thou?"));
    }

    #[test]
    fn others() {
        let x = parse(b"<presence from='a@b'/>").unwrap();
        assert_eq!(x.kind, Kind::Presence);
        assert_eq!(x.from.as_deref(), Some("a@b"));
        assert_eq!(x.body, None);

        let x = parse(b"<iq type='get' id='q1'><query xmlns='jabber:iq:roster'/></iq>").unwrap();
        assert_eq!(x.kind, Kind::Iq);
        assert_eq!(x.id.as_deref(), Some("q1"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<stream:stream>").is_none());
        assert!(parse(b"<message>").is_none()); // unclosed
        assert!(parse(b"plain text").is_none());
    }

    #[test]
    fn rejects_misordered_tag_delimiters() {
        assert!(parse(b"><").is_none());
        assert!(parse(b"x><message>").is_none());
    }
}
