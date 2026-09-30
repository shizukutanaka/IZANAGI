//! VCD `.vcd` — Value Change Dump waveform log (IEEE 1364).
//!
//! Keyword sections `$date`/`$version`/`$timescale`/`$scope`/`$var`
//! …`$end`, closed by `$enddefinitions`, then `#<time>` markers and
//! `0x`/`1x`/`b…`/`r…` value-change rows.
//!
//! ```
//! let d = b"$timescale 1ns $end\n$scope module top $end\n\
//! $var wire 1 ! clk $end\n$upscope $end\n$enddefinitions $end\n\
//! #0\n0!\n#5\n1!\n";
//! let f = izanagi_kit::vcd::parse(d).unwrap();
//! assert_eq!(f.timescale.as_deref(), Some("1ns"));
//! assert_eq!(f.vars, 1);
//! assert_eq!(f.timestamps, 2);
//! assert_eq!(f.value_changes, 2);
//! ```
//!
//! Reference: IEEE 1364 (Verilog LRM) VCD section; gtkwave /
//! vcd parsers. Integer-only.

/// Parsed `.vcd` header + event census.
#[derive(Debug, Clone, PartialEq)]
pub struct Vcd {
    /// `$timescale` argument (e.g. `1ns`), when present.
    pub timescale: Option<String>,
    /// `$scope` blocks.
    pub scopes: u32,
    /// `$var` declarations.
    pub vars: u32,
    /// `$enddefinitions` seen (header closed).
    pub definitions_closed: bool,
    /// `#<time>` markers.
    pub timestamps: u32,
    /// Value-change rows (`0!`/`1!`/`x!`/`z!`/`b…`/`r…`).
    pub value_changes: u32,
    /// `$comment` blocks.
    pub comments: u32,
}

/// Parse the keyword sections and count event rows. `None` when no
/// `$`-keyword structure is present.
pub fn parse(d: &[u8]) -> Option<Vcd> {
    let s = core::str::from_utf8(d).ok()?;
    let mut f = Vcd {
        timescale: None,
        scopes: 0,
        vars: 0,
        definitions_closed: false,
        timestamps: 0,
        value_changes: 0,
        comments: 0,
    };
    let mut saw_kw = false;
    let mut in_data = false;
    for l in s.lines().map(str::trim) {
        if l.is_empty() {
            continue;
        }
        if in_data {
            if l.as_bytes()[0] == b'#' {
                f.timestamps += 1;
            } else {
                let c = l.as_bytes()[0];
                if c == b'0'
                    || c == b'1'
                    || c == b'x'
                    || c == b'X'
                    || c == b'z'
                    || c == b'Z'
                    || c == b'b'
                    || c == b'B'
                    || c == b'r'
                    || c == b'R'
                    || c == b's'
                {
                    f.value_changes += 1;
                }
            }
            continue;
        }
        if !l.starts_with('$') {
            continue;
        }
        saw_kw = true;
        let body = l[1..].split_whitespace().next().unwrap_or("");
        match body {
            "timescale" => {
                // `$timescale 1ns $end` — the arg is between the keyword and `$end`
                let t = l.strip_prefix("$timescale").unwrap_or("").trim();
                let arg = t.strip_suffix("$end").unwrap_or(t).trim();
                f.timescale = Some(arg.to_string());
            }
            "scope" => f.scopes += 1,
            "var" => f.vars += 1,
            "comment" => f.comments += 1,
            "enddefinitions" => {
                f.definitions_closed = true;
                in_data = true;
            }
            _ => {}
        }
    }
    if !saw_kw || !f.definitions_closed {
        return None;
    }
    Some(f)
}

/// `true` when a `$`-keyword block is present.
pub fn detect(d: &[u8]) -> bool {
    d.windows(10).any(|w| w == b"$timescale")
        || d.windows(8).any(|w| w == b"$version")
        || d.windows(15).any(|w| w == b"$enddefinitions")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"$date today $end\n$version toy $end\n\
        $comment c $end\n$timescale 1ns $end\n\
        $scope module top $end\n$var wire 1 ! clk $end\n$var reg 8 \" bus $end\n\
        $upscope $end\n$enddefinitions $end\n\
        #0\n0!\nb00000000 \"\n#5\n1!\nx!\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.timescale.as_deref(), Some("1ns"));
        assert_eq!(f.scopes, 1);
        assert_eq!(f.vars, 2);
        assert!(f.definitions_closed);
        assert_eq!(f.comments, 1);
        assert_eq!(f.timestamps, 2);
        assert_eq!(f.value_changes, 4);
    }

    #[test]
    fn no_change_rows_counted_in_header() {
        // `$var` rows begin with `$` — never confused with `0x`-style rows
        let f = parse(b"$var wire 1 ! a $end\n$enddefinitions $end\n").unwrap();
        assert_eq!(f.value_changes, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#0\n0!\n").is_none()); // no keyword block
        assert!(parse(b"$timescale 1ns $end\n").is_none()); // no $enddefinitions
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"hello"));
    }
}
