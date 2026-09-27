//! Texinfo: `@node`/`@chapter`/`@section` structure commands, `@macro` for
//! multi-line commands, and `@end name` pairing detection (texinfo manual).
//!
//! ```
//! use izanagi_kit::texinfo::parse;
//!
//! let d = b"@node Top\n@chapter Intro\n@verbatim\ncode\n@end verbatim\n";
//! let t = parse(d).unwrap();
//! assert_eq!(t.chapters, vec!["Intro"]);
//! assert_eq!(t.blocks[0], "verbatim");
//! ```

/// Parsed texinfo input.
#[derive(Debug, Clone)]
pub struct Texinfo {
    /// `@node` names in order.
    pub nodes: Vec<String>,
    /// `@chapter` / `@top` titles in order.
    pub chapters: Vec<String>,
    /// Block commands closed by a matching `@end name`.
    pub blocks: Vec<String>,
    /// Other commands seen (name only).
    pub others: Vec<String>,
}

/// Parse texinfo source. `@end` names are popped against the open block stack
/// — mismatched names are rejected.
pub fn parse(data: &[u8]) -> Option<Texinfo> {
    let text = std::str::from_utf8(data).ok()?;
    let mut nodes = Vec::new();
    let mut chapters = Vec::new();
    let mut blocks = Vec::new();
    let mut others = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if !t.starts_with('@') {
            continue;
        }
        let mut i = 1usize;
        while i < t.len() && t.as_bytes()[i].is_ascii_alphabetic() {
            i += 1;
        }
        if i == 1 {
            continue; // @{ } @@ etc.
        }
        let cmd = &t[1..i];
        let arg = t[i..].trim();
        match cmd {
            "node" => nodes.push(arg.to_string()),
            "chapter" | "top" | "appendix" => chapters.push(arg.to_string()),
            "end" => {
                let open = stack.pop()?;
                if arg != open {
                    return None;
                }
                blocks.push(arg.to_string());
            }
            // block-opening commands (subset: verbatim/example/quotation/itemize/...)
            "verbatim" | "example" | "quotation" | "itemize" | "enumerate" | "table" | "macro"
            | "ifset" | "ifclear" | "ignore" | "menu" => {
                stack.push(cmd.to_string());
            }
            _ => others.push(cmd.to_string()),
        }
    }
    if !stack.is_empty() {
        return None;
    }
    Some(Texinfo {
        nodes,
        chapters,
        blocks,
        others,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"@node N\n@chapter C\n@example\nx\n@end example\n";
        let t = parse(d).unwrap();
        assert_eq!(t.nodes, vec!["N"]);
        assert_eq!(t.blocks, vec!["example"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"@end bogus\n").is_none()); // close with empty stack
        assert!(parse(b"@verbatim\n@end example\n").is_none()); // mismatch
        assert!(parse(b"@verbatim\n").is_none()); // unclosed
    }
}
