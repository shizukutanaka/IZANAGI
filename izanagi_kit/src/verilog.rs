//! Verilog `.v` — IEEE 1364 hardware description source.
//!
//! Census over comment-stripped lines: `` `timescale`` /
//! `` `define`` directives, `module`/`endmodule` pairs, `input` /
//! `output` / `inout` / `wire` / `reg` / `logic` declarations,
//! `always` blocks, `assign` statements and `parameter`s.
//!
//! ```
//! let d = b"`timescale 1ns/1ps\nmodule top(input clk, output reg q);\n\
//! assign x = 1;\nalways @(posedge clk) q <= x;\nendmodule\n";
//! let f = izanagi_kit::verilog::parse(d).unwrap();
//! assert_eq!(f.modules, vec!["top"]);
//! assert_eq!(f.assigns, 1);
//! assert_eq!(f.always_blocks, 1);
//! ```
//!
//! Reference: IEEE 1364 Verilog LRM; iverilog / yosys frontends.
//! Integer-only.

/// Parsed `.v` source census.
#[derive(Debug, Clone, PartialEq)]
pub struct Verilog {
    /// `module` names in file order (`macromodule` included).
    pub modules: Vec<String>,
    /// `endmodule` keywords seen.
    pub endmodules: u32,
    /// `` `timescale`` argument (e.g. `1ns/1ps`).
    pub timescale: Option<String>,
    /// `` `define`` macro names.
    pub defines: Vec<String>,
    /// `` `include`` directives.
    pub includes: u32,
    /// `input` declarations (lines starting with the keyword).
    pub inputs: u32,
    /// `output` declarations.
    pub outputs: u32,
    /// `inout` declarations.
    pub inouts: u32,
    /// `wire` declarations.
    pub wires: u32,
    /// `reg`/`logic` declarations.
    pub regs: u32,
    /// `parameter`/`localparam` declarations.
    pub parameters: u32,
    /// `always`/`always_ff`/`always_comb`/`always_latch` blocks.
    pub always_blocks: u32,
    /// `assign` statements.
    pub assigns: u32,
    /// `initial` blocks.
    pub initials: u32,
}

/// Strip `//` line comments and `/* … */` blocks; `\` directives
/// survive (they start lines). Returns per-line text.
fn strip_comments(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let mut block = false;
    let mut str_lit = false;
    while i < b.len() {
        if block {
            if i + 1 < b.len() && b[i] == b'*' && b[i + 1] == b'/' {
                block = false;
                i += 2;
            } else {
                if b[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
        } else if str_lit {
            match b[i] {
                b'"' => {
                    str_lit = false;
                    out.push('"');
                }
                _ => out.push(' '),
            }
            i += 1;
        } else if b[i] == b'"' {
            str_lit = true;
            out.push('"');
            i += 1;
        } else if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
            block = true;
            i += 2;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

/// Parse the source census. `None` without a `module`/`endmodule`
/// pair.
pub fn parse(d: &[u8]) -> Option<Verilog> {
    let s = core::str::from_utf8(d).ok()?;
    let clean = strip_comments(s);
    let mut f = Verilog {
        modules: Vec::new(),
        endmodules: 0,
        timescale: None,
        defines: Vec::new(),
        includes: 0,
        inputs: 0,
        outputs: 0,
        inouts: 0,
        wires: 0,
        regs: 0,
        parameters: 0,
        always_blocks: 0,
        assigns: 0,
        initials: 0,
    };
    // statements may share a line (`module a; input x; … endmodule`)
    for l in clean.split(['\n', ';']).map(str::trim) {
        if l.is_empty() {
            continue;
        }
        if let Some(dir) = l.strip_prefix('`') {
            let mut it = dir.split_whitespace();
            match it.next().unwrap_or("") {
                "timescale" => f.timescale = Some(it.next().unwrap_or("").to_string()),
                "define" => {
                    if let Some(n) = it.next() {
                        f.defines.push(n.split('(').next().unwrap_or(n).to_string());
                    }
                }
                "include" => f.includes += 1,
                _ => {}
            }
            continue;
        }
        let first = l.split_whitespace().next().unwrap_or("");
        match first {
            "module" | "macromodule" => {
                let rest = l[first.len()..].trim_start();
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                    .collect();
                if !name.is_empty() {
                    f.modules.push(name);
                }
            }
            "endmodule" => f.endmodules += 1,
            "input" => f.inputs += 1,
            "output" => f.outputs += 1,
            "inout" => f.inouts += 1,
            "wire" | "tri" | "tri0" | "tri1" | "wand" | "wor" => f.wires += 1,
            "reg" | "logic" | "integer" => f.regs += 1,
            "parameter" | "localparam" => f.parameters += 1,
            "always" | "always_ff" | "always_comb" | "always_latch" => f.always_blocks += 1,
            "assign" => f.assigns += 1,
            "initial" => f.initials += 1,
            _ => {}
        }
    }
    if f.modules.is_empty() || f.endmodules == 0 {
        return None;
    }
    Some(f)
}

/// `true` when `module`…`endmodule` structure is present.
pub fn detect(d: &[u8]) -> bool {
    let s = match core::str::from_utf8(d) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("module") && s.contains("endmodule")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"`timescale 1ns/1ps\n`define W 8\n`include \"d.v\"\n\
        // module fake;\nmodule top(input clk, output reg [7:0] q);\n\
        /* module hidden; */\nwire w; reg r; inout pad; parameter P = 1;\n\
        assign w = 1;\nalways @(posedge clk) q <= w;\ninitial r = 0;\nendmodule\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.modules, vec!["top"]);
        assert_eq!(f.endmodules, 1);
        assert_eq!(f.timescale.as_deref(), Some("1ns/1ps"));
        assert_eq!(f.defines, vec!["W"]);
        assert_eq!(f.includes, 1);
        assert_eq!(f.inputs, 0); // port-style inputs live in the module line
        assert_eq!(f.wires, 1);
        assert_eq!(f.regs, 1);
        assert_eq!(f.inouts, 1);
        assert_eq!(f.parameters, 1);
        assert_eq!(f.always_blocks, 1);
        assert_eq!(f.assigns, 1);
        assert_eq!(f.initials, 1);
    }

    #[test]
    fn ansi_header_ports_and_multiple_modules() {
        let d = b"module a; input x; output y; endmodule\nmodule b; endmodule\n";
        let f = parse(d).unwrap();
        assert_eq!(f.modules, vec!["a", "b"]);
        assert_eq!(f.endmodules, 2);
        assert_eq!(f.inputs, 1);
        assert_eq!(f.outputs, 1);
    }

    #[test]
    fn comments_do_not_create_modules() {
        let d = b"// module nope; endmodule\nmodule real; endmodule\n";
        let f = parse(d).unwrap();
        assert_eq!(f.modules, vec!["real"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"module x\n").is_none()); // never closed
        assert!(parse(b"endmodule\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"entity x is"));
    }
}
