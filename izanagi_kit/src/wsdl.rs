//! WSDL 1.1 / 2.0 service description — `<wsdl:definitions>` (1.1) or
//! `<description xmlns="…/wsdl">` (2.0) roots, `<service>`/`<portType>`/
//! `<interface>`, `<operation>`, `<message>`/`element` declarations,
//! `<binding>` with `soap:`/`soap12:` transport URIs.
//!
//! ```
//! let d = b"<wsdl:definitions xmlns:wsdl=\"http://schemas\x2exmlsoap\x2eorg/wsdl/\" xmlns:soap=\"http://schemas\x2exmlsoap\x2eorg/wsdl/soap/\">\n<wsdl:types/><wsdl:message name=\"Req\"/><wsdl:portType name=\"P\"><wsdl:operation name=\"op\"/></wsdl:portType>\n<wsdl:binding name=\"B\"/><wsdl:service name=\"S\"><wsdl:port/></wsdl:service>\n</wsdl:definitions>";
//! let w = izanagi_kit::wsdl::parse(d).unwrap();
//! assert_eq!(w.major_version, 1);
//! assert_eq!(w.services, 1);
//! assert_eq!(w.port_types, 1);
//! assert_eq!(w.operations, 1);
//! assert!(w.soap);
//! assert!(izanagi_kit::wsdl::detect(d));
//! ```

/// A censused WSDL document.
#[derive(Debug)]
pub struct Wsdl {
    /// `1` for `<wsdl:definitions>`/`wsdl namespace`, `2` for WSDL 2.0
    /// `<description>` + `www.w3.org/ns/wsdl`.
    pub major_version: u8,
    /// `<service` element count.
    pub services: u32,
    /// `<portType` (1.1) + `<interface` (2.0) count.
    pub port_types: u32,
    /// `<operation` count.
    pub operations: u32,
    /// `<message` (1.1) count.
    pub messages: u32,
    /// `<binding` count.
    pub bindings: u32,
    /// `<port`/`endpoint` count.
    pub endpoints: u32,
    /// `soap:`/`soap12:` binding markers present.
    pub soap: bool,
    /// `targetNamespace` attribute, when present.
    pub target_namespace: Option<String>,
}

fn count(s: &str, pat: &str) -> u32 {
    u32::try_from(s.matches(pat).count()).unwrap_or(u32::MAX)
}

/// Both `<wsdl:x` and `<x` under a `wsdl`/`wsdl2` namespace count; the
/// element-level pattern `tag` + space/`>`/`/` catches either spelling.
fn elem_count(s: &str, tag: &str) -> u32 {
    // unprefixed open tags (`<service ` / `<service>` / `<service/>`)
    let mut n = count(s, &format!("<{tag} "))
        + count(s, &format!("<{tag}>"))
        + count(s, &format!("<{tag}/"));
    // prefixed tags (`<wsdl:service…`): `:{tag}` occurrences that are not
    // preceded by `</` (closing tag) and are followed by a boundary char
    let pat = format!(":{tag}");
    for (i, _) in s.match_indices(&pat) {
        let j = i + pat.len();
        let after_ok = s
            .as_bytes()
            .get(j)
            .is_some_and(|c| matches!(c, b' ' | b'>' | b'/' | b'\t' | b'\r' | b'\n'));
        let is_close = s[..i]
            .rfind('<')
            .is_some_and(|k| s.as_bytes().get(k + 1) == Some(&b'/'));
        if after_ok && !is_close {
            n += 1;
        }
    }
    n
}

fn attr<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let i = s.find(&format!("{name}="))? + name.len() + 1;
    let rest = &s[i..];
    let q = rest.as_bytes().first().copied()?;
    if q != b'"' && q != b'\'' {
        return None;
    }
    let rest = &rest[1..];
    let j = rest.find(q as char)?;
    Some(&rest[..j])
}

/// `wsdl` namespace or `<wsdl:`-prefixed root.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains(":definitions") && (s.contains("wsdl") || s.contains("schemas.xmlsoap.org/wsdl"))
        || s.contains("/ns/wsdl")
}

/// Parses the document; `None` without a WSDL marker.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Wsdl> {
    let s = core::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let major_version = if s.contains("/ns/wsdl") { 2 } else { 1 };
    Some(Wsdl {
        major_version,
        services: elem_count(s, "service"),
        port_types: elem_count(s, "portType") + elem_count(s, "interface"),
        operations: elem_count(s, "operation"),
        messages: elem_count(s, "message"),
        bindings: elem_count(s, "binding"),
        endpoints: elem_count(s, "port") + elem_count(s, "endpoint"),
        soap: s.contains("soap:") || s.contains("soap12:") || s.contains("/soap"),
        target_namespace: attr(s, "targetNamespace").map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: &[u8] = b"<wsdl:definitions xmlns:wsdl=\"http://schemas\x2exmlsoap\x2eorg/wsdl/\" targetNamespace=\"urn:t\"><wsdl:message name=\"Req\"/><wsdl:portType name=\"P\"><wsdl:operation name=\"op\"/></wsdl:portType><wsdl:binding name=\"B\"><soap:binding/></wsdl:binding><wsdl:service name=\"S\"><wsdl:port/></wsdl:service></wsdl:definitions>";

    #[test]
    fn detect_works() {
        assert!(detect(W));
        assert!(!detect(b"<html/>"));
    }

    #[test]
    fn parses() {
        let w = parse(W).unwrap();
        assert_eq!(w.major_version, 1);
        assert_eq!(w.messages, 1);
        assert_eq!(w.port_types, 1);
        assert_eq!(w.operations, 1);
        assert_eq!(w.bindings, 2); // wsdl:binding + soap:binding
        assert_eq!(w.services, 1);
        assert_eq!(w.endpoints, 1);
        assert!(w.soap);
        assert_eq!(w.target_namespace.as_deref(), Some("urn:t"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<x/>").is_none());
    }
}
