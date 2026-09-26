//! Org-mode: `* heading` levels, `#+KEY: value` keywords, `#+BEGIN_X`/
//! `#+END_X` blocks, and TODO keyword detection at heading start.
//!
//! ```
//! use izanagi_kit::org::parse;
//!
//! let d = b"#+TITLE: Doc\n* TODO Task one\n** DONE Sub\n#+BEGIN_SRC sh\nls\n#+END_SRC\n";
//! let o = parse(d).unwrap();
//! assert_eq!(o.title.as_deref(), Some("Doc"));
//! assert_eq!(o.headings[0].todo.as_deref(), Some("TODO"));
//! assert_eq!(o.blocks.len(), 1);
//! ```

/// One headline.
#[derive(Debug, Clone)]
pub struct Heading {
    /// `*` count.
    pub level: usize,
    /// TODO keyword if present (`TODO`/`DONE`/`NEXT`/`WAITING`).
    pub todo: Option<String>,
    /// Heading text after the keyword.
    pub title: String,
}

/// Parsed org file.
#[derive(Debug, Clone)]
pub struct Org {
    /// `#+TITLE:` value.
    pub title: Option<String>,
    /// All `#+KEY: value` pairs (uppercased key).
    pub keywords: Vec<(String, String)>,
    /// Headlines in order.
    pub headings: Vec<Heading>,
    /// `#+BEGIN_`/`#+END_` block names (paired).
    pub blocks: Vec<String>,
}

const TODOS: &[&str] = &["TODO", "DONE", "NEXT", "WAITING", "CANCELLED"];

/// Parse org source. `BEGIN`/`END` names must pair.
pub fn parse(data: &[u8]) -> Option<Org> {
    let text = std::str::from_utf8(data).ok()?;
    let mut title = None;
    let mut keywords = Vec::new();
    let mut headings = Vec::new();
    let mut blocks = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    for line in text.lines() {
        let t = line.trim_end();
        if t.starts_with('*') {
            let lvl = t.bytes().take_while(|&b| b == b'*').count();
            let rest = t[lvl..].trim_start();
            let (todo, title_txt) = match rest.split_once(' ') {
                Some((w, r)) if TODOS.contains(&w) => (Some(w.to_string()), r),
                _ => (None, rest),
            };
            headings.push(Heading {
                level: lvl,
                todo,
                title: title_txt.to_string(),
            });
            continue;
        }
        if let Some(rest) = t.strip_prefix("#+") {
            if let Some(c) = rest.find(':') {
                let k = rest[..c].to_uppercase();
                let v = rest[c + 1..].trim().to_string();
                if k == "TITLE" {
                    title = Some(v.clone());
                }
                keywords.push((k, v));
                continue;
            }
            if let Some(name) = rest.strip_prefix("BEGIN_") {
                stack.push(name.split_whitespace().next()?.to_string());
                continue;
            }
            if let Some(name) = rest.strip_prefix("END_") {
                let open = stack.pop()?;
                if name.trim_end() != open {
                    return None;
                }
                blocks.push(open);
                continue;
            }
        }
    }
    if !stack.is_empty() {
        return None;
    }
    Some(Org {
        title,
        keywords,
        headings,
        blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"#+TITLE: T\n* TODO A\n* plain\n#+BEGIN_QUOTE\nq\n#+END_QUOTE\n";
        let o = parse(d).unwrap();
        assert_eq!(o.headings.len(), 2);
        assert_eq!(o.headings[1].todo, None);
        assert_eq!(o.blocks, vec!["QUOTE"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"#+END_X\n").is_none());
        assert!(parse(b"#+BEGIN_SRC\n").is_none());
    }
}
