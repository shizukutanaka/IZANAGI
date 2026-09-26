//! NASA PDS3 label (ODL) parsing — `KEY = value` statements, `OBJECT` /
//! `END_OBJECT` blocks, terminated by `END`.
//!
//! ```
//! use izanagi_kit::pds::parse;
//!
//! let l = b"PDS_VERSION_ID = PDS3\r\nOBJECT = FILE\r\n^IMAGE = 100\r\nEND_OBJECT = FILE\r\nEND\r\n";
//! let p = parse(l).unwrap();
//! assert_eq!(p.get("PDS_VERSION_ID"), Some(&b"PDS3"[..]));
//! ```

/// One `KEY = value` statement (raw bytes, spaces trimmed).
#[derive(Debug, Clone, Copy)]
pub struct Assign<'a> {
    /// Key name.
    pub key: &'a [u8],
    /// Value bytes (may start with `^` for pointer values, quoted, or bare).
    pub value: &'a [u8],
}

/// One `OBJECT`/`GROUP` block.
#[derive(Debug, Clone, Copy)]
pub struct Object<'a> {
    /// `OBJECT` or `GROUP` marker name.
    pub name: &'a [u8],
    /// Assignment indices inside this block (into `Pds::assigns`).
    pub span: (usize, usize),
}

/// Parsed PDS3 label.
pub struct Pds<'a> {
    /// All top-level and nested assignments, in order.
    pub assigns: Vec<Assign<'a>>,
    /// Object/group blocks in order.
    pub objects: Vec<Object<'a>>,
    /// Whether a terminating `END` line was seen.
    pub terminated: bool,
}

impl<'a> Pds<'a> {
    /// First value for `key` (case-sensitive, matches PDS convention).
    pub fn get(&self, key: &str) -> Option<&'a [u8]> {
        self.assigns
            .iter()
            .find(|a| a.key == key.as_bytes())
            .map(|a| a.value)
    }
}

fn trim(b: &[u8]) -> &[u8] {
    let mut s = b;
    while let Some((&f, r)) = s.split_first() {
        if f == b' ' || f == b'\t' || f == b'\r' {
            s = r;
        } else {
            break;
        }
    }
    while let Some(&l) = s.last() {
        if l == b' ' || l == b'\t' || l == b'\r' {
            s = &s[..s.len() - 1];
        } else {
            break;
        }
    }
    s
}

/// Parses a PDS3 ODL label. Tolerates LF or CRLF endings and `/* */`-style
/// comments on their own lines.
pub fn parse(d: &[u8]) -> Option<Pds<'_>> {
    if d.is_empty() {
        return None;
    }
    let mut assigns = Vec::new();
    let mut objects = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (assign_start, objects idx)
    let mut terminated = false;
    let mut saw_version = false;
    for raw in d.split(|&b| b == b'\n') {
        let line = trim(raw);
        if line.is_empty() || line.starts_with(b"/*") {
            continue;
        }
        if line == b"END" {
            terminated = true;
            break;
        }
        if let Some(rest) = line
            .strip_prefix(b"OBJECT =")
            .or_else(|| line.strip_prefix(b"OBJECT="))
            .or_else(|| line.strip_prefix(b"GROUP ="))
            .or_else(|| line.strip_prefix(b"GROUP="))
        {
            stack.push((assigns.len(), objects.len()));
            objects.push(Object {
                name: trim(rest),
                span: (assigns.len(), assigns.len()),
            });
            continue;
        }
        if line.starts_with(b"END_OBJECT") || line.starts_with(b"END_GROUP") {
            if let Some((start, oi)) = stack.pop() {
                if let Some(o) = objects.get_mut(oi) {
                    o.span = (start, assigns.len());
                }
            }
            continue;
        }
        if let Some(eq) = line.iter().position(|&b| b == b'=') {
            let key = trim(&line[..eq]);
            if key.is_empty() {
                return None;
            }
            if key == b"PDS_VERSION_ID" {
                saw_version = true;
            }
            assigns.push(Assign {
                key,
                value: trim(&line[eq + 1..]),
            });
        }
    }
    if !terminated || !saw_version {
        return None;
    }
    Some(Pds {
        assigns,
        objects,
        terminated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABEL: &[u8] = b"PDS_VERSION_ID = PDS3\r\n\
RECORD_TYPE = STREAM\r\n\
OBJECT = FILE\r\n\
^IMAGE = 1024\r\n\
END_OBJECT = FILE\r\n\
GROUP = PARAMETERS\r\n\
SAMPLE_BITS = 16\r\n\
END_GROUP = PARAMETERS\r\n\
END\r\n";

    #[test]
    fn parses() {
        let p = parse(LABEL).unwrap();
        assert!(p.terminated);
        assert_eq!(p.get("PDS_VERSION_ID"), Some(&b"PDS3"[..]));
        assert_eq!(p.get("RECORD_TYPE"), Some(&b"STREAM"[..]));
        assert_eq!(p.get("SAMPLE_BITS"), Some(&b"16"[..]));
        assert_eq!(p.objects.len(), 2);
        assert_eq!(p.objects[0].name, b"FILE");
        assert_eq!(p.objects[1].name, b"PARAMETERS");
        assert_eq!(p.objects[0].span, (2, 3));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        // no END
        assert!(parse(b"PDS_VERSION_ID = PDS3\nA = B\n").is_none());
        // no version
        assert!(parse(b"A = B\nEND\n").is_none());
    }
}
