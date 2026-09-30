//! SPEF `.spef` — Standard Parasitic Exchange Format (IEEE 1481).
//!
//! Header records are `*KEYWORD value` lines: `*SPEF`,
//! `*DESIGN_NAME`, `*DATE`, `*VENDOR`, `*PROGRAM`, `*VERSION`,
//! `*DESIGN_FLOW`, `*DIVIDER`, `*DELIMITER`, `*BUS_DELIMITER`,
//! `*UNIT`/`T_UNIT`/`F_UNIT`/`C_UNIT`. Body sections `*PORTS`,
//! `*CONN` …`*END` and `*D_NET` blocks carry parasitics.
//!
//! ```
//! let d = b"*SPEF \"IEEE 1481-1998\"\n*DESIGN_NAME \"top\"\n\
//! *DATE \"d\"\n*VENDOR \"v\"\n*PROGRAM \"p\"\n*VERSION \"1\"\n\
//! *DIVIDER /\n*DELIMITER :\n*BUS_DELIMITER [ ]\n*T_UNIT 1 NS\n\
//! *PORTS\n*1 I *C 0 0\n*END\n*D_NET n1 10\n*END\n";
//! let f = izanagi_kit::spef::parse(d).unwrap();
//! assert_eq!(f.design_name.as_deref(), Some("top"));
//! assert_eq!(f.d_nets, 1);
//! assert_eq!(f.ports, 1);
//! ```
//!
//! Reference: IEEE 1481 SPEF standard; OpenRCX / OpenSTA
//! documentation. Integer-only.

/// Parsed `.spef` header + section census.
#[derive(Debug, Clone, PartialEq)]
pub struct Spef {
    /// `*SPEF` standard string (e.g. `IEEE 1481-1998`).
    pub standard: String,
    /// `*DESIGN_NAME` (unquoted).
    pub design_name: Option<String>,
    /// `*DATE` (unquoted).
    pub date: Option<String>,
    /// `*VENDOR` (unquoted).
    pub vendor: Option<String>,
    /// `*PROGRAM` (unquoted).
    pub program: Option<String>,
    /// `*T_UNIT`/`C_UNIT`/`F_UNIT`/`R_UNIT` values seen.
    pub units: Vec<String>,
    /// `*PORTS` entries.
    pub ports: u32,
    /// `*D_NET` net blocks.
    pub d_nets: u32,
    /// `*CONN` connection records.
    pub conns: u32,
    /// `*CAP` capacitance records.
    pub caps: u32,
    /// `*RES` resistance records.
    pub res: u32,
    /// `*INDUC` inductance records.
    pub inducs: u32,
}

fn unq(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

/// Parse `*KEYWORD` records. `None` without a `*SPEF` header line.
pub fn parse(d: &[u8]) -> Option<Spef> {
    let s = core::str::from_utf8(d).ok()?;
    let mut f = Spef {
        standard: String::new(),
        design_name: None,
        date: None,
        vendor: None,
        program: None,
        units: Vec::new(),
        ports: 0,
        d_nets: 0,
        conns: 0,
        caps: 0,
        res: 0,
        inducs: 0,
    };
    let mut saw = false;
    let mut in_dnet = false;
    for l in s.lines().map(str::trim) {
        if l.is_empty() {
            continue;
        }
        if l == "*END" {
            in_dnet = false;
            continue;
        }
        if let Some(rest) = l.strip_prefix('*') {
            let mut it = rest.split_whitespace();
            let kw = it.next().unwrap_or("");
            let arg = rest[kw.len()..].trim();
            match kw {
                "SPEF" => {
                    f.standard = unq(arg);
                    saw = true;
                }
                "DESIGN_NAME" => f.design_name = Some(unq(arg)),
                "DATE" => f.date = Some(unq(arg)),
                "VENDOR" => f.vendor = Some(unq(arg)),
                "PROGRAM" => f.program = Some(unq(arg)),
                "T_UNIT" | "C_UNIT" | "F_UNIT" | "R_UNIT" => {
                    f.units.push([kw, " ", &unq(arg)].concat());
                }
                "D_NET" => {
                    f.d_nets += 1;
                    in_dnet = true;
                }
                "CONN" if in_dnet => f.conns += 1,
                "CAP" if in_dnet => f.caps += 1,
                "RES" if in_dnet => f.res += 1,
                "INDUC" if in_dnet => f.inducs += 1,
                _ => {}
            }
            continue;
        }
        // `*PORTS` body lines begin with `*N`/`*I`… — count `*`-less rows
        // inside the section tracked by a simple marker
        // (handled below by the `ports` counter when kw was PORTS)
        if l.starts_with('*') {
            continue;
        }
        // bare lines inside *PORTS look like `*1 I *C 0 0`; those begin
        // with `*` too — handled in the branch above; nothing else here
    }
    // second pass for `*PORTS` members (lines like `*1 I *C 0 0`)
    let mut in_ports = false;
    for l in s.lines().map(str::trim) {
        if l == "*PORTS" {
            in_ports = true;
            continue;
        }
        if l == "*END" {
            in_ports = false;
            continue;
        }
        if in_ports && l.starts_with('*') {
            f.ports += 1;
        }
    }
    if !saw {
        return None;
    }
    Some(f)
}

/// `true` when a `*SPEF` record is present.
pub fn detect(d: &[u8]) -> bool {
    d.windows(5).any(|w| w == b"*SPEF")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"*SPEF \"IEEE 1481-1998\"\n*DESIGN_NAME \"top\"\n\
        *DATE \"d\"\n*VENDOR \"v\"\n*PROGRAM \"p\"\n*VERSION \"1\"\n\
        *DIVIDER /\n*DELIMITER :\n*BUS_DELIMITER [ ]\n*T_UNIT 1 NS\n*C_UNIT 1 PF\n\
        *PORTS\n*1 I *C 0 0\n*2 O *C 1 1\n*END\n\
        *D_NET n1 10\n*CONN\n*I *1 I *C 0 0\n*CAP\n1 n1 5\n*RES\n1 n1 n2 7\n*END\n\
        *D_NET n2 4\n*INDUC\n1 n2 3\n*END\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.standard, "IEEE 1481-1998");
        assert_eq!(f.design_name.as_deref(), Some("top"));
        assert_eq!(f.vendor.as_deref(), Some("v"));
        assert_eq!(f.units, vec!["T_UNIT 1 NS", "C_UNIT 1 PF"]);
        assert_eq!(f.ports, 2);
        assert_eq!(f.d_nets, 2);
        assert_eq!(f.conns, 1);
        assert_eq!(f.caps, 1);
        assert_eq!(f.res, 1);
        assert_eq!(f.inducs, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"*PORTS\n*END\n").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"*spEF"));
    }
}
