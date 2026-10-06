//! nullmailer `/etc/nullmailer/*` 設定検出モジュール。
//!
//! nullmailer の `remotes` ファイルは
//! `<host> <proto> [--options]` 形式で、プロトコルは
//! `smtp`/`ssl`/`qmqp`、オプションは `--user`/`--pass`/`--port`/
//! `--auth-login`/`--auth-plain`/`--starttls`/`--insecure`/`--ssl`/
//! `--x509certfile`/`--x509keyfile`/`--x509cafile`/`--x509crlfile`/
//! `--x509fmtder`/`--source`/`--helo`/`--auth` 等。
//! 単一値ファイル(`adminaddr`/`defaultdomain`/`defaulthost`/
//! `idhost`/`helohost`/`sendtimeout`/`pausetime`/`remotes`)は
//! ファイル名なしでは判別不能なため remotes 形式のみ検出する。
//!
//! ```
//! let b = b"smtp.gmail.com smtp --port=587 --starttls --user=me --pass=secret\n\
//!           mail.example.com ssl --port=465 --auth-login\n";
//! let c = izanagi_kit::nullmailerconf::parse(b);
//! assert!(izanagi_kit::nullmailerconf::detect(b));
//! assert_eq!(c.remotes, 2);
//! ```

const PROTOS: &[&str] = &["smtp", "ssl", "qmqp"];

const OPTS: &[&str] = &[
    "--auth",
    "--auth-login",
    "--auth-plain",
    "--helo",
    "--insecure",
    "--pass",
    "--port",
    "--source",
    "--ssl",
    "--starttls",
    "--user",
    "--x509cafile",
    "--x509certfile",
    "--x509crlfile",
    "--x509fmtder",
    "--x509keyfile",
];

fn is_remote(t: &str) -> bool {
    let mut it = t.split_whitespace();
    let Some(host) = it.next() else {
        return false;
    };
    if host.is_empty()
        || !host.bytes().all(|c| {
            c.is_ascii_alphanumeric()
                || c == b'.'
                || c == b'-'
                || c == b'['
                || c == b']'
                || c == b':'
        })
    {
        return false;
    }
    it.next().is_some_and(|p| PROTOS.contains(&p))
        && it.any(|o| {
            OPTS.iter()
                .any(|k| o == *k || o.starts_with(&format!("{k}=")))
        })
}

/// `b` が nullmailer remotes に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut remotes = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_remote(tr) {
            remotes += 1;
        }
    }
    remotes >= 1
}

/// nullmailer remotes の統計。
#[derive(Debug, Default, Clone)]
pub struct NullmailerConf {
    /// remote 行数。
    pub remotes: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を nullmailer remotes として統計する。
pub fn parse(b: &[u8]) -> NullmailerConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = NullmailerConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_remote(tr) {
            c.remotes += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"smtp.x.com smtp --user=u --pass=p\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.remotes, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"smtp.x.com smtp\n"));
        assert!(!detect(b"host proto\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
        assert!(!detect(b"postmaster@x.com\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# smtp.x.com smtp --user=u\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.remotes, 0);
    }
}
