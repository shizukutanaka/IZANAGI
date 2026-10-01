//! fetchmail `.fetchmailrc` の検出・カウント。
//!
//! `poll host`/`server`/`skip`/`defaults`/`set` キーワードと
//! `user "u"`/`password "p"`/`proto pop3`/`is "me" here` 等のオプション語。
//!
//! ```
//! let cfg = b"set daemon 300\nset postmaster \"me\"\n\n\
//!             poll mail.example.com proto pop3 user \"jsmith\" password \"xx\" is \"jay\" here ssl\n\
//!             skip mail2.example.com proto imap\n\
//!             defaults proto imap timeout 60\n";
//! assert!(izanagi_kit::fetchmailconf::detect(cfg));
//! let c = izanagi_kit::fetchmailconf::parse(cfg).unwrap();
//! assert_eq!(c.poll_entries, 1);
//! assert_eq!(c.set_entries, 2);
//! assert_eq!(c.skip_entries, 1);
//! ```

/// fetchmailrc の先頭語/キーワード。
const HEADS: &[&str] = &[
    "poll",
    "server",
    "skip",
    "defaults",
    "set",
    "via",
    "aka",
    "localdomains",
];

/// オプション語 (行中で検出)。
const OPTION_WORDS: &[&str] = &[
    "proto",
    "protocol",
    "user",
    "username",
    "password",
    "pass",
    "is",
    "here",
    "ssl",
    "sslfingerprint",
    "sslproto",
    "sslcertck",
    "sslcertfile",
    "sslkey",
    "timeout",
    "interval",
    "dns",
    "checkalias",
    "fetchall",
    "keep",
    "flush",
    "limit",
    "fetchlimit",
    "fetchsizelimit",
    "batchlimit",
    "expunge",
    "uidl",
    "auth",
    "mda",
    "bsmtp",
    "lmtp",
    "smtphost",
    "smtpaddress",
    "smtpname",
    "antispam",
    "no rewrite",
    "envelope",
    "qvirtual",
    "tracepolls",
    "netsec",
    "principal",
    "esmtpname",
    "esmtppassword",
    "preconnect",
    "postconnect",
    "monitor",
    "plugin",
    "plugout",
    "interface",
    "invisible",
    "showdots",
    "service",
    "port",
    "foldernames",
    "mimedecode",
    "idle",
    "warnings",
    "logfile",
    "idfile",
    "postmaster",
    "bouncemail",
    "spambounce",
    "softbounce",
    "properties",
    "smtphunt",
    "fetchdomains",
    "smtpaddress",
    "daemon",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `poll <host>` 行数。
    pub poll_entries: usize,
    /// `server` 行数 (poll の別名形式)。
    pub server_entries: usize,
    /// `skip <host>` 行数。
    pub skip_entries: usize,
    /// `defaults` 行数。
    pub default_entries: usize,
    /// `set <option>` 行数。
    pub set_entries: usize,
    /// 既知オプション語の出現数 (行内トークン単位)。
    pub option_words: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// `b` が `.fetchmailrc` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.poll_entries + c.server_entries + c.skip_entries + c.default_entries + c.set_entries >= 2
        && c.option_words >= 2
}

/// `b` を `.fetchmailrc` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        poll_entries: 0,
        server_entries: 0,
        skip_entries: 0,
        default_entries: 0,
        set_entries: 0,
        option_words: 0,
        comments: 0,
    };
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let mut it = s.split([' ', '\t']);
        let head = it.next().unwrap_or("");
        match head {
            "poll" => c.poll_entries += 1,
            "server" => c.server_entries += 1,
            "skip" => c.skip_entries += 1,
            "defaults" => c.default_entries += 1,
            "set" => c.set_entries += 1,
            _ if HEADS.contains(&head) => {}
            _ => continue,
        }
        for w in it {
            let w = w.trim_matches('"').trim_matches('\'');
            if OPTION_WORDS.contains(&w) {
                c.option_words += 1;
            }
        }
    }
    if c.poll_entries + c.server_entries + c.skip_entries + c.default_entries + c.set_entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_fetchmailrc() {
        let cfg = b"# fetchmail\nset daemon 600\nset logfile /var/log/fetchmail.log\n\n\
                    poll pop.provider.net proto pop3 user \"alice\" password \"s3cret\" is \"al\" here\n\
                    poll imap.provider.net proto imap user \"bob\" password \"yy\" ssl keep\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.poll_entries, 2);
        assert_eq!(c.set_entries, 2);
    }

    #[test]
    fn rejects_shell() {
        assert!(!detect(b"echo hello\nls -la\ncat file\n"));
    }
}
