//! IMAP wire lines (RFC 3501): tagged commands `tag CMD args`,
//! untagged responses `* TYPE …`, and continuation requests `+ …`.
//! `parse_line` classifies one line; `parse` consumes a transcript.
//!
//! ```
//! use izanagi_kit::imap::{parse_line, Line};
//! let l = parse_line(b"a001 LOGIN user pass").unwrap();
//! assert_eq!(l.tag, Some("a001".to_string()));
//! let u = parse_line(b"* 23 FETCH (FLAGS ())").unwrap();
//! assert_eq!(u.verb, "23");
//! assert_eq!(u.rest, "FETCH (FLAGS ())");
//! ```

use std::string::String;
use std::vec::Vec;

/// Classification of an IMAP protocol line.
#[derive(Clone, Debug)]
pub struct Line {
    /// Tag for client commands / tagged replies (`tag OK …`); `None`
    /// for untagged `*` and continuation `+` lines.
    pub tag: Option<String>,
    /// First token after the tag/marker — command verb for commands,
    /// response keyword (`OK`, `NO`, `BAD`, `FLAGS`, number, …) for
    /// untagged lines, `+` payload head for continuations.
    pub verb: String,
    /// Everything after `verb` (trimmed of one leading space).
    pub rest: String,
    /// Line classification.
    pub kind: Kind,
}

/// Which wire role a line plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `tag COMMAND args` — client command or tagged status reply.
    Tagged,
    /// `* …` — untagged data/status.
    Untagged,
    /// `+ …` — continuation request / command continuation.
    Continuation,
}

/// Tagged-line verbs: the RFC 3501 command set plus the tagged status
/// atoms (`OK`/`NO`/`BAD`) and widely deployed extension commands.
/// `X`-prefixed experimental commands are also accepted.
const COMMAND_VERBS: &[&str] = &[
    "APPEND",
    "AUTHENTICATE",
    "CAPABILITY",
    "CHECK",
    "CLOSE",
    "COMPRESS",
    "COPY",
    "CREATE",
    "DELETE",
    "DELETEACL",
    "ENABLE",
    "EXAMINE",
    "EXPUNGE",
    "FETCH",
    "GETACL",
    "GETMETADATA",
    "GETQUOTA",
    "GETQUOTAROOT",
    "ID",
    "IDLE",
    "LIST",
    "LISTRIGHTS",
    "LOGIN",
    "LOGOUT",
    "LSUB",
    "MOVE",
    "MYRIGHTS",
    "NAMESPACE",
    "NOOP",
    "RENAME",
    "SEARCH",
    "SELECT",
    "SETACL",
    "SETMETADATA",
    "SETQUOTA",
    "SORT",
    "STARTTLS",
    "STATUS",
    "STORE",
    "SUBSCRIBE",
    "THREAD",
    "UID",
    "UNSELECT",
    "UNSUBSCRIBE",
    "XLIST",
    "BAD",
    "NO",
    "OK",
];

/// Untagged `*` first-word keywords; a bare message number (`* 23
/// FETCH …`) is also accepted.
const UNTAGGED_VERBS: &[&str] = &[
    "ALERT",
    "ANNOTATION",
    "BAD",
    "BYE",
    "CAPABILITY",
    "ENABLED",
    "ESEARCH",
    "EXISTS",
    "EXPUNGE",
    "FETCH",
    "FLAGS",
    "ID",
    "IDLE",
    "LANGUAGE",
    "LIST",
    "LSUB",
    "METADATA",
    "MYRIGHTS",
    "NAMESPACE",
    "NO",
    "OK",
    "PARSE",
    "PREAUTH",
    "QRESYNC",
    "QUOTA",
    "QUOTAROOT",
    "READ-ONLY",
    "READ-WRITE",
    "RECENT",
    "SEARCH",
    "STATUS",
    "TRYCREATE",
    "VANISHED",
    "XLIST",
];

