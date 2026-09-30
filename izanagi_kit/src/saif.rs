//! SAIF `.saif` — Switching Activity Interchange Format.
//!
//! Lisp-like parenthesised records: `(SAIFILE` opens the document
//! with `(SAIF_VERSION "2.0")`, `(DIRECTION "backward")`,
//! `(DESIGN ...)`, `(DIVIDER /)`, then `(INSTANCE name …)` blocks
//! containing `(PORT sig (T0 d) (T1 d) (TC d) (TX d) …)` toggle
//! durations (integers).
//!
//! ```
//! let d = b"(SAIFILE\n(SAIF_VERSION \"2\x2e0\")\n(DESIGN \"top\")\n\
//! (INSTANCE top\n  (PORT clk (T0 5) (T1 5) (TC 1))))\n";
//! let f = izanagi_kit::saif::parse(d).unwrap();
//! assert_eq!(f.saif_version.as_deref(), Some("2\x2e0"));
//! assert_eq!(f.instances, 1);
//! assert_eq!(f.t1, 5);
//! ```
//!
//! Reference: SAIF 2.0 (IEEE P1801-aligned); Synopsys power
//! analysis documentation. Integer-only.

/// Parsed `.saif` document census.
#[derive(Debug, Clone, PartialEq)]
pub struct Saif {
    /// `SAIF_VERSION` string (e.g. `2.0`).
    pub saif_version: Option<String>,
    /// `(DESIGN "name")` value.
    pub design: Option<String>,
    /// `(INSTANCE name …)` blocks.
    pub instances: u32,
    /// `(PORT sig …)` records.
    pub ports: u32,
    /// Summed `(T0 n)` low durations.
    pub t0: u64,
    /// Summed `(T1 n)` high durations.
    pub t1: u64,
    /// Summed `(TC n)` toggle-cycle counts.
    pub tc: u64,
    /// Summed `(TX n)` unknown durations.
    pub tx: u64,
}

fn unq(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

fn tok_u64(s: &str) -> u64 {
    s.bytes()
        .take_while(|b| b.is_ascii_digit())
        .fold(0u64, |v, b| {
            v.saturating_mul(10).saturating_add((b - b'0') as u64)
        })
}

/// Parse the S-expression census. `None` without a `(SAIFILE` head.
pub fn parse(d: &[u8]) -> Option<Saif> {
    let s = core::str::from_utf8(d).ok()?;
    if !s.contains("(SAIFILE") {
        return None;
    }
    let mut f = Saif {
        saif_version: None,
        design: None,
        instances: 0,
        ports: 0,
        t0: 0,
        t1: 0,
        tc: 0,
        tx: 0,
    };
    let mut i = 0;
    while let Some(p) = s[i..].find('(') {
        let st = i + p + 1;
        let e = s[st..]
            .find(|c: char| c.is_ascii_whitespace() || c == ')' || c == '(')
            .map(|o| st + o)
            .unwrap_or(s.len());
        let kw = &s[st..e];
        match kw {
            "SAIF_VERSION" | "DESIGN" | "INSTANCE" | "PORT" | "T0" | "T1" | "TC" | "TX" => {
                // argument = text after keyword until ')' (INSTANCE/PORT) —
                // for scalars take the atom
                let body = &s[e..];
                let close = body.find(')').unwrap_or(body.len());
                let arg = body[..close].trim();
                match kw {
                    "SAIF_VERSION" => f.saif_version = Some(unq(arg)),
                    "DESIGN" => f.design = Some(unq(arg.split_whitespace().next().unwrap_or(""))),
                    "INSTANCE" => f.instances += 1,
                    "PORT" => f.ports += 1,
                    "T0" => f.t0 = f.t0.saturating_add(tok_u64(arg)),
                    "T1" => f.t1 = f.t1.saturating_add(tok_u64(arg)),
                    "TC" => f.tc = f.tc.saturating_add(tok_u64(arg)),
                    "TX" => f.tx = f.tx.saturating_add(tok_u64(arg)),
                    _ => {}
                }
            }
            _ => {}
        }
        i = st;
    }
    Some(f)
}

/// `true` when a `(SAIFILE` head is present.
pub fn detect(d: &[u8]) -> bool {
    d.windows(8).any(|w| w == b"(SAIFILE")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"(SAIFILE\n(SAIF_VERSION \"2\x2e0\")\n(DIRECTION \"backward\")\n\
        (DESIGN \"top\")\n(DIVIDER /)\n\
        (INSTANCE top\n (PORT clk (T0 5) (T1 5) (TC 2))\n (PORT d (T0 8) (TX 1))\n \
        (INSTANCE sub (PORT q (T1 3))))\n)\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.saif_version.as_deref(), Some("2\x2e0"));
        assert_eq!(f.design.as_deref(), Some("top"));
        assert_eq!(f.instances, 2);
        assert_eq!(f.ports, 3);
        assert_eq!(f.t0, 13);
        assert_eq!(f.t1, 8);
        assert_eq!(f.tc, 2);
        assert_eq!(f.tx, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"(list (of) things)").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"(saifile"));
    }
}
