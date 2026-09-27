//! OFX (Open Financial Exchange) file parsing.
//!
//! OFX 1.x files are SGML: a `KEY:VALUE` header block
//! (`OFXHEADER:100`, `DATA:OFXSG`, `VERSION:102`…), a blank line,
//! then an `<OFX>` aggregate with unclosed `<TAG>value` leaf
//! elements. OFX 2.x is XML — this parser handles the SGML form
//! and detects the XML prolog.
//!
//! ```
//! use izanagi_kit::ofx;
//! let d = b"OFXHEADER:100\nDATA:OFXSG\nVERSION:102\n\n<OFX><SIGNONMSGSRSV1></SIGNONMSGSRSV1></OFX>";
//! let o = ofx::parse(d).unwrap();
//! assert_eq!(o.header, Some(100));
//! assert_eq!(o.data.as_deref(), Some("OFXSG"));
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// A parsed OFX file.
#[derive(Clone, Debug, PartialEq)]
pub struct Ofx {
    /// True for OFX 2.x XML form.
    pub is_xml: bool,
    /// `OFXHEADER` numeric value (e.g. 100, 103, 160).
    pub header: Option<u32>,
    /// `DATA` value (e.g. `OFXSG`, `OFCREQ`).
    pub data: Option<String>,
    /// `VERSION` field.
    pub version: Option<u32>,
    /// All header `KEY:VALUE` pairs.
    pub headers: BTreeMap<String, String>,
    /// Top-level aggregate tag names inside `<OFX>` (order kept).
    pub blocks: Vec<String>,
}

/// Parses an OFX file: SGML headers, then scans `<OFX>` for
/// top-level aggregate names (nested depth 1).
pub fn parse(d: &[u8]) -> Option<Ofx> {
    let text = std::str::from_utf8(d).ok()?;
    let mut headers = BTreeMap::new();
    let mut rest = text;
    let mut is_xml = false;
    loop {
        let line_end = rest.find('\n').unwrap_or(rest.len());
        let line = rest.get(..line_end)?.trim_end_matches('\r').trim();
        rest = rest.get(line_end + 1.min(rest.len() - line_end)..)?;
        if line.is_empty() {
            break;
        }
        if line.starts_with("<?") {
            is_xml = true;
            continue;
        }
        let (k, v) = line.split_once(':')?;
        headers.insert(k.trim().to_string(), v.trim().to_string());
    }
    let mut blocks = Vec::new();
    let ofx_at = rest.find("<OFX>")?;
    let inner = rest.get(ofx_at + 5..)?;
    // scan depth-1 tags: <NAME> ... </NAME>
    let mut depth = 0u32;
    let mut cur = inner;
    while let Some(lt) = cur.find('<') {
        cur = cur.get(lt..)?;
        if cur.starts_with("</") {
            let gt = cur.find('>')?;
            let name_end = cur.get(2..gt)?.find([' ', '\t', '/']).unwrap_or(gt - 2);
            let _name = &cur[2..2 + name_end];
            depth = depth.saturating_sub(1);
            cur = cur.get(gt + 1..)?;
            continue;
        }
        let gt = cur.find('>')?;
        if gt == 1 {
            return None;
        }
        let raw = cur.get(1..gt)?;
        if raw.is_empty() {
            return None;
        }
        if let Some(name) = raw.strip_suffix('/') {
            // self-closing; still records at depth 1
            if depth == 0 {
                blocks.push(name.to_string());
            }
        } else if raw
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            && !raw.is_empty()
        {
            if depth == 0 {
                blocks.push(raw.to_string());
            }
            depth += 1;
        } else {
            // leaf with attributes or lowercase tag → still an open tag
            if depth == 0 {
                let name_end = raw.find([' ', '\t']).unwrap_or(raw.len());
                if name_end == 0 || raw.starts_with('!') || raw.starts_with('?') {
                    cur = cur.get(gt + 1..)?;
                    continue;
                }
                blocks.push(raw[..name_end].to_string());
            }
            depth += 1;
        }
        cur = cur.get(gt + 1..)?;
        if blocks.len() > 4096 {
            return None;
        }
    }
    if blocks.is_empty() {
        return None;
    }
    let parse_u32 = |k: &str| -> Option<u32> { headers.get(k)?.trim().parse().ok() };
    Some(Ofx {
        is_xml,
        header: parse_u32("OFXHEADER"),
        data: headers.get("DATA").cloned(),
        version: parse_u32("VERSION"),
        headers,
        blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    const DOC: &[u8] = b"OFXHEADER:100\nDATA:OFXSG\nVERSION:102\nSECURITY:NONE\nENCODING:USASCII\nCHARSET:1252\nCOMPRESSION:NONE\nOLDFILEUID:NONE\nNEWFILEUID:NONE\n\n<OFX><SIGNONMSGSRSV1><SONRS><STATUS><CODE>0</CODE></STATUS></SONRS></SIGNONMSGSRSV1><BANKMSGSRSV1></BANKMSGSRSV1></OFX>";

    #[test]
    fn parses_headers_and_blocks() {
        let o = parse(DOC).unwrap();
        assert_eq!(o.header, Some(100));
        assert_eq!(o.data.as_deref(), Some("OFXSG"));
        assert_eq!(o.version, Some(102));
        assert_eq!(o.blocks, vec!["SIGNONMSGSRSV1", "BANKMSGSRSV1"]);
        assert!(!o.is_xml);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"no headers").is_none());
        assert!(parse(b"OFXHEADER:100\n\n").is_none()); // no <OFX>
        assert!(parse(b"OFXHEADER:100\n\n<OFX></OFX>").is_none()); // empty
    }
}
