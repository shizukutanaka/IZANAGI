//! SPECCTRA `.dsn` — the design/session file exchanged between PCB
//! layout tools and autorouters. An S-expression document rooted at
//! `(pcb <name> (parser …) (resolution …) (structure/layer …)
//! (placement …) (library …) (network/net …) (wiring …) )`.
//!
//! `parse` requires the `(pcb` root and counts the standard
//! sub-sections.
//!
//! ```
//! let f = b"(pcb board1 (parser (host_cad \"kicad\")) (resolution mm 10) (layer F.Cu (type signal)) (net GND) (wiring))";
//! let d = izanagi_kit::dsn::parse(f).unwrap();
//! assert_eq!(d.name, "board1");
//! assert_eq!(d.nets, 1);
//! assert!(d.layers >= 1);
//! ```

/// Parsed DSN summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Dsn {
    /// Board name, the atom right after `(pcb`.
    pub name: String,
    /// `(layer …)` declarations.
    pub layers: usize,
    /// `(net …)` declarations inside `network`.
    pub nets: usize,
    /// True when `(wiring …)` exists (routed session output).
    pub has_wiring: bool,
    /// True when `(placement …)` exists.
    pub has_placement: bool,
}

fn count(s: &str, needle: &str) -> usize {
    s.matches(needle).count()
}

/// Parse a `.dsn`; `None` without a `(pcb` root.
pub fn parse(d: &[u8]) -> Option<Dsn> {
    let s = std::str::from_utf8(d).ok()?;
    let t = s.trim_start();
    if !t.starts_with("(pcb") {
        return None;
    }
    let rest = t[4..].trim_start();
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let name = rest[..end].to_string();
    Some(Dsn {
        name,
        layers: count(s, "(layer"),
        nets: count(s, "(net"),
        has_wiring: s.contains("(wiring"),
        has_placement: s.contains("(placement"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"(pcb b (layer a) (layer b) (net N1) (net N2) (placement) (wiring))";
        let d = parse(f).unwrap();
        assert_eq!(d.name, "b");
        assert_eq!(d.layers, 2);
        assert_eq!(d.nets, 2);
        assert!(d.has_wiring && d.has_placement);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"(dsn x)").is_none());
        assert!(parse(b"(pcb").is_none());
        assert!(parse(b"(pcb )").is_none());
    }
}
