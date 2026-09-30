//! OpenBSD `bgpd.conf` (OpenBGPD) configuration census.
//!
//! An OpenBGPD config starts with `AS <asn>`/`router-id <ip>`/`listen on`,
//! `socket`, `fib-update`, `log` and `network <prefix>` lines, then
//! `group "name" { remote-as <n> neighbor <ip> { descr "x" } }` blocks and
//! filter rules (`allow`/`deny`/`match` `from`/`to`/`community`), `announce`
//! and `include`. `parse` counts each class.
//!
//! ```rust
//! let b = concat!(
//!     "AS 65001\n",
//!     "router-id 192.0.2.1\n",
//!     "fib-update yes\n",
//!     "network 10.0.0.0/24\n",
//!     "group \"peers\" {\n",
//!     "    remote-as 65002\n",
//!     "    neighbor 10.0.0.2 {\n",
//!     "        descr \"p1\"\n",
//!     "    }\n",
//!     "    announce all\n",
//!     "}\n",
//!     "deny from any\n",
//! );
//! let c = izanagi_kit::openbgpd::Openbgpd::parse(b.as_bytes()).unwrap();
//! assert_eq!(c.groups, 1);
//! assert_eq!(c.neighbors, 4);
//! ```

/// OpenBGPD configuration census.
#[derive(Debug, Clone)]
pub struct Openbgpd {
    /// `AS`/`router-id`/`listen on`/`socket`/`fib-update`/`log`/`rd`/`nexthop`/`mrt`/`connect-retry`/`holdtime`/`staletime`/`dump`/`flow`/`knf`/`conffile`/`include` global lines.
    pub globals: usize,
    /// `network <prefix>`/`network inet6` lines.
    pub networks: usize,
    /// `group "name" {`/`group {` block headers.
    pub groups: usize,
    /// `neighbor <ip>`/`peer`/`remote-as`/`local-as`/`descr`/`multihop`/`ttl-security`/`announce`/`enforce`/`export`/`import`/`depend on`/`ipsec`/`tcp md5sig`/`passive`/`down`/`capabilities`/`default `/`community` lines.
    pub neighbors: usize,
    /// `allow`/`deny`/`match` filter rules.
    pub filters: usize,
    /// `from`/`to`/`prefixlen`/`source-as`/`transit-as`/`peer-as`/`max-prefix`/`inet`/`inet6`/`large-community`/`ovs`/`ext-community`/`set ` filter atoms.
    pub atoms: usize,
    /// `include`/`rde`/`socket`/`prefix-set`/`as-set`/`roa-set`/`origin-set` lines.
    pub extras: usize,
}

/// Whether the buffer looks like an OpenBGPD configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("router-id") || t.contains("fib-update") || t.contains("AS "))
        && (t.contains("neighbor")
            || t.contains("announce")
            || t.contains("remote-as")
            || t.contains("fib-update"))
}

impl Openbgpd {
    /// Parse an OpenBGPD configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            globals: 0,
            networks: 0,
            groups: 0,
            neighbors: 0,
            filters: 0,
            atoms: 0,
            extras: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s == "}" || s == "{" {
                continue;
            }
            if s.starts_with("AS ")
                || s.starts_with("router-id")
                || s.starts_with("listen ")
                || s.starts_with("socket")
                || s.starts_with("fib-")
                || s.starts_with("log ")
                || s.starts_with("rd ")
                || s.starts_with("nexthop")
                || s.starts_with("mrt")
                || s.starts_with("connect-")
                || s.starts_with("holdtime")
                || s.starts_with("staletime")
                || s.starts_with("dump")
                || s.starts_with("flow")
                || s.starts_with("min ")
            {
                c.globals += 1;
                continue;
            }
            if s.starts_with("network") {
                c.networks += 1;
                continue;
            }
            if s.starts_with("group") {
                c.groups += 1;
                continue;
            }
            if s.starts_with("include")
                || s.starts_with("rde")
                || s.starts_with("prefix-set")
                || s.starts_with("as-set")
                || s.starts_with("roa-set")
                || s.starts_with("origin-set")
                || s.starts_with("filter-")
                || s.starts_with("rpk")
            {
                c.extras += 1;
                continue;
            }
            if s.starts_with("allow") || s.starts_with("deny") || s.starts_with("match") {
                c.filters += 1;
                for part in s.split_whitespace().skip(1) {
                    if [
                        "from",
                        "to",
                        "prefixlen",
                        "source-as",
                        "transit-as",
                        "peer-as",
                        "max-prefix",
                        "inet",
                        "inet6",
                        "community",
                        "large-community",
                        "ext-community",
                        "ovs",
                        "set",
                    ]
                    .contains(&part)
                    {
                        c.atoms += 1;
                    }
                }
                continue;
            }
            if s.starts_with("from ")
                || s.starts_with("to ")
                || s.starts_with("set ")
                || s.starts_with("quick ")
            {
                c.atoms += 1;
                continue;
            }
            if s.starts_with("neighbor")
                || s.starts_with("peer")
                || s.starts_with("remote-as")
                || s.starts_with("local-as")
                || s.starts_with("descr")
                || s.starts_with("multihop")
                || s.starts_with("ttl-security")
                || s.starts_with("announce")
                || s.starts_with("enforce")
                || s.starts_with("export")
                || s.starts_with("import")
                || s.starts_with("depend")
                || s.starts_with("ipsec")
                || s.starts_with("tcp ")
                || s.starts_with("passive")
                || s.starts_with("down")
                || s.starts_with("capabilities")
                || s.starts_with("default")
                || s.starts_with("community")
                || s.starts_with("localpref")
                || s.starts_with("med")
                || s.starts_with("weight")
            {
                c.neighbors += 1;
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let b = concat!(
            "AS 65001\n",
            "router-id 192.0.2.1\n",
            "listen on 192.0.2.1\n",
            "fib-update yes\n",
            "log updates\n",
            "network 10.0.0.0/24\n",
            "network inet6 connected\n",
            "group \"peers\" {\n",
            "    remote-as 65002\n",
            "    neighbor 10.0.0.2 {\n",
            "        descr \"p1\"\n",
            "    }\n",
            "    neighbor 10.0.0.3\n",
            "    announce all\n",
            "}\n",
            "deny from any\n",
            "allow from any prefixlen 8 - 24\n",
            "match community 65001:1\n",
            "include \"/etc/bgpd.local\"\n",
        );
        let c = Openbgpd::parse(b.as_bytes()).unwrap();
        assert_eq!(c.globals, 5);
        assert_eq!(c.networks, 2);
        assert_eq!(c.groups, 1);
        assert_eq!(c.neighbors, 5);
        assert_eq!(c.filters, 3);
        assert_eq!(c.atoms, 4);
        assert_eq!(c.extras, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Openbgpd::parse(b"foo = 1").is_none());
    }
}
