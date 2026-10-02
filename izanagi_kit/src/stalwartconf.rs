//! Stalwart Mail Server `config.toml` 式ドットキー設定の検出と構造カウント。
//!
//! `server.*`/`storage.*`/`store.*`/`tracer.*`/`webdav.*`/`jmap.*`/`imap.*`/
//! `smtp.*`/`autoconfig.*`/`spam.*`/`acme.*` 等のドットプレフィックス階層を
//! ファミリ別に分類する。標準 TOML テーブル形式 `[jmap.protocol]` と
//! フラット `key = value` 両方を扱う。
//!
//! ```
//! let c = izanagi_kit::stalwartconf::parse(
//!     b"[server.listener.http]\nbind = \"127.0.0.1:8080\"\nprotocol = \"http\"\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::stalwartconf::detect(
//!     b"server.hostname = \"mail.example.com\"\nserver.key = \"v1\"\n"));
//! ```

/// 既知のキー・先頭プレフィックス(ファミリ)。
const FAMILIES: &[&str] = &[
    "acme",
    "asn",
    "auth",
    "authentication",
    "autoconfig",
    "autodiscover",
    "cluster",
    "config",
    "directory",
    "ent",
    "eval",
    "fail2ban",
    "imap",
    "jmap",
    "managesieve",
    "milter",
    "oauth",
    "pusher",
    "queue",
    "remote",
    "resolver",
    "session",
    "settings",
    "sieve",
    "smtp",
    "spam-filter",
    "spam_filter",
    "spam",
    "spamtrap",
    "storage",
    "store",
    "tracer",
    "troubleshoot",
    "webdav",
    "worker",
    "cache",
    "lookup",
    "metrics",
    "server",
];
/// 葉側既知キー(`key`/`value` 単独 key としても出る代表)。
const LEAF_HINTS: &[&str] = &[
    "account-id",
    "acme",
    "address",
    "admin",
    "aes-key",
    "aes_iv",
    "allow-from",
    "allow-lists",
    "assignments",
    "auth",
    "bind",
    "blob-store",
    "blob_store",
    "bucket",
    "cache",
    "cert",
    "command",
    "connect-timeout",
    "contact",
    "days",
    "dkim",
    "dns",
    "domain",
    "directory",
    "dsn",
    "ec",
    "enable",
    "hostname",
    "id",
    "key",
    "keys",
    "language",
    "listener",
    "management",
    "maxage",
    "message-id",
    "methods",
    "name",
    "oauth",
    "pass",
    "path",
    "policy",
    "port",
    "protocol",
    "query",
    "report",
    "scores",
    "secret",
    "sender",
    "server",
    "shared-secret",
    "signature",
    "signer",
    "speed",
    "store",
    "strategy",
    "subject",
    "template",
    "test",
    "tls",
    "token",
    "ttl",
    "type",
    "url",
    "user",
    "verify",
];

/// ファミリ別カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[a.b]` TOML テーブル/配列ヘッダ。
    pub sections: usize,
    /// `server.*`/`imap.*` 等ドットキー代入。
    pub options: usize,
    /// `#`/`;`/`//` コメント行。
    pub comments: usize,
    /// ドット無しの裸キー代入。
    pub plain: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// ドットキーの先頭ファミリが既知かどうか。
fn family_of(key: &str) -> Option<&str> {
    key.split('.')
        .next()
        .and_then(|f| FAMILIES.iter().find(|x| x == &&f).copied())
}

/// 葉キーが既知ヒントかどうか(最後のドット後)。
fn leaf_known(key: &str) -> bool {
    key.rsplit('.').next().is_some_and(|l| {
        LEAF_HINTS.contains(&l) || l.starts_with(|c: char| c.is_ascii_alphabetic())
    })
}

/// b が Stalwart 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut opts = 0;
    for line in text.lines() {
        let t = line.trim();
        if (t.starts_with('[') && t.ends_with(']')) || (t.starts_with("[[") && t.ends_with("]]")) {
            let inner = t.trim_matches(|c| c == '[' || c == ']');
            if family_of(inner).is_some() {
                secs += 1;
            }
        } else if t.find('=').is_some_and(|p| {
            let k = t[..p].trim();
            family_of(k).is_some() && leaf_known(k)
        }) {
            opts += 1;
        }
    }
    secs + opts >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        plain: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && (t.ends_with(']') || t.ends_with("]]")) {
            c.sections += 1;
            continue;
        }
        if let Some(p) = t.find('=') {
            let k = t[..p].trim();
            if k.contains('.') && family_of(k).is_some() {
                c.options += 1;
            } else {
                c.plain += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# stalwart\n[server]\nhostname = \"mail.example.com\"\n\n[server.listener.http]\nbind = \"127.0.0.1:8080\"\nprotocol = \"http\"\n\n[jmap]\ndirectory = \"sql\"\n";

    #[test]
    fn stalwartconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 0);
        assert_eq!(c.plain, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn dotted_flat() {
        let c = parse(b"server.hostname = \"mail\"\nserver.key = \"v\"\n").unwrap();
        assert_eq!(c.options, 2);
        assert!(detect(b"server.hostname = \"mail\"\nserver.key = \"v\"\n"));
    }

    #[test]
    fn not_stalwart() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(b"foo = bar\n"));
    }
}
