//! keepalived configuration census.
//!
//! `keepalived.conf` is braced blocks: `global_defs`, `vrrp_script`,
//! `vrrp_instance`, `vrrp_sync_group`, `virtual_server`, `real_server`,
//! `authentication`, `virtual_ipaddress`, `virtual_routes`, `track_script`,
//! `track_interface`, `static_*`, `snmp_*`/`notification_*` plus bare
//! IP/CIDR value lines inside virtual blocks and `key value` options
//! (`state MASTER`, `priority 100`, `advert_int 1`, `auth_type`,
//! `lb_algo`, `delay_loop`). `parse` counts blocks and option classes.
//!
//! ```rust
//! let k = concat!(
//!     "global_defs {\n",
//!     "   router_id LVS1\n",
//!     "}\n",
//!     "vrrp_instance VI_1 {\n",
//!     "    state MASTER\n",
//!     "    interface eth0\n",
//!     "    virtual_router_id 51\n",
//!     "    priority 100\n",
//!     "    virtual_ipaddress {\n",
//!     "        192.168.0.100\n",
//!     "    }\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::keepalived::Keepalived::parse(k.as_bytes()).unwrap();
//! assert_eq!(c.instances, 1);
//! assert_eq!(c.ips, 1);
//! ```

const BLOCKS: &[&str] = &[
    "global_defs",
    "vrrp_script",
    "vrrp_instance",
    "vrrp_sync_group",
    "vrrp_track_process",
    "virtual_server",
    "real_server",
    "authentication",
    "virtual_ipaddress",
    "virtual_ipaddress_excluded",
    "virtual_routes",
    "virtual_rules",
    "track_script",
    "track_interface",
    "track_file",
    "static_ipaddress",
    "static_routes",
    "static_rules",
    "snmp",
    "notification_email",
    "notification_email_from",
    "smtp_server",
    "smtp_connect_timeout",
    "bfd_instance",
    "checker",
    "tcp_check",
    "http_get",
    "ssl_get",
    "smtp_check",
    "dns_check",
    "misc_check",
];

const OPTIONS: &[&str] = &[
    "state",
    "interface",
    "virtual_router_id",
    "priority",
    "advert_int",
    "smtp_alert",
    "nopreempt",
    "preempt_delay",
    "garp_master_delay",
    "auth_type",
    "auth_pass",
    "weight",
    "lb_algo",
    "lb_kind",
    "delay_loop",
    "script",
    "interval",
    "fall",
    "rise",
    "timeout",
    "user",
    "init_fail",
    "notify_master",
    "notify_backup",
    "notify_fault",
    "notify_stop",
    "notify",
    "dont_track_primary",
    "accept",
    "promote_secondaries",
    "router_id",
    "vrrp_skip_check_adv_addr",
    "vrrp_strict",
    "vrrp_garp_interval",
    "vrrp_gna_interval",
    "enable_script_security",
    "script_user",
    "max_auto_priority",
    "dynamic_interfaces",
    "unicast_src_ip",
    "unicast_peer",
    "native_ipv6",
    "lvs_sync_daemon",
    "lvs_flush",
    "allow_if_changes",
    "vrrp_version",
    "vrrp_iptables",
    "vrrp_check_unicast_src",
    "vrrp_no_swap",
    "checker_no_swap",
    "connect_timeout",
    "connect_ip",
    "bindto",
    "use_ssl",
    "retry",
    "delay_before_retry",
    "nb_get_retry",
];

/// keepalived configuration census.
#[derive(Debug, Clone)]
pub struct Keepalived {
    /// `vrrp_instance` block headers.
    pub instances: usize,
    /// `vrrp_script`/`vrrp_track_process`/`checker` blocks.
    pub scripts: usize,
    /// `virtual_server` blocks.
    pub servers: usize,
    /// `real_server` blocks.
    pub reals: usize,
    /// Other recognized block headers (`global_defs`/auth/virtual_*/track_*/static_*/snmp/notification_*).
    pub blocks: usize,
    /// `key value`/`key` option lines inside blocks.
    pub options: usize,
    /// Bare IP/CIDR literal lines (inside virtual_ipaddress/routes/unicast lists).
    pub ips: usize,
    /// `{`/`}` brace lines.
    pub braces: usize,
}

fn is_ipish(s: &str) -> bool {
    !s.is_empty()
        && s.split_whitespace().next().is_some_and(|h| {
            h.chars()
                .all(|ch| ch.is_ascii_hexdigit() || ch == '.' || ch == ':' || ch == '/')
                && h.chars().any(|ch| ch == '.' || ch == ':' || ch == '/')
        })
}

/// Whether the buffer looks like a keepalived configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("vrrp_instance")
        || t.contains("global_defs")
        || t.contains("vrrp_script")
        || (t.contains("virtual_server") && t.contains("real_server"))
}

impl Keepalived {
    /// Parse a keepalived configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            instances: 0,
            scripts: 0,
            servers: 0,
            reals: 0,
            blocks: 0,
            options: 0,
            ips: 0,
            braces: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with('!') {
                continue;
            }
            if s == "{" || s == "}" || s == "};" {
                c.braces += 1;
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if s.starts_with("vrrp_instance") {
                c.instances += 1;
                continue;
            }
            if s.starts_with("vrrp_script")
                || s.starts_with("vrrp_track_process")
                || s.starts_with("checker")
            {
                c.scripts += 1;
                continue;
            }
            if s.starts_with("virtual_server") {
                c.servers += 1;
                continue;
            }
            if s.starts_with("real_server") {
                c.reals += 1;
                continue;
            }
            let lower = s.to_ascii_lowercase();
            if BLOCKS.iter().any(|k| lower.starts_with(k)) && s.contains('{') {
                c.blocks += 1;
                continue;
            }
            if is_ipish(s) {
                c.ips += 1;
                continue;
            }
            if OPTIONS.contains(&head) || s.contains('=') {
                c.options += 1;
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
            "global_defs {\n",
            "   router_id LVS1\n",
            "   enable_script_security\n",
            "}\n",
            "vrrp_script chk {\n",
            "    script \"/x.sh\"\n",
            "    interval 2\n",
            "}\n",
            "vrrp_instance VI_1 {\n",
            "    state MASTER\n",
            "    interface eth0\n",
            "    virtual_router_id 51\n",
            "    priority 100\n",
            "    authentication {\n",
            "        auth_type PASS\n",
            "        auth_pass 1111\n",
            "    }\n",
            "    virtual_ipaddress {\n",
            "        192.168.0.100\n",
            "        fd00::1\n",
            "    }\n",
            "    track_script {\n",
            "        chk\n",
            "    }\n",
            "}\n",
            "virtual_server 192.168.0.100 80 {\n",
            "    delay_loop 6\n",
            "    lb_algo rr\n",
            "    lb_kind NAT\n",
            "    real_server 192.168.0.10 80 {\n",
            "        weight 1\n",
            "        TCP_CHECK {\n",
            "            connect_timeout 3\n",
            "        }\n",
            "    }\n",
            "}\n",
        );
        let c = Keepalived::parse(b.as_bytes()).unwrap();
        assert_eq!(c.instances, 1);
        assert_eq!(c.scripts, 1);
        assert_eq!(c.servers, 1);
        assert_eq!(c.reals, 1);
        assert_eq!(c.blocks, 5);
        assert_eq!(c.ips, 2);
        assert_eq!(c.braces, 9);
        assert_eq!(c.options, 15);
    }

    #[test]
    fn rejects_other() {
        assert!(Keepalived::parse(b"foo = 1").is_none());
    }
}
