//! IRC protocol messages (RFC 1459/2812):
//! `[:prefix] COMMAND [param …] [:trailing]` — the last parameter may
//! be introduced by `:` and contain spaces. `parse_line` reads one
//! message; `parse` reads a transcript.
//!
//! ```
//! use izanagi_kit::irc::parse_line;
//! let m = parse_line(b":nick!u@h PRIVMSG #chan :hello there").unwrap();
//! assert_eq!(m.prefix.as_deref(), Some("nick!u@h"));
//! assert_eq!(m.command, "PRIVMSG");
//! assert_eq!(m.params, vec!["#chan".to_string()]);
//! assert_eq!(m.trailing.as_deref(), Some("hello there"));
//! ```

use std::string::String;
use std::vec::Vec;

/// One IRC message.
#[derive(Clone, Debug)]
pub struct Irc {
    /// Optional `:prefix` (nick!user@host or server name).
    pub prefix: Option<String>,
    /// Command word or 3-digit numeric, uppercased.
    pub command: String,
    /// Middle parameters (before the trailing one).
    pub params: Vec<String>,
    /// Optional trailing parameter (after ` :`).
    pub trailing: Option<String>,
}

/// Named commands: RFC 1459/2812 core plus common IRCv3 and vendor
/// commands. Three-digit numerics (`001`–`999`) are always accepted.
const COMMANDS: &[&str] = &[
    "ACCOUNT",
    "ADMIN",
    "AUTHENTICATE",
    "AWAY",
    "BATCH",
    "CAP",
    "CHATHISTORY",
    "CHGHOST",
    "CLEAR",
    "CONNECT",
    "DIE",
    "ERROR",
    "HELP",
    "INFO",
    "INVITE",
    "ISON",
    "JOIN",
    "KICK",
    "KILL",
    "LINKS",
    "LIST",
    "LUSERS",
    "MARKREAD",
    "MODE",
    "MONITOR",
    "MOTD",
    "NAMES",
    "NICK",
    "NOTICE",
    "OPER",
    "PART",
    "PASS",
    "PING",
    "PONG",
    "PRIVMSG",
    "QUIT",
    "REHASH",
    "RESTART",
    "RULES",
    "SERVICE",
    "SERVLIST",
    "SETNAME",
    "SQUERY",
    "SQUIT",
    "STATS",
    "SUMMON",
    "TAGMSG",
    "TIME",
    "TOPIC",
    "TRACE",
    "USER",
    "USERHOST",
    "USERS",
    "VERSION",
    "WALLOPS",
    "WEBIRC",
    "WHO",
    "WHOIS",
    "WHOWAS",
];

/// The RFC command token is `1*letter` (must be a registered command)
/// or `3digit` (numeric reply).
fn valid_command(cmd: &str) -> bool {
    if cmd.len() == 3 && cmd.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    cmd.bytes().all(|b| b.is_ascii_alphabetic())
        && COMMANDS.iter().any(|c| c.eq_ignore_ascii_case(cmd))
}

/// Parse one IRC line (no trailing CR/LF); `None` on empty, a missing
/// command, or a command outside the registered IRC command set.
pub fn parse_line(line: &[u8]) -> Option<Irc> {
    let s = std::str::from_utf8(line)
        .ok()?
        .trim_end_matches(['\r', '\n']);
    if s.is_empty() {
        return None;
    }
    let (prefix, rest) = if let Some(r) = s.strip_prefix(':') {
        let i = r.find(' ')?; // prefix but no command
        (Some(r[..i].to_string()), r[i + 1..].trim_start())
    } else {
        (None, s)
    };
    // Command = next word
    let (command, mut rest) = match rest.find(' ') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    if command.is_empty() || !valid_command(command) {
        return None;
    }
    let mut params = Vec::new();
    let mut trailing = None;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if let Some(t) = rest.strip_prefix(':') {
            trailing = Some(t.to_string());
            break;
        }
        match rest.find(' ') {
            Some(i) => {
                params.push(rest[..i].to_string());
                rest = &rest[i + 1..];
            }
            None => {
                params.push(rest.to_string());
                break;
            }
        }
    }
    Some(Irc {
        prefix,
        command: command.to_uppercase(),
        params,
        trailing,
    })
}

/// Parse a transcript; `None` when empty or a line fails.
pub fn parse(d: &[u8]) -> Option<Vec<Irc>> {
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
    fn full() {
        let m = parse_line(b":srv 001 nick :Welcome").unwrap();
        assert_eq!(m.prefix.as_deref(), Some("srv"));
        assert_eq!(m.command, "001");
        assert_eq!(m.params, vec!["nick".to_string()]);
        assert_eq!(m.trailing.as_deref(), Some("Welcome"));

        let m = parse_line(b"PING :abc").unwrap();
        assert_eq!(m.prefix, None);
        assert_eq!(m.command, "PING");
        assert_eq!(m.trailing.as_deref(), Some("abc"));

        let m = parse_line(b"NICK newname").unwrap();
        assert_eq!(m.params, vec!["newname".to_string()]);
        assert_eq!(m.trailing, None);

        let m = parse_line(b"MODE #c +o nick").unwrap();
        assert_eq!(
            m.params,
            vec!["#c".to_string(), "+o".to_string(), "nick".to_string()]
        );
    }

    #[test]
    fn rejects() {
        assert!(parse_line(b"").is_none());
        assert!(parse_line(b":onlyprefix").is_none());
        assert!(parse(b"").is_none());
        let t = parse(b"PING :a\r\nPONG :a\r\n").unwrap();
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse_line(b"hello world").is_none()); // HELLO not a command
        assert!(parse_line(b"the quick brown fox").is_none());
        assert!(parse_line(b"abc123 mixed").is_none()); // not alpha, not 3digit
        assert!(parse(b"hello world this is not irc at all\n").is_none());
        // Registered commands and numerics stay accepted.
        assert!(parse_line(b"CAP LS 302").is_some());
        assert!(parse_line(b":srv 001 nick :hi").is_some());
        assert!(parse_line(b"privmsg #c :x").is_some());
    }
}
