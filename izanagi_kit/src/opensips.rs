//! OpenSIPS `opensips.cfg` (cfg-script) census.
//!
//! Kamailio-fork scripting format: `#`/`//` comments (no `#!` defines),
//! global `key=value` params (`debug`, `log_level`, `listen`, `socket`,
//! `mpath`, `memdump` …), `loadmodule`/`modparam` lines, and route
//! blocks (`route {`, `route[name]`, `startup_route`, `timer_route`,
//! `local_route`, `event_route`, `branch_route`, `failure_route`,
//! `onreply_route`, `error_route`, `missing_languages`).
//!
//! ```rust
//! let o = b"debug=3\nlog_level=2\nsocket=udp:127.0.0.1:5060\nloadmodule \"tm.so\"\nmodparam(\"tm\", \"fr_timer\", 5)\nroute { xlog(\"hi\"); }\n";
//! assert!(izanagi_kit::opensips::detect(o));
//! let c = izanagi_kit::opensips::Opensips::parse(o).unwrap();
//! assert_eq!(c.routes, 1);
//! ```

/// opensips.cfg census.
#[derive(Debug, Clone)]
pub struct Opensips {
    /// `loadmodule`/`modparam`/`mpath`/`loadpath` lines.
    pub modules: usize,
    /// `key=value` global parameter lines matching a known param.
    pub settings: usize,
    /// Route-block headers (`route[`/`startup_route`/`timer_route`/…).
    pub routes: usize,
    /// `#`/`//`/`/*` comment lines.
    pub comments: usize,
}

/// OpenSIPS global parameter names (before `=`).
const KEYS: &[&str] = &[
    "advertised_address",
    "advertised_port",
    "alias",
    "auto_aliases",
    "cfg_file",
    "chroot",
    "daemon",
    "db_version_table",
    "debug_mode",
    "debug",
    "disable_503_translation",
    "disable_core_dump",
    "disable_dns_blacklist",
    "disable_dns_failover",
    "disable_stateless_fwd",
    "dns",
    "dns_retr_time",
    "dns_servers_no",
    "dns_try_ipv6",
    "dns_try_naptr",
    "dr_enable",
    "enable_asserts",
    "event_pkg_threshold",
    "exec_msg_threshold",
    "exec_post_threshold",
    "exec_pre_threshold",
    "fork",
    "group",
    "import_file",
    "include_file",
    "listen",
    "loadmodule",
    "loadpath",
    "log_level",
    "log_name",
    "log_stderror",
    "log_facility",
    "maxbuffer",
    "mem_warming",
    "memdump",
    "memlog",
    "memgroup",
    "mhomed",
    "mix_mixed_sdp",
    "mpath",
    "multi",
    "open_files_limit",
    "options_reply_code",
    "parallel_stateless",
    "pid",
    "pid_file",
    "query_buffer_size",
    "query_flush_time",
    "reply_to_via",
    "restart_persistency",
    "rev_dns",
    "server_header",
    "server_signature",
    "shm_hash_split_percentage",
    "shm_secondary_hash_size",
    "sip_warning",
    "socket",
    "socket_workers",
    "statistics",
    "stdout",
    "tcp_accept_aliases",
    "tcp_children",
    "tcp_connect_timeout",
    "tcp_connection_lifetime",
    "tcp_max_connections",
    "tcp_max_msg_chunks",
    "tcp_max_msg_time",
    "tcp_no_new_conn_bflag",
    "tcp_threshold",
    "tls_ca_dir",
    "tls_ca_list",
    "tls_certificate",
    "tls_ciphers_list",
    "tls_dh_params",
    "tls_ec_curve",
    "tls_handshake_timeout",
    "tls_log",
    "tls_max_connections",
    "tls_method",
    "tls_port_no",
    "tls_private_key",
    "tls_require_client_certificate",
    "tls_send_timeout",
    "tls_verify_cert",
    "tos",
    "udp_workers",
    "unix_sock",
    "unix_tx_timeout",
    "user",
    "user_agent_header",
    "version",
    "workdir",
    "working_mode",
    "xlog_buf_size",
    "xlog_default_level",
    "xlog_force_color",
    "xlog_level",
];

/// Route-block openers (first word).
const ROUTES: &[&str] = &[
    "async",
    "branch_route",
    "error_route",
    "event_route",
    "failure_route",
    "launch",
    "local_route",
    "missing_languages",
    "onreply_route",
    "request_route",
    "route",
    "startup_route",
    "timer_route",
];

fn route_head(t: &str) -> bool {
    let w = t.split([' ', '\t', '[', '{', '(']).next().unwrap_or("");
    ROUTES.contains(&w) && (t.contains('[') || t.contains('{') || w == "startup_route")
}

fn assign_key(t: &str) -> Option<&str> {
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

/// Detect an OpenSIPS `*.cfg` script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut marks = 0usize;
    let mut routes = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with("//") || tr.starts_with("/*") {
            continue;
        }
        if route_head(tr) {
            marks += 1;
            routes += 1;
            continue;
        }
        if tr.starts_with("loadmodule")
            || tr.starts_with("modparam")
            || tr.starts_with("mpath")
            || tr.starts_with("loadpath")
        {
            marks += 1;
            continue;
        }
        if let Some(k) = assign_key(tr) {
            if KEYS.contains(&k) {
                marks += 1;
            }
        }
    }
    marks >= 3 && routes >= 1
}

impl Opensips {
    /// Count cfg-script line categories. Returns `None` when the input
    /// does not look like an `opensips.cfg`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            modules: 0,
            settings: 0,
            routes: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            if route_head(tr) {
                c.routes += 1;
                continue;
            }
            if tr.starts_with("loadmodule")
                || tr.starts_with("modparam")
                || tr.starts_with("mpath")
                || tr.starts_with("loadpath")
            {
                c.modules += 1;
                continue;
            }
            if let Some(k) = assign_key(tr) {
                if KEYS.contains(&k) {
                    c.settings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# opensips\n####### Global Parameters #########\ndebug=3\nlog_stderror=no\nlog_facility=LOG_LOCAL0\nsocket=udp:10.0.0.1:5060\nsocket=tcp:10.0.0.1:5060\nworkers=4\n\n####### Modules Section ########\nmpath=\"/usr/lib/opensips/modules/\"\nloadmodule \"signaling.so\"\nloadmodule \"sl.so\"\nloadmodule \"tm.so\"\nmodparam(\"tm\", \"fr_timer\", 5)\nmodparam(\"tm\", \"fr_inv_timer\", 30)\nloadmodule \"rr.so\"\nmodparam(\"rr\", \"append_fromtag\", 1)\n\n####### Routing Logic ########\nroute {\n\txlog(\"incoming\");\n}\nroute[relay] {\n}\nfailure_route[missed_call] {\n}\nstartup_route {\n}\n";
        assert!(detect(b));
        let c = Opensips::parse(b).unwrap();
        assert_eq!(c.modules, 8);
        assert!(c.settings >= 5);
        assert_eq!(c.routes, 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"debug=3\nfoo=1\nbar=2\n"));
        assert!(!detect(b"kamailio\n"));
        assert!(Opensips::parse(b"").is_none());
    }
}
