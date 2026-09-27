//! Apache `.htaccess` / `httpd.conf` directive scanning.
//!
//! Lines are `Name arg arg ...` with `#` comments and `\` continuation.
//! Container sections `<IfModule mod_negotiation>` ... `</IfModule>`
//! attach their inner directives a `container` name; `<`/`>` args on the
//! container line itself are kept as the section name verbatim.
//!
//! ```
//! use izanagi_kit::htaccess;
//! let h = htaccess::parse(b"# c\nRewriteEngine On\n<IfModule m>\nDeny from all\n</IfModule>\n");
//! assert_eq!(h.directives[0].name, b"RewriteEngine".to_vec());
//! assert_eq!(h.directives[1].container.as_deref(), Some(b"IfModule".as_ref()));
//! ```

use std::vec::Vec;

/// One directive line.
#[derive(Clone, Debug, PartialEq)]
pub struct Directive {
    /// Directive name, verbatim case (`b"RewriteEngine"`).
    pub name: Vec<u8>,
    /// Whitespace-separated arguments.
    pub args: Vec<Vec<u8>>,
    /// Enclosing `<Section ...>` name (first word inside `<>`), if any.
    pub container: Option<Vec<u8>>,
}

/// A parsed config file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Htaccess {
    /// Directives in file order.
    pub directives: Vec<Directive>,
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && (s[a] == b' ' || s[a] == b'\t') {
        a += 1;
    }
    while b > a && (s[b - 1] == b' ' || s[b - 1] == b'\t' || s[b - 1] == b'\r') {
        b -= 1;
    }
    &s[a..b]
}

fn split_args(s: &[u8]) -> Vec<Vec<u8>> {
    s.split(|&b| b == b' ' || b == b'\t')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_vec())
        .collect()
}

/// Parses a config file; comments and blanks skipped, never fails.
pub fn parse(d: &[u8]) -> Htaccess {
    // Join `\`-continued lines.
    let mut logical: Vec<u8> = Vec::with_capacity(d.len());
    let mut i = 0;
    while i < d.len() {
        if d[i] == b'\\' && i + 1 < d.len() && d[i + 1] == b'\n' {
            logical.push(b' ');
            i += 2;
            continue;
        }
        logical.push(d[i]);
        i += 1;
    }
    let mut directives = Vec::new();
    let mut container: Option<Vec<u8>> = None;
    for raw in logical.split(|&b| b == b'\n') {
        // Strip inline comments (a `#` starts a comment anywhere).
        let line_raw = match raw.iter().position(|&b| b == b'#') {
            Some(p) => &raw[..p],
            None => raw,
        };
        let line = trim(line_raw);
        if line.is_empty() {
            continue;
        }
        if line[0] == b'<' {
            if line.get(1) == Some(&b'/') {
                // `</Name>` closes the current section.
                container = None;
                continue;
            }
            // `<Name args>` or `<Name args>` unterminated lines are kept
            // as a section with their first word as the name.
            let inner = if line[line.len() - 1] == b'>' {
                &line[1..line.len() - 1]
            } else {
                &line[1..]
            };
            let inner = trim(inner);
            let name_end = inner
                .iter()
                .position(|&b| b == b' ' || b == b'\t')
                .unwrap_or(inner.len());
            container = Some(inner[..name_end].to_vec());
            continue;
        }
        let name_end = line
            .iter()
            .position(|&b| b == b' ' || b == b'\t')
            .unwrap_or(line.len());
        directives.push(Directive {
            name: line[..name_end].to_vec(),
            args: split_args(&line[name_end..]),
            container: container.clone(),
        });
    }
    Htaccess { directives }
}

/// All directives named `name` (case-insensitive), in order.
pub fn find<'a>(h: &'a Htaccess, name: &[u8]) -> Vec<&'a Directive> {
    h.directives
        .iter()
        .filter(|d| d.name.eq_ignore_ascii_case(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives_and_args() {
        let h = parse(b"RewriteEngine On\nAddType text/html .html .htm\n");
        assert_eq!(h.directives.len(), 2);
        assert_eq!(
            h.directives[1].args,
            vec![b"text/html".to_vec(), b".html".to_vec(), b".htm".to_vec()]
        );
        assert!(h.directives[0].container.is_none());
    }

    #[test]
    fn section_nesting_flat() {
        let h = parse(b"<IfModule mod_x>\nDeny from all\n</IfModule>\nOrder a,b\n");
        assert_eq!(
            h.directives[0].container.as_deref(),
            Some(b"IfModule".as_ref())
        );
        assert!(h.directives[1].container.is_none());
    }

    #[test]
    fn comments_and_continuation() {
        let h = parse(b"A 1 # trailing\nB 2 \\\n 3\n# full\n");
        assert_eq!(h.directives.len(), 2);
        assert_eq!(h.directives[1].args, vec![b"2".to_vec(), b"3".to_vec()]);
        assert_eq!(find(&h, b"a").len(), 1);
        assert!(find(&h, b"zz").is_empty());
    }
}
