//! `.netrc` token grammar: `machine NAME` / `default` entries carry
//! `login`/`password`/`account` fields; `macdef NAME …` macro blocks
//! end at a blank line.
//!
//! ```
//! use izanagi_kit::netrc::parse;
//!
//! let d = b"machine ftp.example.com login ume password s3cr3t account a1\ndefault login anon password anon@\n";
//! let n = parse(d).unwrap();
//! assert_eq!(n.entries.len(), 2);
//! assert_eq!(n.entries[0].machine.as_deref(), Some("ftp.example.com"));
//! assert_eq!(n.entries[0].password.as_deref(), Some("s3cr3t"));
//! assert!(n.entries[1].is_default());
//! ```

use std::string::String;
use std::vec::Vec;

/// One `machine`/`default` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Machine name; `None` for `default`.
    pub machine: Option<String>,
    /// `login` value.
    pub login: Option<String>,
    /// `password` value.
    pub password: Option<String>,
    /// `account` value.
    pub account: Option<String>,
}

/// One `macdef` macro: name + verbatim body lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macdef {
    /// Macro name.
    pub name: String,
    /// Body text until the terminating blank line.
    pub body: String,
}

/// Parsed `.netrc`.
#[derive(Debug, Clone, Default)]
pub struct Netrc {
    /// `machine`/`default` entries in file order.
    pub entries: Vec<Entry>,
    /// `macdef` definitions.
    pub macros: Vec<Macdef>,
}

/// Parse a `.netrc` file.
pub fn parse(d: &[u8]) -> Option<Netrc> {
    let text = std::str::from_utf8(d).ok()?;
    let mut n = Netrc::default();
    // `macdef` blocks swallow raw lines until a blank line — split
    // off those regions first, then tokenize the rest.
    let mut toks: Vec<&str> = Vec::new();
    let mut in_mac = false;
    for line in text.split('\n') {
        let l = line.trim();
        if in_mac {
            if l.is_empty() {
                in_mac = false;
            } else if let Some(m) = n.macros.last_mut() {
                m.body.push_str(l);
                m.body.push('\n');
            }
            continue;
        }
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let mut it = l.split_whitespace().peekable();
        while let Some(t) = it.next() {
            if t == "macdef" {
                if let Some(name) = it.next() {
                    n.macros.push(Macdef {
                        name: name.into(),
                        body: String::new(),
                    });
                    in_mac = true;
                }
                break;
            }
            toks.push(t);
        }
    }
    // token walk: machine|default start an entry, k v fill it
    let mut cur: Option<Entry> = None;
    let mut i = 0usize;
    while i < toks.len() {
        match toks[i] {
            "machine" | "default" => {
                if let Some(e) = cur.take() {
                    n.entries.push(e);
                }
                let machine = if toks[i] == "machine" {
                    i += 1;
                    toks.get(i).map(|s| String::from(*s))
                } else {
                    None
                };
                cur = Some(Entry {
                    machine,
                    login: None,
                    password: None,
                    account: None,
                });
            }
            k @ ("login" | "password" | "account") => {
                i += 1;
                let v = toks.get(i).map(|s| String::from(*s));
                if let Some(e) = cur.as_mut() {
                    match k {
                        "login" => e.login = v,
                        "password" => e.password = v,
                        "account" => e.account = v,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    if let Some(e) = cur.take() {
        n.entries.push(e);
    }
    if n.entries.is_empty() && n.macros.is_empty() && !text.trim().is_empty() {
        return None;
    }
    Some(n)
}

impl Entry {
    /// `default` entry.
    pub fn is_default(&self) -> bool {
        self.machine.is_none()
    }
}

impl Netrc {
    /// Look up an entry by machine name; falls back to `default`.
    pub fn entry(&self, machine: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.machine.as_deref() == Some(machine))
            .or_else(|| self.entries.iter().find(|e| e.is_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"machine a.example.com login ume password pw\nmacdef init\nbinary\nquit\n\nmachine b.example.com login bob\ndefault login anon password none\n";

    #[test]
    fn fields() {
        let n = parse(DOC).unwrap();
        assert_eq!(n.entries.len(), 3);
        assert_eq!(n.entries[0].machine.as_deref(), Some("a.example.com"));
        assert_eq!(n.entries[0].login.as_deref(), Some("ume"));
        assert_eq!(n.entries[0].password.as_deref(), Some("pw"));
        assert_eq!(n.entries[1].login.as_deref(), Some("bob"));
        assert!(n.entries[1].password.is_none());
        let d = n.entries[2].clone();
        assert!(d.is_default());
        assert_eq!(
            n.entry("b.example.com").unwrap().login.as_deref(),
            Some("bob")
        );
        assert_eq!(n.entry("missing").unwrap().login.as_deref(), Some("anon"));
        assert_eq!(n.macros.len(), 1);
        assert_eq!(n.macros[0].name, "init");
        assert!(n.macros[0].body.contains("binary"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").unwrap().entries.is_empty());
        assert!(parse(b"\xFF\xFE").is_none());
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a config file at all\n").is_none());
    }
}
