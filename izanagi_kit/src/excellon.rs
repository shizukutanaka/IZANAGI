//! Excellon CNC drill file parsing.
//!
//! `M48` header block: `METRIC`/`INCH`, tool rows `Tn C<diam>` etc., `%`
//! ends the header; body has `Tn` tool selects and `X..Y..` hits; `M30` end.
//!
//! ```
//! use izanagi_kit::excellon::parse;
//!
//! let f = b"M48\nMETRIC\nT1C0.800\nT2C1.000\n%\nT1\nX1000Y2000\nT2\nM30\n";
//! let r = parse(f).unwrap();
//! assert!(r.metric);
//! assert_eq!(r.tools.len(), 2);
//! assert_eq!(r.holes.len(), 1);
//! ```

/// One drill hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hole {
    /// Tool index (1-based).
    pub tool: u8,
    /// X coordinate verbatim text (scaled per file units).
    pub x: Vec<u8>,
    /// Y coordinate verbatim text.
    pub y: Vec<u8>,
}

/// A tool row `TnC<diameter>`; diameter kept verbatim (fixed-point text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// Tool number.
    pub num: u8,
    /// Diameter text (e.g. `0.800`).
    pub diameter: Vec<u8>,
}

/// Parsed Excellon file.
pub struct Excellon {
    /// `METRIC` (vs `INCH`); required in header.
    pub metric: bool,
    /// Tool definitions in order.
    pub tools: Vec<Tool>,
    /// Drill hits in order.
    pub holes: Vec<Hole>,
}

/// Parses an Excellon drill file. Requires `M48`, units, `%`, `M30`.
pub fn parse(d: &[u8]) -> Option<Excellon> {
    let mut metric = None;
    let mut tools = Vec::new();
    let mut holes = Vec::new();
    let mut in_header = false;
    let mut ended = false;
    let mut cur_tool = 0u8;
    for line in d.split(|&b| b == b'\n') {
        let line = trim(line);
        if line.is_empty() || line.starts_with(b";") {
            continue;
        }
        if line.starts_with(b"M48") {
            in_header = true;
            continue;
        }
        if line == b"%" {
            in_header = false;
            continue;
        }
        if line.starts_with(b"M30") {
            ended = true;
            break;
        }
        if in_header {
            if line.starts_with(b"METRIC") {
                metric = Some(true);
                continue;
            }
            if line.starts_with(b"INCH") {
                metric = Some(false);
                continue;
            }
            if line[0] == b'T' {
                let cpos = line.iter().position(|&b| b == b'C')?;
                let num = parse_u8(&line[1..cpos])?;
                tools.push(Tool {
                    num,
                    diameter: line[cpos + 1..].to_vec(),
                });
            }
            continue;
        }
        if line[0] == b'T' && !line.contains(&b'X') {
            cur_tool = parse_u8(&line[1..])?;
            continue;
        }
        // X..Y.. hit
        if let (Some(xp), Some(yp)) = (
            line.iter().position(|&b| b == b'X'),
            line.iter().position(|&b| b == b'Y'),
        ) {
            if yp > xp {
                holes.push(Hole {
                    tool: cur_tool,
                    x: line[xp + 1..yp].to_vec(),
                    y: line[yp + 1..].to_vec(),
                });
            }
        }
    }
    if !ended || tools.is_empty() {
        return None;
    }
    Some(Excellon {
        metric: metric?,
        tools,
        holes,
    })
}

fn trim(s: &[u8]) -> &[u8] {
    let mut e = s.len();
    while e > 0 && matches!(s[e - 1], b'\r' | b' ' | b'\t') {
        e -= 1;
    }
    let mut b0 = 0;
    while b0 < e && matches!(s[b0], b' ' | b'\t') {
        b0 += 1;
    }
    &s[b0..e]
}

fn parse_u8(d: &[u8]) -> Option<u8> {
    let mut v: u32 = 0;
    for &b in d {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    u8::try_from(v).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let f = b"M48\nMETRIC\nT1C0.800\nT2C1.000\n%\nT1\nX1000Y2000\nX1500Y2500\nT2\nX9Y8\nM30\n";
        let r = parse(f).unwrap();
        assert!(r.metric);
        assert_eq!(r.tools.len(), 2);
        assert_eq!(r.tools[0].diameter, b"0.800");
        assert_eq!(r.holes.len(), 3);
        assert_eq!(r.holes[0].tool, 1);
        assert_eq!(r.holes[2].tool, 2);
        assert_eq!(r.holes[2].x, b"9");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"M48\n%\nM30").is_none()); // no units
        assert!(parse(b"M48\nMETRIC\n%\nM30").is_none()); // no tools
        assert!(parse(b"M48\nMETRIC\nT1C0.5\n%\nT1\nX1Y2\n").is_none()); // no M30
    }
}
