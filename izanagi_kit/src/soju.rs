//! `soju.conf` 検出モジュール。
//!
//! soju (IRC バウンサ) の設定はフラットなディレクティブ行で、
//! `listen`、`db`、`hostname`、`title`、`motd`、`tls`、`log` 等が
//! 使われる。
//!
//! ```
//! let b = br#"listen irc+insecure://0.0.0.0:6667
//! listen unix+admin://admin
//! db sqlite3 /var/lib/soju/main.db
//! hostname irc.example.net
//! title My Bouncer
//! tls /etc/ssl/cert.pem /etc/ssl/key.pem
//! "#;
//! let c = izanagi_kit::soju::parse(b);
//! assert!(izanagi_kit::soju::detect(b));
//! assert_eq!(c.directives, 6);
//! ```

const DIRECTIVES: &[&str] = &[
    "accept-user-ip",
    "db",
    "filename",
    "hostname",
    "http-origin",
    "listen",
    "log",
    "max-user-networks",
    "motd",
    "offline-store",
    "title",
    "tls",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with(';')
}

fn dir_line(t: &str) -> bool {
    DIRECTIVES
        .iter()
        .any(|d| t == *d || t.starts_with(&format!("{d} ")))
}

/// `b` が soju.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dirs = 0usize;
    let mut saw_listen = false;
    let mut saw_db = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if dir_line(tr) {
            dirs += 1;
            if tr == "listen" || tr.starts_with("listen ") {
                saw_listen = true;
            } else if tr == "db" || tr.starts_with("db ") {
                saw_db = true;
            }
        }
    }
    (saw_listen && saw_db) || dirs >= 3
}

/// soju.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct SojuConf {
    /// 既知ディレクティブ行数。
    pub directives: usize,
    /// `listen` 行数。
    pub listens: usize,
    /// `db` 行数。
    pub dbs: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を soju.conf として統計する。
pub fn parse(b: &[u8]) -> SojuConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SojuConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if dir_line(tr) {
            c.directives += 1;
            if tr == "listen" || tr.starts_with("listen ") {
                c.listens += 1;
            } else if tr == "db" || tr.starts_with("db ") {
                c.dbs += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"listen irc+insecure://0.0.0.0:6667
db sqlite3 /var/lib/soju/main.db
hostname irc.example.net
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
        assert_eq!(c.listens, 1);
        assert_eq!(c.dbs, 1);
    }

    #[test]
    fn detects_multi_listen() {
        let b = br#"listen irc+insecure://0.0.0.0:6667
listen unix+admin://admin
listen wss://irc.example.net:443
db sqlite3 main.db
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.listens, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"listen 80\n"));
        assert!(!detect(b"db foo\n"));
        assert!(!detect(b"hostname x\ntitle y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}
