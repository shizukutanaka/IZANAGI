//! UPF `.upf` — Unified Power Format (IEEE 1801), Tcl-flavoured.
//!
//! Logical lines (`\` continuations, `#` comments) begin with power
//! verbs: `create_power_domain`, `create_supply_net`,
//! `create_supply_set`, `set_domain_supply_net`,
//! `create_power_switch`, `set_isolation`, `set_retention`,
//! `set_level_shifter`, `create_power_state_table`,
//! `add_power_state`, `load_upf`, `save_upf`.
//!
//! ```
//! let d = b"create_power_domain TOP\ncreate_supply_net VDD -domain TOP\n\
//! set_isolation iso -domain TOP\n";
//! let f = izanagi_kit::upf::parse(d).unwrap();
//! assert_eq!(f.power_domains, 1);
//! assert_eq!(f.supply_nets, 1);
//! assert_eq!(f.isolations, 1);
//! ```
//!
//! Reference: IEEE 1801 (UPF) command set; Synopsys / OpenROAD
//! documentation. Integer-only.

/// Parsed `.upf` command census.
#[derive(Debug, Clone, PartialEq)]
pub struct Upf {
    /// Distinct command verbs used, first-seen order.
    pub commands: Vec<String>,
    /// `create_power_domain` commands.
    pub power_domains: u32,
    /// `create_supply_net` + `create_supply_set` commands.
    pub supply_nets: u32,
    /// `set_domain_supply_net` / `connect_supply_net` commands.
    pub supply_links: u32,
    /// `create_power_switch` commands.
    pub power_switches: u32,
    /// `set_isolation` commands.
    pub isolations: u32,
    /// `set_retention` commands.
    pub retentions: u32,
    /// `set_level_shifter` commands.
    pub level_shifters: u32,
    /// `create_power_state_table` + `add_power_state` commands.
    pub power_states: u32,
    /// `load_upf`/`save_upf` commands.
    pub upf_io: u32,
    /// Total non-comment logical lines.
    pub lines: u32,
}

fn logical_lines(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for l in s.lines() {
        let l = l.trim_end();
        if let Some(head) = l.strip_suffix('\\') {
            cur.push_str(head);
            cur.push(' ');
        } else {
            cur.push_str(l);
            out.push(core::mem::take(&mut cur));
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Parse the command census. `None` when no known UPF verb appears.
pub fn parse(d: &[u8]) -> Option<Upf> {
    let s = core::str::from_utf8(d).ok()?;
    let mut f = Upf {
        commands: Vec::new(),
        power_domains: 0,
        supply_nets: 0,
        supply_links: 0,
        power_switches: 0,
        isolations: 0,
        retentions: 0,
        level_shifters: 0,
        power_states: 0,
        upf_io: 0,
        lines: 0,
    };
    let mut known = false;
    for l in logical_lines(s) {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let cmd = l.split_whitespace().next().unwrap_or("");
        if cmd.starts_with('[') {
            continue;
        }
        let hit = match cmd {
            "create_power_domain" => {
                f.power_domains += 1;
                true
            }
            "create_supply_net" | "create_supply_set" => {
                f.supply_nets += 1;
                true
            }
            "set_domain_supply_net" | "connect_supply_net" => {
                f.supply_links += 1;
                true
            }
            "create_power_switch" => {
                f.power_switches += 1;
                true
            }
            "set_isolation" => {
                f.isolations += 1;
                true
            }
            "set_retention" => {
                f.retentions += 1;
                true
            }
            "set_level_shifter" => {
                f.level_shifters += 1;
                true
            }
            "create_power_state_table" | "add_power_state" | "create_pst" => {
                f.power_states += 1;
                true
            }
            "load_upf" | "save_upf" => {
                f.upf_io += 1;
                true
            }
            _ => false,
        };
        if !hit && !cmd.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
            continue;
        }
        f.lines += 1;
        known |= hit;
        if !f.commands.iter().any(|c| c == cmd) {
            f.commands.push(cmd.to_string());
        }
    }
    if !known {
        return None;
    }
    Some(f)
}

/// `true` when a known UPF verb appears at line start.
pub fn detect(d: &[u8]) -> bool {
    let s = match core::str::from_utf8(d) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with("create_power_domain")
            || l.starts_with("set_isolation")
            || l.starts_with("create_supply_net")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"create_power_domain TOP -elements {}\n\
        create_power_domain PD1\n\
        create_supply_net VDD -domain TOP\ncreate_supply_set SS -domain PD1\n\
        set_domain_supply_net PD1 -primary_power_net VDD\n\
        connect_supply_net VDD -ports a\ncreate_power_switch sw -domain PD1\n\
        set_isolation iso -domain PD1\nset_retention ret -domain PD1\n\
        set_level_shifter ls -domain PD1\ncreate_power_state_table t\n\
        add_power_state t -state {on}\nload_upf sub.upf\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.power_domains, 2);
        assert_eq!(f.supply_nets, 2);
        assert_eq!(f.supply_links, 2);
        assert_eq!(f.power_switches, 1);
        assert_eq!(f.isolations, 1);
        assert_eq!(f.retentions, 1);
        assert_eq!(f.level_shifters, 1);
        assert_eq!(f.power_states, 2);
        assert_eq!(f.upf_io, 1);
        assert_eq!(f.lines, 13);
    }

    #[test]
    fn continuation_and_comment() {
        let d = b"# c\ncreate_power_domain TOP \\\n  -elements {}\n";
        let f = parse(d).unwrap();
        assert_eq!(f.power_domains, 1);
        assert_eq!(f.lines, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"create_clock -period 10\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"set_false_path x"));
    }
}
