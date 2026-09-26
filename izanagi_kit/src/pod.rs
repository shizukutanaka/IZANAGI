//! Perl POD: `=head1..4`, `=item`, `=over`/`=back`, `=begin`/`=end` format
//! blocks, and `=pod`/`=cut` pod-region markers (perlpod).
//!
//! ```
//! use izanagi_kit::pod::parse;
//!
//! let d = b"=head1 NAME\n\nprog - demo\n\n=item a\n\n=cut\n";
//! let p = parse(d).unwrap();
//! assert_eq!(p.heads[0], (1, "NAME".to_string()));
//! assert_eq!(p.items, 1);
//! ```

/// Parsed POD.
#[derive(Debug, Clone)]
pub struct Pod {
    /// `(level, title)` of each `=headN`.
    pub heads: Vec<(usize, String)>,
    /// `=item` count.
    pub items: usize,
    /// `=begin fmt` names with a matching `=end fmt`.
    pub blocks: Vec<String>,
    /// Whether a `=cut` (end-of-pod) marker was seen.
    pub cut: bool,
    /// Other `=cmd` names seen.
    pub others: Vec<String>,
}

/// Parse POD source.
pub fn parse(data: &[u8]) -> Option<Pod> {
    let text = std::str::from_utf8(data).ok()?;
    let mut heads = Vec::new();
    let mut items = 0usize;
    let mut blocks = Vec::new();
    let mut cut = false;
    let mut others = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    for line in text.lines() {
        let t = line;
        if !t.starts_with('=') {
            continue;
        }
        let mut i = 1usize;
        while i < t.len() && t.as_bytes()[i].is_ascii_alphanumeric() {
            i += 1;
        }
        if i == 1 || !t.as_bytes()[1].is_ascii_alphabetic() {
            return None; // '=' alone or '=1' is invalid
        }
        let cmd = &t[1..i];
        let arg = t[i..].trim();
        match cmd {
            "head" => return None, // needs digit
            "head1" | "head2" | "head3" | "head4" => {
                heads.push((usize::from(cmd.as_bytes()[4] - b'0'), arg.to_string()));
            }
            "item" => items += 1,
            "cut" => cut = true,
            "begin" => stack.push(arg.to_string()),
            "end" => {
                let open = stack.pop()?;
                if arg != open {
                    return None;
                }
                blocks.push(open);
            }
            _ => others.push(cmd.to_string()),
        }
    }
    if !stack.is_empty() {
        return None;
    }
    Some(Pod {
        heads,
        items,
        blocks,
        cut,
        others,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d =
            b"=pod\n\n=head1 A\n\n=over\n\n=item x\n\n=back\n\n=begin html\n\n=end html\n\n=cut\n";
        let p = parse(d).unwrap();
        assert_eq!(p.heads[0].0, 1);
        assert_eq!(p.items, 1);
        assert!(p.cut);
        assert_eq!(p.blocks, vec!["html"]);
        assert!(p.others.contains(&"pod".to_string()));
        assert!(p.others.contains(&"over".to_string()));
        assert!(p.others.contains(&"back".to_string()));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"=end html\n").is_none());
        assert!(parse(b"=begin x\n=end y\n").is_none());
        assert!(parse(b"=\n").is_none());
    }
}