fn known(verb: &str, set: &[&str]) -> bool {
    verb.get(..1).is_some_and(|c| c.eq_ignore_ascii_case("X"))
        || set.iter().any(|v| v.eq_ignore_ascii_case(verb))
}

/// Parse one IMAP line (no trailing CR/LF); `None` on empty input, a
/// missing tag/verb, or a verb outside the RFC 3501 command set and
/// its common registered extensions (`X…` experimental verbs pass).
pub fn parse_line(line: &[u8]) -> Option<Line> {
    let s = std::str::from_utf8(line)
        .ok()?
        .trim_end_matches(['\r', '\n']);
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix("* ") {
        let (verb, rest) = first_word(rest);
        if verb.is_empty()
            || (!verb.bytes().all(|b| b.is_ascii_digit()) && !known(&verb, UNTAGGED_VERBS))
        {
            return None;
        }
        return Some(Line {
            tag: None,
            verb,
            rest,
            kind: Kind::Untagged,
        });
    }
    if let Some(rest) = s.strip_prefix("+ ") {
        return Some(Line {
            tag: None,
            verb: "+".to_string(),
            rest: rest.to_string(),
            kind: Kind::Continuation,
        });
    }
    let (tag, rest) = first_word(s);
    if tag.is_empty() {
        return None;
    }
    let (verb, rest) = first_word(&rest);
    if verb.is_empty() || !known(&verb, COMMAND_VERBS) {
        return None;
    }
    Some(Line {
        tag: Some(tag),
        verb,
        rest,
        kind: Kind::Tagged,
    })
}

fn first_word(s: &str) -> (String, String) {
    let s = s.trim_start();
    match s.find(' ') {
        Some(i) => (s[..i].to_string(), s[i + 1..].to_string()),
        None => (s.to_string(), String::new()),
    }
}

/// Parse a transcript; `None` when empty or a line fails.
pub fn parse(d: &[u8]) -> Option<Vec<Line>> {
    if d.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for line in d.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        out.push(parse_line(line)?);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        let l = parse_line(b"a142 SELECT inbox").unwrap();
        assert_eq!(l.kind, Kind::Tagged);
        assert_eq!(l.tag.as_deref(), Some("a142"));
        assert_eq!(l.verb, "SELECT");
        assert_eq!(l.rest, "inbox");

        let l = parse_line(b"* OK IMAP4rev1 ready").unwrap();
        assert_eq!(l.kind, Kind::Untagged);
        assert_eq!(l.verb, "OK");

        let l = parse_line(b"+ go ahead").unwrap();
        assert_eq!(l.kind, Kind::Continuation);
        assert_eq!(l.rest, "go ahead");

        let l = parse_line(b"a001 OK done").unwrap();
        assert_eq!(l.kind, Kind::Tagged);
        assert_eq!(l.verb, "OK");
    }

    #[test]
    fn rejects() {
        assert!(parse_line(b"").is_none());
        assert!(parse_line(b"tagonly").is_none()); // no verb
        let t = parse(b"* OK ready\r\na1 NOOP\r\na1 OK NOOP done\r\n").unwrap();
        assert_eq!(t.len(), 3);
        assert!(parse(b"tagwithnoverb\r\n").is_none()); // tag but no verb
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse_line(b"the quick brown fox").is_none()); // verb "quick" unknown
        assert!(parse_line(b"a1 FROBNICATE x").is_none()); // not an IMAP command
        assert!(parse_line(b"* FROBNICATE x").is_none()); // untagged keyword unknown
        assert!(parse_line(b"* ").is_none());
        assert!(parse(b"hello world this is not imap at all\n").is_none());
        // Prose that happens to start with a tag-like word still fails.
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        // Registered extensions and `X…` verbs stay accepted.
        assert!(parse_line(b"a2 IDLE").is_some());
        assert!(parse_line(b"a3 X-GM-EXT-1 foo").is_some());
        assert!(parse_line(b"* 23 FETCH (FLAGS ())").is_some());
    }
}
