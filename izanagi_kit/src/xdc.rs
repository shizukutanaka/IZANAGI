//! Xilinx Design Constraints (`.xdc`, Tcl-like): `set_property`,
//! `create_clock`, `create_generated_clock`, `set_input_delay`,
//! `set_output_delay`, `set_false_path`, `set_max_delay`,
//! `set_multicycle_path` … with `get_ports`/`get_pins`/`get_cells`/
//! `get_clocks`/`get_nets` object lists; `#` comments.
//!
//! ```
//! let d = b"# constraints\nset_property PACKAGE_PIN W5 [get_ports clk]\ncreate_clock -period 10.000 [get_ports clk]\nset_false_path -from [get_ports rst]\n";
//! let p = izanagi_kit::xdc::parse(d).unwrap();
//! assert_eq!(p.set_property, 1);
//! assert_eq!(p.clock_commands, 1);
//! assert!(izanagi_kit::xdc::detect(d));
//! ```

/// Census of an XDC constraints file.
#[derive(Debug, Clone, PartialEq)]
pub struct Xdc {
    /// `set_property` commands.
    pub set_property: u32,
    /// `create_clock`/`create_generated_clock`/`create_clock_group`.
    pub clock_commands: u32,
    /// `set_input_delay`/`set_output_delay`.
    pub io_delay_commands: u32,
    /// `set_false_path`/`set_max_delay`/`set_multicycle_path`/`set_clock_uncertainty`.
    pub timing_exceptions: u32,
    /// `set_load`/`set_driving_cell`/`set_operating_conditions` and other `set_*`.
    pub other_set_commands: u32,
    /// `get_ports`/`get_pins`/`get_cells`/`get_clocks`/`get_nets` object refs.
    pub object_refs: u32,
    /// `PACKAGE_PIN` properties assigned.
    pub package_pins: u32,
    /// `IOSTANDARD` properties assigned.
    pub iostandards: u32,
    /// `#` comment lines.
    pub comment_lines: u32,
    /// Unbalanced `[`/`]`/`{`/`}` brackets anywhere.
    pub unbalanced: bool,
}

/// `true` when a recognized XDC command token appears.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8
        && (b.windows(12).any(|w| w == *b"set_property")
            || b.windows(12).any(|w| w == *b"create_clock")
            || b.windows(15).any(|w| w == *b"set_input_delay")
            || b.windows(14).any(|w| w == *b"set_false_path"))
}

/// Census; `None` without XDC commands.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xdc> {
    if !detect(b) {
        return None;
    }
    let mut x = Xdc {
        set_property: 0,
        clock_commands: 0,
        io_delay_commands: 0,
        timing_exceptions: 0,
        other_set_commands: 0,
        object_refs: 0,
        package_pins: 0,
        iostandards: 0,
        comment_lines: 0,
        unbalanced: false,
    };
    let mut depth = 0i32;
    let mut saw_unbalance = false;
    for &c in b {
        match c {
            b'[' | b'{' => depth += 1,
            b']' | b'}' => {
                depth -= 1;
                if depth < 0 {
                    saw_unbalance = true;
                    depth = 0;
                }
            }
            _ => {}
        }
    }
    x.unbalanced = saw_unbalance || depth != 0;
    for line in b.split(|c| *c == b'\n') {
        let t = {
            let mut k = 0;
            while k < line.len() && line[k].is_ascii_whitespace() {
                k += 1;
            }
            &line[k..]
        };
        if t.starts_with(b"#") {
            x.comment_lines += 1;
            continue;
        }
        if t.is_empty() {
            continue;
        }
        let pats: [&[u8]; 5] = [
            b"get_ports",
            b"get_pins",
            b"get_cells",
            b"get_clocks",
            b"get_nets",
        ];
        for pat in pats {
            x.object_refs += t.windows(pat.len()).filter(|w| *w == pat).count() as u32;
        }
        let mut w_end = 0;
        while w_end < t.len() && (t[w_end].is_ascii_alphanumeric() || t[w_end] == b'_') {
            w_end += 1;
        }
        let cmd = &t[..w_end];
        match cmd {
            b"set_property" => {
                x.set_property += 1;
                if t.windows(11).any(|w| w == *b"PACKAGE_PIN") {
                    x.package_pins += 1;
                }
                if t.windows(10).any(|w| w == *b"IOSTANDARD") {
                    x.iostandards += 1;
                }
            }
            b"create_clock" | b"create_generated_clock" | b"create_clock_group" => {
                x.clock_commands += 1;
            }
            b"set_input_delay" | b"set_output_delay" => x.io_delay_commands += 1,
            b"set_false_path"
            | b"set_max_delay"
            | b"set_multicycle_path"
            | b"set_clock_uncertainty" => x.timing_exceptions += 1,
            b"set_load"
            | b"set_driving_cell"
            | b"set_operating_conditions"
            | b"set_units"
            | b"set_switching_activity" => x.other_set_commands += 1,
            _ => {}
        }
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        b"# top.xdc\nset_property PACKAGE_PIN W5 [get_ports clk]\nset_property IOSTANDARD LVCMOS33 [get_ports clk]\ncreate_clock -period 10.000 -name sysclk [get_ports clk]\nset_false_path -from [get_ports rst_n] -to [get_cells u_reg]\nset_input_delay 2.0 -clock sysclk [get_ports din]\n"
            .to_vec()
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"random text"));
        assert!(!detect(b"set"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.set_property, 2);
        assert_eq!(p.clock_commands, 1);
        assert_eq!(p.io_delay_commands, 1);
        assert_eq!(p.timing_exceptions, 1);
        assert_eq!(p.package_pins, 1);
        assert_eq!(p.iostandards, 1);
        assert_eq!(p.object_refs, 6);
        assert_eq!(p.comment_lines, 1);
        assert!(!p.unbalanced);
    }

    #[test]
    fn unbalanced_bracket() {
        assert!(
            parse(b"set_property PACKAGE_PIN W5 [get_ports clk")
                .unwrap()
                .unbalanced
        );
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"nothing").is_none());
    }
}
