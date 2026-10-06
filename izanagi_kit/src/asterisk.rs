//! Asterisk `.conf` files (`sip.conf`, `pjsip.conf`, `extensions.conf`,
//! `voicemail.conf`, `queues.conf` …) census.
//!
//! `[section]` INI-ish groups; `key = value`, `key => value`, and
//! `exten => x,prio,Application(args)` dialplan lines; `;` comments;
//! `#include` / `#exec` directives.
//!
//! ```rust
//! let a = b"[general]\ncontext=default\nallowguest=no\n[6001]\ntype=friend\nsecret=1234\nhost=dynamic\nexten => 6001,1,Dial(SIP/6001)\n";
//! assert!(izanagi_kit::asterisk::detect(a));
//! let c = izanagi_kit::asterisk::Asterisk::parse(a).unwrap();
//! assert!(c.extens >= 1);
//! ```

/// Asterisk config census.
#[derive(Debug, Clone)]
pub struct Asterisk {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value`/`key => value` assignments matching a known key.
    pub settings: usize,
    /// `exten =>`/`same =>`/`register =>`/`include =>` arrow lines.
    pub extens: usize,
    /// `;` comment lines.
    pub comments: usize,
}

/// Keys common across chan_sip / pjsip / voicemail / queues / manager
/// config files (first word of an assignment line).
const KEYS: &[&str] = &[
    "accountcode",
    "acl",
    "adsi",
    "agent",
    "alert_info",
    "allow",
    "allow_transfer",
    "allowguest",
    "amaflags",
    "aors",
    "asverb",
    "auth",
    "auth_type",
    "autoframing",
    "autologoff",
    "avpf",
    "bindaddr",
    "bindport",
    "callerid",
    "callgroup",
    "canreinvite",
    "cc_agent_policy",
    "chanvar",
    "cid_number",
    "codec_pref",
    "comedia",
    "compactheaders",
    "context",
    "counter1",
    "default_expiration",
    "defaultexpiry",
    "defaultuser",
    "deny",
    "device_state_busy_at",
    "dial",
    "direct_media",
    "directory",
    "disallow",
    "dtmf",
    "dtmfmode",
    "dump",
    "emailbody",
    "emailsubject",
    "encryption",
    "endpoint_identifier_order",
    "engine",
    "eventlist",
    "expiry",
    "externaddr",
    "externhost",
    "externip",
    "externrefresh",
    "faxdetect",
    "force_avp",
    "force_rport",
    "format",
    "fromdomain",
    "fromuser",
    "fullname",
    "global",
    "group",
    "groupcount",
    "hint",
    "host",
    "ice_support",
    "identify",
    "identify_by",
    "ignore",
    "inband",
    "include",
    "insecure",
    "irc",
    "jbenable",
    "jbforce",
    "jbimpl",
    "jbmaxsize",
    "jbresyncthreshold",
    "jbtargetextra",
    "joinempty",
    "keep",
    "language",
    "lastms",
    "leavewhenempty",
    "limitonpeers",
    "localnet",
    "mailbox",
    "manager",
    "masquerade",
    "maxcallbitrate",
    "maxcallbitrate_rx",
    "maxcallbitrate_tx",
    "maxlen",
    "media_address",
    "media_encryption",
    "member",
    "members",
    "minexpiry",
    "mohinterpret",
    "mohsuggest",
    "monitor",
    "musiconhold",
    "nat",
    "nat_mode",
    "notifycid",
    "outbound_proxy",
    "parkinglot",
    "parkingtime",
    "pbx",
    "pedantic",
    "peer",
    "permit",
    "pickupgroup",
    "port",
    "prefer",
    "progressinband",
    "promiscredir",
    "qualify",
    "qualifyfreq",
    "queue",
    "read",
    "realm",
    "record_file",
    "regcontext",
    "regexten",
    "register",
    "reinvite",
    "relaxdtmf",
    "remotesecret",
    "ringinuse",
    "rtcachefriends",
    "rtpholdtimeout",
    "rtpkeepalive",
    "rtptimeout",
    "rtupdate",
    "secret",
    "send_diversion",
    "send_pai",
    "send_rpid",
    "sendrpid",
    "session-expires",
    "session-minse",
    "session-refresher",
    "session-timers",
    "setvar",
    "sharedexten",
    "sipdebug",
    "srvlookup",
    "strategy",
    "subscribemwi",
    "supportpath",
    "template",
    "timeout",
    "tos_audio",
    "tos_video",
    "transport",
    "trunk",
    "trustrpid",
    "type",
    "udpbindaddr",
    "username",
    "usereqphone",
    "videosupport",
    "vmexten",
    "voicemail",
    "wrapuptime",
];

fn arrow_name(t: &str) -> Option<&str> {
    let (k, _) = t.split_once("=>")?;
    let k = k.trim();
    if k.is_empty() {
        return None;
    }
    Some(k)
}

fn key_name(t: &str) -> &str {
    if let Some((k, _)) = t.split_once(" => ") {
        return k.trim();
    }
    if let Some((k, _)) = t.split_once('=') {
        return k.trim();
    }
    t.split(char::is_whitespace).next().unwrap_or("")
}

/// Detect an Asterisk `.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut extens = 0usize;
    let mut secs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with(';') || tr.starts_with('#') {
            continue;
        }
        if let Some(inner) = tr.strip_prefix('[') {
            if inner.strip_suffix(']').is_some() {
                secs += 1;
                continue;
            }
        }
        if let Some(k) = arrow_name(tr) {
            if k.starts_with("exten") || k.starts_with("same") || k == "register" || k == "include"
            {
                extens += 1;
                continue;
            }
            if KEYS.contains(&k) {
                keys += 1;
            }
            continue;
        }
        if tr.contains('=') {
            let k = key_name(tr);
            if KEYS.contains(&k) {
                keys += 1;
            }
        }
    }
    extens >= 1 && (keys >= 1 || secs >= 1) || keys >= 3
}

impl Asterisk {
    /// Count sections, settings and exten lines. Returns `None` when the
    /// input does not look like an Asterisk config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            extens: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with(';') || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(inner) = tr.strip_prefix('[') {
                if inner.strip_suffix(']').is_some() {
                    c.sections += 1;
                    continue;
                }
            }
            if let Some(k) = arrow_name(tr) {
                if k.starts_with("exten")
                    || k.starts_with("same")
                    || k == "register"
                    || k == "include"
                {
                    c.extens += 1;
                } else if KEYS.contains(&k) {
                    c.settings += 1;
                }
                continue;
            }
            if tr.contains('=') && KEYS.contains(&key_name(tr)) {
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
    fn detects_sip() {
        let b = b"; sip.conf\n[general]\ncontext=default\nallowguest=no\nsrvlookup=yes\nbindport=5060\nbindaddr=0.0.0.0\nqualify=yes\n[6001]\ntype=friend\nsecret=1234\nhost=dynamic\ncontext=phones\n";
        assert!(detect(b));
        let c = Asterisk::parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert!(c.settings >= 9);
    }

    #[test]
    fn detects_dialplan() {
        let b = b"[globals]\n[internal]\nexten => 6001,1,Dial(SIP/6001)\nsame => n,Hangup()\nexten => _6XXX,1,Playback(hello)\n";
        assert!(detect(b));
        let c = Asterisk::parse(b).unwrap();
        assert_eq!(c.extens, 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[Unit]\nDescription=x\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
        assert!(Asterisk::parse(b"").is_none());
    }
}
