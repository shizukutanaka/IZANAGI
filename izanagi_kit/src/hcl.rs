//! HCL/Terraform-style body parsing (subset, structural).
//!
//! Handles the common surface: `name = value` attributes
//! (string/number/bool/list), `type "label1" "label2" { … }`
//! nested blocks, `//`/`#`/`/* */` comments. Values are kept
//! verbatim text — no expression evaluation.
//!
//! ```
//! use izanagi_kit::hcl;
//! let d = b"region = \"us-east-1\"\nresource \"aws_s3_bucket\" \"b\" {\n  acl = \"private\"\n}\n";
//! let h = hcl::parse(d).unwrap();
//! assert_eq!(h.blocks[0].ty, "resource");
//! assert_eq!(h.blocks[0].labels, vec!["aws_s3_bucket".to_string(), "b".to_string()]);
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// A `type "l1" "l2" { … }` block (one level, recursive).
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// Block type (`resource`, `provider`, `terraform`…).
    pub ty: String,
    /// Quoted labels.
    pub labels: Vec<String>,
    /// Attributes inside the block.
    pub attrs: BTreeMap<String, String>,
    /// Nested blocks.
    pub children: Vec<Block>,
    /// 1-based line where the block opens.
    pub line: usize,
}

/// A parsed HCL document.
#[derive(Clone, Debug, PartialEq)]
pub struct Hcl {
    /// Top-level attributes.
    pub attrs: BTreeMap<String, String>,
    /// Top-level blocks.
    pub blocks: Vec<Block>,
}

struct Cursor<'a> {
    b: &'a [u8],
    at: usize,
    line: usize,
}

impl<'a> Cursor<'a> {
    fn ws(&mut self) {
        loop {
            match self.b.get(self.at) {
                Some(&c) if c == b' ' || c == b'\t' || c == b'\r' => self.at += 1,
                Some(&b'\n') => {
                    self.at += 1;
                    self.line += 1;
                }
                Some(&b'#') => {
                    while let Some(&c) = self.b.get(self.at) {
                        self.at += 1;
                        if c == b'\n' {
                            break;
                        }
                    }
                }
                Some(&b'/') if self.b.get(self.at + 1) == Some(&b'/') => {
                    while let Some(&c) = self.b.get(self.at) {
                        self.at += 1;
                        if c == b'\n' {
                            break;
                        }
                    }
                }
                Some(&b'/') if self.b.get(self.at + 1) == Some(&b'*') => {
                    while self.at + 1 < self.b.len() {
                        if self.b[self.at] == b'*' && self.b[self.at + 1] == b'/' {
                            self.at += 2;
                            break;
                        }
                        if self.b[self.at] == b'\n' {
                            self.line += 1;
                        }
                        self.at += 1;
                    }
                }
                _ => break,
            }
        }
    }
    fn word(&mut self) -> Option<String> {
        self.ws();
        let start = self.at;
        match self.b.get(self.at) {
            Some(&c) if c.is_ascii_alphabetic() || c == b'_' => {}
            _ => return None,
        }
        while let Some(&c) = self.b.get(self.at) {
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
                self.at += 1;
            } else {
                break;
            }
        }
        if self.at == start {
            return None;
        }
        String::from_utf8(self.b[start..self.at].to_vec()).ok()
    }
    fn string(&mut self) -> Option<String> {
        self.ws();
        if self.b.get(self.at) != Some(&b'"') {
            return None;
        }
        self.at += 1;
        let start = self.at;
        while let Some(&c) = self.b.get(self.at) {
            if c == b'"' {
                let s = String::from_utf8(self.b[start..self.at].to_vec()).ok();
                self.at += 1;
                return s;
            }
            if c == b'\\' {
                self.at += 1;
            }
            self.at += 1;
        }
        None
    }
    fn value(&mut self) -> Option<String> {
        self.ws();
        let start = self.at;
        let mut depth = 0i32;
        while let Some(&c) = self.b.get(self.at) {
            match c {
                b'[' | b'{' | b'(' => depth += 1,
                b']' | b'}' | b')' => {
                    depth -= 1;
                    if depth < 0 {
                        return None;
                    }
                }
                b'\n' if depth == 0 => break,
                _ => {}
            }
            self.at += 1;
        }
        let s = std::str::from_utf8(&self.b[start..self.at]).ok()?.trim();
        if s.is_empty() {
            return None;
        }
        Some(s.to_string())
    }
}

/// Parses an HCL-ish document. Unbalanced braces or a missing
/// `=`/`{` fail the whole parse.
pub fn parse(d: &[u8]) -> Option<Hcl> {
    let mut c = Cursor {
        b: d,
        at: 0,
        line: 1,
    };
    let (attrs, blocks) = body(&mut c, false)?;
    if blocks.is_empty() && attrs.is_empty() {
        return None;
    }
    Some(Hcl { attrs, blocks })
}

fn body(c: &mut Cursor, inside: bool) -> Option<(BTreeMap<String, String>, Vec<Block>)> {
    let mut attrs = BTreeMap::new();
    let mut blocks = Vec::new();
    loop {
        c.ws();
        if c.at >= c.b.len() {
            if inside {
                return None; // unclosed
            }
            break;
        }
        if inside && c.b[c.at] == b'}' {
            c.at += 1;
            break;
        }
        if c.b[c.at] == b'}' && !inside {
            return None;
        }
        let start_line = c.line;
        let name = c.word()?;
        c.ws();
        if c.b.get(c.at) == Some(&b'=') {
            c.at += 1;
            let v = c.value()?;
            attrs.insert(name, v);
            continue;
        }
        // block: labels until `{`
        let mut labels = Vec::new();
        loop {
            c.ws();
            match c.b.get(c.at) {
                Some(b'{') => {
                    c.at += 1;
                    break;
                }
                Some(b'"') => labels.push(c.string()?),
                _ => return None,
            }
        }
        let (a, ch) = body(c, true)?;
        blocks.push(Block {
            ty: name,
            labels,
            attrs: a,
            children: ch,
            line: start_line,
        });
    }
    Some((attrs, blocks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"# c\nvar = 1\nresource \"aws_vpc\" \"main\" {\n  cidr = \"10.0.0.0/16\"\n  tags = {\n    name = \"x\"\n  }\n}\nterraform {\n  required_version = \">= 1.0\"\n}\n";
        let h = parse(d).unwrap();
        assert_eq!(h.attrs.get("var").unwrap(), "1");
        assert_eq!(h.blocks.len(), 2);
        assert_eq!(h.blocks[0].labels.len(), 2);
        assert!(h.blocks[0].attrs.contains_key("cidr"));
        assert_eq!(h.blocks[1].ty, "terraform");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"resource \"x\" {\n").is_none()); // unclosed
        assert!(parse(b"}\n").is_none());
        assert!(parse(b"1abc = 2\n").is_none());
    }
}
