//! SDC `.sdc` — Synopsys Design Constraints (Tcl-flavoured).
//!
//! Logical lines (newline, with `\` continuations) each begin with
//! a Tcl command: `create_clock`, `create_generated_clock`,
//! `set_input_delay`, `set_output_delay`, `set_false_path`,
//! `set_max_delay`/`set_min_delay`, `set_clock_groups`,
//! `set_multicycle_path`, `set_clock_uncertainty`, `set_load`, …
//! `#` starts a comment.
//!
//! ```
//! let d = b"create_clock -period 10 -name clk [get_ports clk]\n\
//! set_input_delay 1 -clock clk [all_inputs]\n\
//! set_false_path -from [get_ports rst]\n";
//! let f = izanagi_kit::sdc::parse(d).unwrap();
//! assert_eq!(f.clocks, 1);
//! assert_eq!(f.input_delays, 1);
//! assert_eq!(f.false_paths, 1);
//! ```
//!
//! Reference: Synopsys Design Constraints (SDC) format; OpenSTA /
//! OpenROAD documentation. Integer-only.

/// Parsed `.sdc` command census.
#[derive(Debug, Clone, PartialEq)]
pub struct Sdc {
    /// Distinct Tcl command verbs used, first-seen order.
    pub commands: Vec<String>,
    /// `create_clock` commands.
    pub clocks: u32,
    /// `create_generated_clock` commands.
    pub generated_clocks: u32,
    /// `set_input_delay` commands.
    pub input_delays: u32,
    /// `set_output_delay` commands.
    pub output_delays: u32,
    /// `set_false_path` commands.
    pub false_paths: u32,
    /// `set_max_delay` + `set_min_delay` commands.
    pub delay_bounds: u32,
    /// `set_clock_groups` commands.
    pub clock_groups: u32,
    /// `set_multicycle_path` commands.
    pub multicycle_paths: u32,
    /// Total non-comment logical lines.
    pub lines: u32,
}

fn is_cmd(w: &str) -> bool {
    !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}

/// Logical lines: join `\`-continued physical lines.
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

/// Parse the command census. `None` when no known SDC verb appears.
pub fn parse(d: &[u8]) -> Option<Sdc> {
    let s = core::str::from_utf8(d).ok()?;
    let mut f = Sdc {
        commands: Vec::new(),
        clocks: 0,
        generated_clocks: 0,
        input_delays: 0,
        output_delays: 0,
        false_paths: 0,
        delay_bounds: 0,
        clock_groups: 0,
        multicycle_paths: 0,
        lines: 0,
    };
    let mut known = false;
    for l in logical_lines(s) {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        // strip a leading Tcl comment or list bracket noise
        let cmd = l.split_whitespace().next().unwrap_or("");
        if cmd.starts_with('[') || !is_cmd(cmd) {
            continue;
        }
        f.lines += 1;
        match cmd {
            "create_clock" => {
                f.clocks += 1;
                known = true;
            }
            "create_generated_clock" => {
                f.generated_clocks += 1;
                known = true;
            }
            "set_input_delay" => {
                f.input_delays += 1;
                known = true;
            }
            "set_output_delay" => {
                f.output_delays += 1;
                known = true;
            }
            "set_false_path"
            | "set_clock_groups"
            | "set_multicycle_path"
            | "set_max_delay"
            | "set_min_delay"
            | "set_clock_uncertainty"
            | "set_clock_latency"
            | "set_clock_transition"
            | "set_driving_cell"
            | "set_load"
            | "set_max_fanout"
            | "set_max_transition"
            | "set_case_analysis"
            | "set_disable_timing"
            | "group_path" => {
                match cmd {
                    "set_false_path" => f.false_paths += 1,
                    "set_clock_groups" => f.clock_groups += 1,
                    "set_multicycle_path" => f.multicycle_paths += 1,
                    "set_max_delay" | "set_min_delay" => f.delay_bounds += 1,
                    _ => {}
                }
                known = true;
            }
            _ => {}
        }
        if !f.commands.iter().any(|c| c == cmd) {
            f.commands.push(cmd.to_string());
        }
    }
    if !known {
        return None;
    }
    Some(f)
}

/// `true` when a known SDC verb appears at line start.
pub fn detect(d: &[u8]) -> bool {
    let s = match core::str::from_utf8(d) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with("create_clock")
            || l.starts_with("set_input_delay")
            || l.starts_with("set_output_delay")
            || l.starts_with("set_false_path")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"# cmt\ncreate_clock -period 10 -name clk [get_ports clk]\n\
        create_generated_clock -name gclk -source clk -divide_by 2 [get_pins ff/Q]\n\
        set_input_delay 1 -clock clk [all_inputs]\n\
        set_output_delay 2 -clock clk [all_outputs]\n\
        set_false_path -from [get_ports rst]\nset_max_delay 5 -to [all_outputs]\n\
        set_clock_groups -async -group {clk} -group {gclk}\n\
        set_multicycle_path 2 -from [get_pins a]\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.clocks, 1);
        assert_eq!(f.generated_clocks, 1);
        assert_eq!(f.input_delays, 1);
        assert_eq!(f.output_delays, 1);
        assert_eq!(f.false_paths, 1);
        assert_eq!(f.delay_bounds, 1);
        assert_eq!(f.clock_groups, 1);
        assert_eq!(f.multicycle_paths, 1);
        assert!(f.commands.contains(&"create_clock".to_string()));
        assert_eq!(f.lines, 8);
    }

    #[test]
    fn continuations_and_comments() {
        let d = b"create_clock -period 10 \\\n  -name clk [get_ports clk]\n# ignored\n";
        let f = parse(d).unwrap();
        assert_eq!(f.clocks, 1);
        assert_eq!(f.lines, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"puts hello\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"proc foo {} {}"));
    }
}
