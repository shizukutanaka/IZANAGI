//! Icecast `icecast.xml` census.
//!
//! icecast.xml is a single-root XML file: `<icecast>` holds
//! `<location>`/`<admin>`/`<hostname>`/`<fileserve>`, a `<limits>`
//! block (`clients`/`sources`/`queue-size`/`burst-size`/
//! `client-timeout`/`header-timeout`/`source-timeout`/
//! `burst-on-connect`), `<listen-socket>` entries
//! (`port`/`bind-address`/`ssl`), `<master>` relaying,
//! `<relay>` blocks (`server`/`port`/`mount`/`local-mount`),
//! `<mount>` blocks (`mount-name`/`username`/`password`/
//! `max-listeners`/`dump-file`/`intro`/`fallback-mount`/
//! `fallback-override`/`hidden`/`public`/`authentication`
//! (`type`), `<auth>`-ish `<authentication>` (`source-password`/
//! `admin-user`/`admin-password`/`relays-on-demand`), `<paths>`
//! (`logdir`/`webroot`/`adminroot`/`alias`), `<logging>`
//! (`accesslog`/`errorlog`/`loglevel`/`logsize`/`logarchive`),
//! and `<security>` (`chroot`/`changeowner` (`user`/`group`)).
//!
//! ```rust
//! let c = izanagi_kit::icecast::Icecast::parse(
//!     b"<icecast><location>Earth</location><admin>root</admin></icecast>",
//! ).unwrap();
//! assert_eq!(c.elements, 3);
//! ```

/// icecast.xml census.
#[derive(Debug, Clone)]
pub struct Icecast {
    /// Total element tags (open or self-closing).
    pub elements: usize,
    /// `<listen-socket>` blocks.
    pub sockets: usize,
    /// `<mount>` blocks.
    pub mounts: usize,
    /// `<relay>` blocks.
    pub relays: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

const KNOWN: &[&str] = &[
    "location",
    "admin",
    "hostname",
    "fileserve",
    "server-id",
    "limits",
    "clients",
    "sources",
    "queue-size",
    "burst-size",
    "client-timeout",
    "header-timeout",
    "source-timeout",
    "burst-on-connect",
    "listen-socket",
    "port",
    "bind-address",
    "ssl",
    "master",
    "master-server",
    "master-port",
    "master-update-interval",
    "master-username",
    "master-password",
    "relay",
    "server",
    "local-mount",
    "mount",
    "mount-name",
    "username",
    "password",
    "max-listeners",
    "dump-file",
    "intro",
    "fallback-mount",
    "fallback-override",
    "fallback-when-full",
    "charset",
    "hidden",
    "public",
    "authentication",
    "type",
    "source-password",
    "admin-user",
    "admin-password",
    "relays-on-demand",
    "paths",
    "logdir",
    "webroot",
    "adminroot",
    "alias",
    "ssl-certificate",
    "ssl-allowed-ciphers",
    "x-forwarded-for",
    "deny-ip",
    "allow-ip",
    "logging",
    "accesslog",
    "errorlog",
    "loglevel",
    "logsize",
    "logarchive",
    "security",
    "chroot",
    "changeowner",
    "user",
    "group",
];

fn tag_name(tok: &str) -> &str {
    tok.trim_start_matches('/')
        .trim_start_matches('?')
        .trim_start_matches('!')
        .split([' ', '/', '>'])
        .next()
        .unwrap_or("")
}

/// Whether the buffer looks like icecast.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<icecast>")
        && KNOWN
            .iter()
            .filter(|k| t.contains(&format!("<{k}>")) || t.contains(&format!("<{k} ")))
            .count()
            >= 2
}

impl Icecast {
    /// Parse icecast.xml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            elements: 0,
            sockets: 0,
            mounts: 0,
            relays: 0,
            comments: 0,
        };
        let mut rest = t;
        while let Some(i) = rest.find('<') {
            let after = &rest[i + 1..];
            if after.starts_with("!--") {
                if let Some(j) = after.find("-->") {
                    c.comments += 1;
                    rest = &after[j + 3..];
                    continue;
                }
                break;
            }
            let Some(end) = after.find('>') else { break };
            let tok = &after[..end];
            let name = tag_name(tok);
            if !name.is_empty()
                && !tok.starts_with('/')
                && !tok.starts_with('?')
                && !tok.starts_with('!')
            {
                c.elements += 1;
                match name {
                    "listen-socket" => c.sockets += 1,
                    "mount" => c.mounts += 1,
                    "relay" => c.relays += 1,
                    _ => {}
                }
            }
            rest = &after[end + 1..];
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_icecast_xml() {
        let b = concat!(
            "<icecast>\n",
            "<location>Earth</location>\n",
            "<admin>root@example.com</admin>\n",
            "<listen-socket><port>8000</port></listen-socket>\n",
            "<authentication><source-password>x</source-password></authentication>\n",
            "<mount><mount-name>/live</mount-name></mount>\n",
            "<paths><logdir>/var/log/icecast</logdir></paths>\n",
            "<!-- done -->\n",
            "</icecast>\n",
        );
        let c = Icecast::parse(b.as_bytes()).unwrap();
        assert_eq!(c.elements, 11);
        assert_eq!(c.sockets, 1);
        assert_eq!(c.mounts, 1);
        assert_eq!(c.relays, 0);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Icecast::parse(b"<foo><bar/></foo>").is_none());
    }
}
