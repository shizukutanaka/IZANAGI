//! Kamailio `kamailio.cfg` (and `*.cfg` cfg-script) census.
//!
//! Shebang-style defines `#!define`/`#!subst`/`#!ifdef`/`#!endif`/
//! `#!trydef`/`#!redefine`/`#!include`/`#!import_file`, global
//! `key=value` params (`debug`, `listen`, `port`, `children` …),
//! `loadmodule`/`loadmodulex`/`modparam` lines, and named route blocks
//! (`route {`, `route[NAME] {`, `request_route`, `reply_route`,
//! `onreply_route`, `failure_route`, `branch_route`, `onsend_route`,
//! `event_route`, `htable`, `sip_timer`).
//!
//! ```rust
//! let k = b"#!define WITH_MYSQL\ndebug=2\nlisten=udp:127.0.0.1:5060\nloadmodule \"tm.so\"\nroute { sl_send_reply(\"100\", \"trying\"); }\n";
//! assert!(izanagi_kit::kamailio::detect(k));
//! let c = izanagi_kit::kamailio::Kamailio::parse(k).unwrap();
//! assert_eq!(c.defines, 1);
//! ```

/// kamailio.cfg census.
#[derive(Debug, Clone)]
pub struct Kamailio {
    /// `#!directive` define lines.
    pub defines: usize,
    /// `loadmodule*`/`modparam`/`loadpath`/`mpath` lines.
    pub modules: usize,
    /// `key=value` global parameter lines matching a known param.
    pub settings: usize,
    /// Route-block headers (`route[`/`request_route`/`failure_route`/…).
    pub routes: usize,
    /// `#`/`//`/`/*` comment lines (non `#!` directives).
    pub comments: usize,
}

/// Kamailio `#!` preprocessor directives.
const DEFINES: &[&str] = &[
    "#!define",
    "#!elif",
    "#!else",
    "#!endif",
    "#!ifdef",
    "#!ifndef",
    "#!import_file",
    "#!include",
    "#!include_file",
    "#!redefine",
    "#!subst",
    "#!substdef",
    "#!substdefs",
    "#!trydef",
    "#!tryinclude",
    "#!defined",
];

/// Global parameter names (before `=`).
const KEYS: &[&str] = &[
    "alias",
    "auto_aliases",
    "cfgengine",
    "check_via",
    "children",
    "chroot",
    "corelog",
    "debug",
    "disable_core_dump",
    "dns",
    "dns_cache_init",
    "dns_retr_time",
    "dns_servers_no",
    "dns_try_ipv6",
    "enable_tls",
    "exit_timeout",
    "flags",
    "fork",
    "group",
    "latency_cfg_log",
    "latency_limit_action",
    "latency_limit_db",
    "latency_log",
    "listen",
    "loadmodule",
    "loadmodulex",
    "loadpath",
    "log_error",
    "log_facility",
    "log_level",
    "log_name",
    "log_prefix",
    "log_prefix_mode",
    "log_stderror",
    "log_timezone",
    "maxbuffer",
    "mem_join",
    "memdbg",
    "memlog",
    "memsummary",
    "mhomed",
    "mmap",
    "modparam",
    "mpath",
    "open_files_limit",
    "phone2tel",
    "pid",
    "pidfile",
    "port",
    "reply_to_via",
    "rev_dns",
    "server_header",
    "server_id",
    "server_signature",
    "shm_force_alloc",
    "sip_warning",
    "socket",
    "stats",
    "syn_branch",
    "tcp_accept_aliases",
    "tcp_accept_haproxy",
    "tcp_children",
    "tcp_connection_lifetime",
    "tcp_max_connections",
    "tcp_no_new_conn_bflag",
    "tcp_rd_buf_size",
    "tls_max_connections",
    "tos",
    "udp4_raw",
    "udp_mtu",
    "udp_mtu_try_proto",
    "user",
    "user_agent_header",
    "version_table",
    "workdir",
    "xavp_via_params",
];

/// Route-block openers (first word of a block header line).
const ROUTES: &[&str] = &[
    "async",
    "branch_route",
    "cfg_reply_route",
    "event_route",
    "failure_route",
    "htable",
    "local_route",
    "onreply_route",
    "reply_route",
    "request_route",
    "route",
    "send_reply_route",
    "sip_timer",
    "startup_route",
    "stun",
    "tcp_connection_closed_route",
    "timer_route",
    "tls_connection_closed_route",
    "udp_mtu_try",
    "ws_handshake_route",
    "xlog",
];

fn define(t: &str) -> bool {
    DEFINES.iter().any(|p| t.starts_with(p))
}

fn route_head(t: &str) -> bool {
    let w = t.split([' ', '\t', '[', '{', '(']).next().unwrap_or("");
    ROUTES.contains(&w)
        && (t.contains('[') || t.contains('{') || w == "request_route" || w == "reply_route")
}

fn assign_key(t: &str) -> Option<&str> {
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

/// Detect a Kamailio `*.cfg` script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut marks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') && !tr.starts_with("#!") {
            continue;
        }
        if tr.starts_with('#') && !define(tr) {
            continue;
        }
        if define(tr)
            || route_head(tr)
            || tr.starts_with("loadmodule")
            || tr.starts_with("loadmodulex")
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
    marks >= 3
}

impl Kamailio {
    /// Count cfg-script line categories. Returns `None` when the input
    /// does not look like a `kamailio.cfg`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            defines: 0,
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
            if tr.starts_with('#') {
                if define(tr) {
                    c.defines += 1;
                } else {
                    c.comments += 1;
                }
                continue;
            }
            if tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            if route_head(tr) {
                c.routes += 1;
                continue;
            }
            if tr.starts_with("loadmodule")
                || tr.starts_with("loadmodulex")
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
        let b = b"#!define WITH_MYSQL\n#!define WITH_AUTH\n# kamailio\n####### Global Parameters\ndebug=3\nlog_stderror=no\nmemdbg=5\nsip_warning=0\nchildren=4\nlisten=udp:10.0.0.1:5060\nport=5060\nloadmodule \"tm.so\"\nloadmodule \"sl.so\"\nloadmodule \"rr.so\"\nmodparam(\"rr\", \"append_fromtag\", 1)\nrequest_route {\n\tsl_send_reply(\"100\", \"trying\");\n}\nroute[REGISTRAR] {\n}\nfailure_route[MANAGE_FAILURE] {\n}\nevent_route[xhttp:request] {\n}\n";
        assert!(detect(b));
        let c = Kamailio::parse(b).unwrap();
        assert_eq!(c.defines, 2);
        assert_eq!(c.modules, 4);
        assert!(c.settings >= 7);
        assert_eq!(c.routes, 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"server {\n listen 80;\n}\n"));
        assert!(!detect(b"foo=1\nbar=2\nbaz=3\n"));
        assert!(Kamailio::parse(b"").is_none());
    }
}
