//! Janus Gateway `*.jcfg` config census (`janus.jcfg`,
//! `janus.plugin.*.jcfg`, `janus.transport.*.jcfg`).
//!
//! libconfig-style: `key: value`, `key: {` groups, `array: […]`,
//! `string: "…"`, `#`/`//` comments. Janus-specific group names
//! (`general`, `nat`, `media`, `plugins`, `events`, `loggers`,
//! `certificates`, `admin`, `apis`, `ws`, `http`, `mqtt`, `rabbitmq`,
//! `nanomsg`, `pfunix`) and keys (`apisecret`, `token_auth`,
//! `session_timeout`, `server_name`, `ws_port` …).
//!
//! ```rust
//! let j = b"general: {\n\ttoken_auth = true\n\tserver_name = \"janus\"\n\tdebug_level = 4\n}\nhttp: { base_path = \"/janus\" }\n";
//! assert!(izanagi_kit::janus::detect(j));
//! let c = izanagi_kit::janus::Janus::parse(j).unwrap();
//! assert!(c.groups >= 1);
//! ```

/// janus.jcfg census.
#[derive(Debug, Clone)]
pub struct Janus {
    /// `key: {`/`key = {` group-open lines matching known Janus groups.
    pub groups: usize,
    /// `key[:=] value` lines matching a known Janus setting.
    pub settings: usize,
    /// `#`/`//`/`/*` comment lines.
    pub comments: usize,
}

/// Top-level group names used across Janus configs.
const GROUPS: &[&str] = &[
    "admin",
    "apis",
    "api",
    "audio_level_event",
    "broadcast",
    "certificates",
    "crypto",
    "events",
    "general",
    "grouping",
    "groups",
    "http",
    "ice",
    "loggers",
    "media",
    "misc",
    "mqtt",
    "nanomsg",
    "nat",
    "pfunix",
    "plugins",
    "rabbitmq",
    "recordings",
    "rooms",
    "streaming",
    "turn_rest_api",
    "websocket",
    "websockets",
    "ws",
];

/// Janus setting keys (first token before `:`/`=`).
const KEYS: &[&str] = &[
    "admin_base_path",
    "admin_http",
    "admin_https",
    "admin_ip",
    "admin_mhd_connection_limit",
    "admin_port",
    "admin_secure_port",
    "admin_ws",
    "admin_ws_port",
    "api_secret",
    "apisecret",
    "auth_secret",
    "backend",
    "base_path",
    "bind",
    "broadcast",
    "bundle",
    "cache_dir",
    "cert_pem",
    "cert_pwd",
    "certificates",
    "cors",
    "debug_handshake",
    "debug_level",
    "debug_locks",
    "debug_timestamps",
    "dtls_accept_selfsigned",
    "dtls_timeout",
    "enable_media_recording",
    "enforce_secure",
    "events",
    "fish",
    "folder",
    "full_trickle",
    "gathering_timeout",
    "general_info",
    "group",
    "host",
    "hostname",
    "ice_debug",
    "ice_enforce_list",
    "ice_ignore_list",
    "ice_lite",
    "ice_keepalive_conncheck",
    "ice_nomination",
    "ice_tcp",
    "ice_ttl",
    "ip",
    "ipv6",
    "keepalive_host",
    "kitcat",
    "local_ip",
    "log_to_file",
    "log_to_stdout",
    "loggers",
    "media",
    "media_level_event",
    "mhd_connection_limit",
    "min_nack_queue",
    "mn",
    "name",
    "nat_1_1_mapping",
    "nice_debug",
    "no_webrtc_encryption",
    "path",
    "pe",
    "pid_file",
    "plugins_folder",
    "port",
    "protected_folders",
    "publish",
    "queue_admin",
    "queue_janus",
    "reclaim_session_timeout",
    "recordings_extglob",
    "recordings_folder",
    "recordings_tmp_extglob",
    "rtp_port_range",
    "rtsp_timeout",
    "secure_path",
    "server_name",
    "session_timeout",
    "snd_room",
    "stun_server",
    "stun_port",
    "string_ids",
    "threads",
    "token_auth",
    "token_auth_secret",
    "transport",
    "turn_password",
    "turn_port",
    "turn_rest_api_key",
    "turn_server",
    "turn_type",
    "turn_user",
    "type",
    "verbose_events",
    "ws",
    "ws_ip",
    "ws_logging",
    "ws_port",
    "wss_port",
];

fn first_token(t: &str) -> &str {
    t.split([':', '=', ' ', '\t', '{']).next().unwrap_or("")
}

/// Detect a `*.jcfg` Janus config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut marks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with("//") || tr.starts_with("/*") {
            continue;
        }
        let tok = first_token(tr);
        if GROUPS.contains(&tok) && (tr.contains('{') || tr.contains(':') || tr.contains('=')) {
            marks += 1;
            continue;
        }
        if KEYS.contains(&tok) && (tr.contains(':') || tr.contains('=')) {
            marks += 1;
        }
    }
    marks >= 3
}

impl Janus {
    /// Count groups and settings. Returns `None` when the input does not
    /// look like a `*.jcfg`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            groups: 0,
            settings: 0,
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
            let tok = first_token(tr);
            if GROUPS.contains(&tok) && (tr.contains('{') || tr.contains(':') || tr.contains('=')) {
                c.groups += 1;
                continue;
            }
            if KEYS.contains(&tok) && (tr.contains(':') || tr.contains('=')) {
                c.settings += 1;
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
        let b = b"# janus.jcfg\ngeneral: {\n\tconfigs_folder = \"etc/janus\"\n\tplugins_folder = \"lib/janus/plugins\"\n\tevents = true\n\ttoken_auth = true\n\tapi_secret = \"janussecret\"\n\tdebug_level = 4\n}\n\nnat: {\n\tstun_server = \"stun.voip.eutelia.it\"\n\tstun_port = 3478\n\tnice_debug = false\n}\n\nmedia: {\n\tipv6 = false\n\trtp_port_range = \"20000-40000\"\n}\n\nhttp: {\n\tbase_path = \"/janus\"\n\tthreads = \"unlimited\"\n\thttp = true\n\tport = 8088\n}\n";
        assert!(detect(b));
        let c = Janus::parse(b).unwrap();
        assert_eq!(c.groups, 6);
        assert!(c.settings >= 10);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[section]\nkey = value\n"));
        assert!(!detect(b"foo: 1\nbar: 2\n"));
        assert!(Janus::parse(b"").is_none());
    }
}
