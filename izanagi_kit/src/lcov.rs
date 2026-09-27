//! LCOV coverage files (`lcov.info`) — per-source line/function data.
//!
//! `TN:` test name, `SF:` source file opens a record, `FN:`/`FNDA:`
//! functions, `FNH/FNF` functions hit/found, `DA:line,hits` line data,
//! `LH:`/`LF:` lines hit/found, `end_of_record` closes a record.
//!
//! ```
//! use izanagi_kit::lcov::parse;
//!
//! let l = parse(b"TN:run1\nSF:src/a.c\nFN:1,main\nFNDA:3,main\nDA:1,5\nDA:2,0\nLF:2\nLH:1\nend_of_record\n").unwrap();
//! assert_eq!(l.files[0].path, "src/a.c");
//! assert_eq!(l.files[0].hits, vec![(1, 5), (2, 0)]);
//! assert_eq!(l.files[0].lines_hit, Some(1));
//! ```

/// Per-source-file coverage record.
#[derive(Clone, Debug)]
pub struct LcovFile {
    /// `SF:` path.
    pub path: String,
    /// `FN:` `(line, name)` function table.
    pub functions: Vec<(u32, String)>,
    /// `FNDA:` hit counts per function `(hits, name)`.
    pub function_hits: Vec<(u64, String)>,
    /// `DA:` `(line, hit count)` pairs in file order.
    pub hits: Vec<(u32, u64)>,
    /// `LF:` lines found.
    pub lines_found: Option<u64>,
    /// `LH:` lines hit.
    pub lines_hit: Option<u64>,
}

/// A parsed lcov document.
#[derive(Clone, Debug)]
pub struct Lcov {
    /// Last `TN:` test name.
    pub testname: Option<String>,
    /// File records (`SF:`…`end_of_record`).
    pub files: Vec<LcovFile>,
}

fn kv<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    Some(t.strip_prefix(key)?.trim_start())
}

/// Parse an lcov tracefile. `None` when no complete `SF:` record exists
/// or a data line is malformed.
pub fn parse(d: &[u8]) -> Option<Lcov> {
    let text = std::str::from_utf8(d).ok()?;
    let mut testname = None;
    let mut files = Vec::new();
    let mut cur: Option<LcovFile> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(v) = kv(t, "TN:") {
            testname = Some(v.to_string());
        } else if let Some(v) = kv(t, "SF:") {
            if let Some(f) = cur.take() {
                files.push(f);
            }
            cur = Some(LcovFile {
                path: v.to_string(),
                functions: Vec::new(),
                function_hits: Vec::new(),
                hits: Vec::new(),
                lines_found: None,
                lines_hit: None,
            });
        } else if let Some(v) = kv(t, "FNDA:") {
            let (n, name) = v.split_once(',')?;
            cur.as_mut()?
                .function_hits
                .push((n.parse().ok()?, name.to_string()));
        } else if let Some(v) = kv(t, "FN:") {
            let (l, name) = v.split_once(',')?;
            cur.as_mut()?
                .functions
                .push((l.parse().ok()?, name.to_string()));
        } else if let Some(v) = kv(t, "FNF:") {
            v.parse::<u64>().ok()?;
        } else if let Some(v) = kv(t, "FNH:") {
            v.parse::<u64>().ok()?;
        } else if let Some(v) = kv(t, "DA:") {
            let (l, c) = v.split_once(',')?;
            cur.as_mut()?.hits.push((l.parse().ok()?, c.parse().ok()?));
        } else if let Some(v) = kv(t, "LF:") {
            cur.as_mut()?.lines_found = Some(v.parse().ok()?);
        } else if let Some(v) = kv(t, "LH:") {
            cur.as_mut()?.lines_hit = Some(v.parse().ok()?);
        } else if t.starts_with("end_of_record") {
            if let Some(f) = cur.take() {
                files.push(f);
            }
        }
    }
    if let Some(f) = cur.take() {
        files.push(f);
    }
    if files.is_empty() {
        return None;
    }
    Some(Lcov { testname, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let l = parse(b"TN:t\nSF:a.c\nFN:1,f\nFNDA:0,f\nFNF:1\nFNH:0\nDA:3,1\nLF:1\nLH:1\nend_of_record\nSF:b.c\nDA:1,2\nend_of_record\n").unwrap();
        assert_eq!(l.testname.as_deref(), Some("t"));
        assert_eq!(l.files.len(), 2);
        assert_eq!(l.files[0].functions[0].1, "f");
        assert_eq!(l.files[0].function_hits[0].0, 0);
        assert_eq!(l.files[1].hits, vec![(1, 2)]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"TN:x\n").is_none());
        assert!(parse(b"SF:a\nDA:x,1\n").is_none());
    }
}
