//! DIF (Data Interchange Format) — VisiCalc/Lotus-era spreadsheet exchange.
//!
//! Layout: `TABLE` / `VECTORS` / `TUPLES` / `DATA` headers, each
//! `number1,number2` + `"quoted string"`; rows start with `-1,0` `BOT`,
//! the file ends with `EOD`.
//!
//! ```
//! let d = b"TABLE\n0,1\n\"EXCEL\"\nVECTORS\n0,3\n\"\"\nTUPLES\n0,2\n\"\"\nDATA\n0,0\n\"\"\n-1,0\nBOT\n1,0\n\"x\"\n-1,0\nBOT\n-1,0\nEOD\n";
//! let f = izanagi_kit::dif::parse(d).unwrap();
//! assert_eq!(f.tuples, 2);
//! assert_eq!(f.records, 2);
//! ```

/// Parsed DIF header summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dif {
    /// `VECTORS` declared column count.
    pub vectors: u32,
    /// `TUPLES` declared row count.
    pub tuples: u32,
    /// `BOT` (beginning-of-tuple) markers counted in the data section.
    pub records: usize,
    /// Header keywords seen, in order (`TABLE`, `VECTORS`, `TUPLES`, `DATA`).
    pub headers: Vec<String>,
}

/// Read the `n1,n2` pair on the line after a header keyword.
fn pair(lines: &[&str], i: usize) -> Option<(i64, i64)> {
    let l = lines.get(i)?;
    let (a, b) = l.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// Parse a DIF file; `None` without `TABLE`, `DATA`, and `EOD`.
pub fn parse(d: &[u8]) -> Option<Dif> {
    let s = std::str::from_utf8(d).ok()?;
    let lines: Vec<&str> = s.lines().collect();
    if lines.first()?.trim() != "TABLE" {
        return None;
    }
    let mut vectors = 0u32;
    let mut tuples = 0u32;
    let mut records = 0usize;
    let mut headers = Vec::new();
    let mut saw_eod = false;
    let mut i = 0usize;
    while i < lines.len() {
        let l = lines[i].trim();
        match l {
            "TABLE" | "VECTORS" | "TUPLES" | "DATA" => {
                headers.push(l.to_string());
                if let Some((_, v)) = pair(&lines, i + 1) {
                    match l {
                        "VECTORS" => vectors = v.max(0) as u32,
                        "TUPLES" => tuples = v.max(0) as u32,
                        _ => {}
                    }
                }
                i += 3; // keyword, n,n, "label"
            }
            "BOT" => {
                records += 1;
                i += 1;
            }
            "EOD" => {
                saw_eod = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if !saw_eod || !headers.iter().any(|h| h == "DATA") {
        return None;
    }
    Some(Dif {
        vectors,
        tuples,
        records,
        headers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"TABLE\n0,1\n\"EXCEL\"\nVECTORS\n0,3\n\"\"\nTUPLES\n0,2\n\"\"\nDATA\n0,0\n\"\"\n-1,0\nBOT\n-1,0\nBOT\n-1,0\nEOD\n";
        let f = parse(d).unwrap();
        assert_eq!((f.vectors, f.tuples, f.records), (3, 2, 2));
        assert_eq!(f.headers, vec!["TABLE", "VECTORS", "TUPLES", "DATA"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"DATA\n0,0\n\"\"\n").is_none()); // no TABLE
        assert!(parse(b"TABLE\n0,1\n\"x\"\n").is_none()); // no DATA/EOD
    }
}
