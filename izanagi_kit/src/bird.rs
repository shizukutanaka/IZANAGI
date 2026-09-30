//! BIRD Internet Routing Daemon configuration census.
//!
//! `bird.conf` is a C-like language: `router id <ip>;`, `log` lines,
//! `protocol <type> <name> { ... }` blocks, `filter`/`function` blocks,
//! `route <net> via <gw>;`, `neighbor <ip> as <n>;`, `area`/`interface`
//! sub-blocks, `table`, `import`/`export`, `include` directives and
//! `key = value;` statements. `parse` counts each class.
//!
//! ```rust
//! let b = concat!(
//!     "router id 192.0.2.1;\n",
//!     "log syslog all;\n",
//!     "protocol kernel {\n",
//!     "    export all;\n",
//!     "}\n",
//!     "protocol static {\n",
//!     "    route 10.0.0.0/24 via 192.0.2.1;\n",
//!     "}\n",
//!     "protocol bgp peer1 {\n",
//!     "    local as 65001;\n",
//!     "    neighbor 192.0.2.2 as 65002;\n",
//!     "}\n",
//!     "filter f1 { if net ~ [ 10.0.0.0/8+ ] then accept; reject; }\n",
//! );
//! let c = izanagi_kit::bird::Bird::parse(b.as_bytes()).unwrap();
//! assert_eq!(c.protocols, 3);
//! assert_eq!(c.filters, 1);
//! ```

/// BIRD configuration census.
#[derive(Debug, Clone)]
pub struct Bird {
    /// `router id`/`log`/`listen`/`watchdog`/`debug`/`timeformat` global lines.
    pub globals: usize,
    /// `protocol <type>`/`template` block headers.
    pub protocols: usize,
    /// `filter <name>`/`function <name>` blocks.
    pub filters: usize,
    /// `route <net> via`/`route6`/`reject`/`reject6` route statements.
    pub routes: usize,
    /// `neighbor`/`local as`/`remote as`/`as`/`hold time`/`keepalive`/`source address` peer statements.
    pub neighbors: usize,
    /// `area`/`interface`/`stubnet`/`networks`/`pattern`/`channel`/`ipv4`/`ipv6` sub-block headers.
    pub blocks: usize,
    /// `import`/`export`/`preference`/`default`/`merge`/`learn`/`persist`/`scan time`/`graceful restart` lines.
    pub options: usize,
    /// `include`/`define`/`eval`/`exit`/`print`/`show`/`configure`/`disable`/`enable` lines.
    pub directives: usize,
    /// `key = value;` assignment statements.
    pub assignments: usize,
}

/// Whether the buffer looks like a BIRD configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("protocol ")
        && (t.contains("router id")
            || t.contains("filter ")
            || t.contains("kernel")
            || t.contains("export ")
            || t.contains("import "))
        && t.contains(';')
}

impl Bird {
    /// Parse a BIRD configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            globals: 0,
            protocols: 0,
            filters: 0,
            routes: 0,
            neighbors: 0,
            blocks: 0,
            options: 0,
            directives: 0,
            assignments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with("//") {
                continue;
            }
            if s.contains('=') && s.ends_with(';') {
                c.assignments += 1;
            }
            if s.starts_with("protocol ") || s.starts_with("template ") {
                c.protocols += 1;
                continue;
            }
            if s.starts_with("filter ") || s.starts_with("function ") {
                c.filters += 1;
                continue;
            }
            if s.starts_with("router id")
                || s.starts_with("log ")
                || s.starts_with("listen ")
                || s.starts_with("watchdog")
                || s.starts_with("debug ")
                || s.starts_with("timeformat")
            {
                c.globals += 1;
                continue;
            }
            if s.starts_with("route ")
                || s.starts_with("route6 ")
                || s.starts_with("reject")
                || s.starts_with("unreachable ")
            {
                c.routes += 1;
                continue;
            }
            if s.starts_with("neighbor")
                || s.starts_with("local ")
                || s.starts_with("remote ")
                || s.starts_with("as ")
                || s.starts_with("hold time")
                || s.starts_with("keepalive")
                || s.starts_with("source address")
                || s.starts_with("next hop")
                || s.starts_with("password ")
            {
                c.neighbors += 1;
                continue;
            }
            if s.starts_with("area")
                || s.starts_with("interface")
                || s.starts_with("stubnet")
                || s.starts_with("networks ")
                || s.starts_with("pattern")
                || s.starts_with("channel")
                || s.starts_with("ipv4")
                || s.starts_with("ipv6")
                || s.starts_with("table")
            {
                c.blocks += 1;
                continue;
            }
            if s.starts_with("import")
                || s.starts_with("export")
                || s.starts_with("preference")
                || s.starts_with("default")
                || s.starts_with("merge")
                || s.starts_with("learn")
                || s.starts_with("persist")
                || s.starts_with("scan time")
                || s.starts_with("graceful")
                || s.starts_with("check link")
                || s.starts_with("ecmp")
                || s.starts_with("import table")
            {
                c.options += 1;
                continue;
            }
            if s.starts_with("include")
                || s.starts_with("define")
                || s.starts_with("eval")
                || s.starts_with("exit")
                || s.starts_with("print")
                || s.starts_with("show")
                || s.starts_with("configure")
                || s.starts_with("disable")
                || s.starts_with("enable")
                || s.starts_with("restart")
                || s.starts_with("reload")
            {
                c.directives += 1;
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
            "router id 192.0.2.1;\n",
            "log syslog all;\n",
            "include \"aux.conf\";\n",
            "protocol kernel {\n",
            "    scan time 20;\n",
            "    export all;\n",
            "}\n",
            "protocol device {\n",
            "}\n",
            "protocol static {\n",
            "    route 10.0.0.0/24 via 192.0.2.1;\n",
            "    route6 fd00::/64 via fd00::1;\n",
            "}\n",
            "protocol ospf v3 {\n",
            "    area 0 {\n",
            "        interface \"eth0\" { cost 10; };\n",
            "    };\n",
            "}\n",
            "protocol bgp peer1 {\n",
            "    local as 65001;\n",
            "    neighbor 192.0.2.2 as 65002;\n",
            "}\n",
            "filter f1 {\n",
            "    if net ~ [ 10.0.0.0/8+ ] then accept;\n",
            "    reject;\n",
            "}\n",
            "define ASN = 65001;\n",
        );
        let c = Bird::parse(b.as_bytes()).unwrap();
        assert_eq!(c.globals, 2);
        assert_eq!(c.protocols, 5);
        assert_eq!(c.filters, 1);
        assert_eq!(c.routes, 3);
        assert_eq!(c.neighbors, 2);
        assert_eq!(c.blocks, 2);
        assert_eq!(c.options, 2);
        assert_eq!(c.directives, 2);
        assert_eq!(c.assignments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Bird::parse(b"foo = 1").is_none());
    }
}
