//! RFC 4180 CSV parsing — the tabular workhorse: comma-separated
//! fields, `"quoted"` fields that may contain commas, newlines and
//! `""` (an escaped literal quote), `CRLF`/`LF`/`CR` record endings,
//! and a trailing record that may or may not end in a newline.
//!
//! Tolerant by design (the dialect wilds are endless): a closing
//! quote followed by junk keeps the junk verbatim in the field until
//! the next delimiter — matching lazy-quote parsers — rather than
//! failing the whole document. Bytes in, bytes out; `parse_str` is
//! the lossy-UTF-8 convenience.
//!
//! ```
//! use izanagi_kit::csv::parse_str;
//!
//! let rows = parse_str("name,score\n\"a,b\",\"said \"\"hi\"\"\"\r\nx,9");
//! assert_eq!(rows[0], vec!["name", "score"]);
//! assert_eq!(rows[1], vec!["a,b", "said \"hi\""]);
//! assert_eq!(rows[2], vec!["x", "9"]);
//! ```

/// Parse `input` into `records → fields → raw unquoted bytes`.
/// An empty input yields no records; a trailing `\n` does not add a
/// phantom record.
pub fn parse(input: &[u8]) -> Vec<Vec<Vec<u8>>> {
    let mut rows: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut field: Vec<u8> = Vec::new();
    let mut row: Vec<Vec<u8>> = Vec::new();
    // True once the current field has consumed anything (quoted or
    // not) — a `"` only opens a quoted field at a field's very start.
    let mut touched = false;
    let mut i = 0usize;
    let n = input.len();
    while i < n {
        let c = input[i];
        if c == b'"' && !touched {
            // Quoted field: consume until the closing quote; `""` is
            // a literal quote inside.
            i += 1;
            while i < n {
                if input[i] == b'"' {
                    if i + 1 < n && input[i + 1] == b'"' {
                        field.push(b'"');
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    field.push(input[i]);
                    i += 1;
                }
            }
            touched = true;
        } else if c == b',' {
            row.push(std::mem::take(&mut field));
            touched = false;
            i += 1;
        } else if c == b'\n' || c == b'\r' {
            if c == b'\r' && i + 1 < n && input[i + 1] == b'\n' {
                i += 1;
            }
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
            touched = false;
            i += 1;
        } else {
            field.push(c);
            touched = true;
            i += 1;
        }
    }
    // Flush a trailing record without a final newline.
    if touched || !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// [`parse`] with lossy UTF-8 conversion per field — the common case
/// for text files.
pub fn parse_str(input: &str) -> Vec<Vec<String>> {
    parse(input.as_bytes())
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|f| String::from_utf8_lossy(&f).into_owned())
                .collect()
        })
        .collect()
}

/// Serialize `rows` back to canonical RFC 4180 bytes — quoting only
/// when a field contains `,` `"` `\r` or `\n`, doubling inner quotes,
/// `CRLF` record endings. `parse(emit(rows))` is an exact round-trip
/// for any input, which is what the test suite pins down.
pub fn emit(rows: &[Vec<Vec<u8>>]) -> Vec<u8> {
    let mut out = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        if r > 0 {
            out.extend_from_slice(b"\r\n");
        }
        for (i, f) in row.iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            if f.iter().any(|&c| matches!(c, b',' | b'"' | b'\r' | b'\n')) {
                out.push(b'"');
                for &c in f {
                    if c == b'"' {
                        out.push(b'"');
                    }
                    out.push(c);
                }
                out.push(b'"');
            } else {
                out.extend_from_slice(f);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(rows: &[Vec<Vec<u8>>]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| {
                r.iter()
                    .map(|f| String::from_utf8_lossy(f).into_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn rfc4180_cases() {
        // Quoted field with comma, escaped quotes, embedded newline.
        let rows = parse(b"a,\"b,c\",\"x\"\"y\"\"\"\n1,\"line1\nline2\",z\n");
        let f = fields(&rows);
        assert_eq!(f[0], vec!["a", "b,c", "x\"y\""]);
        assert_eq!(f[1], vec!["1", "line1\nline2", "z"]);
        // CRLF and lone CR endings.
        assert_eq!(
            fields(&parse(b"a,b\r\nc,d\r")),
            vec![vec!["a", "b"], vec!["c", "d"]]
        );
        // Empty fields and a trailing empty field.
        assert_eq!(fields(&parse(b"a,,c,\n")), vec![vec!["a", "", "c", ""]]);
        // A completely empty field row.
        assert_eq!(fields(&parse(b",\n")), vec![vec!["", ""]]);
        // Quoted empty field.
        assert_eq!(fields(&parse(b"\"\"\n")), vec![vec![""]]);
    }

    #[test]
    fn edge_cases() {
        // Empty input → no records.
        assert_eq!(parse(b""), Vec::<Vec<Vec<u8>>>::new());
        // Bare newline → one empty field record.
        assert_eq!(fields(&parse(b"\n")), vec![vec![""]]);
        // Whitespace is data.
        assert_eq!(fields(&parse(b" a , b \n")), vec![vec![" a ", " b "]]);
        // Quote mid-field is a literal quote (lazy-quote tolerance).
        assert_eq!(fields(&parse(b"a\"b,c\n")), vec![vec!["a\"b", "c"]]);
        // Junk after closing quote is kept verbatim.
        assert_eq!(fields(&parse(b"\"a\"b,c\n")), vec![vec!["ab", "c"]]);
        // Quoted then unquoted fields on one row.
        assert_eq!(
            fields(&parse(b"\"a\",b,\"c\",d\n")),
            vec![vec!["a", "b", "c", "d"]]
        );
    }

    #[test]
    fn emit_parse_round_trip() {
        let rows: Vec<Vec<Vec<u8>>> = vec![
            vec![b"a,b".to_vec(), b"\"q\"".to_vec(), b"plain".to_vec()],
            vec![b"".to_vec(), b"new\nline".to_vec(), b"\r".to_vec()],
        ];
        let bytes = emit(&rows);
        assert_eq!(parse(&bytes), rows);
        // Canonical form: only quoted when needed, CRLF endings.
        assert_eq!(emit(&[vec![b"x".to_vec()]]), b"x".to_vec());
        assert_eq!(emit(&[vec![b"a\nb".to_vec()]]), b"\"a\nb\"".to_vec());
    }

    #[test]
    fn parse_str_lossy() {
        let rows = parse_str("k,v\nπ,3");
        assert_eq!(rows[1], vec!["π", "3"]);
    }

    #[test]
    fn deterministic_twice() {
        let doc = b"a,\"b\nc\"\r\nd,e";
        assert_eq!(parse(doc), parse(doc));
        assert_eq!(emit(&parse(doc)), emit(&parse(doc)));
    }
}
