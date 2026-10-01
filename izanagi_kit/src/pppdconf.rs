//! pppd `options` / `pap-secrets` / `chap-secrets` / `peers/*` 形式の検出・カウント。
//!
//! オプションは `key value` / `key=value` / 裸フラグの 3 形態。
//! secrets ファイルは `client server secret ip` の 4 欄。
//!
//! ```
//! let cfg = b"debug\n\
//!             lock\n\
//!             name myhost\n\
//!             remotename isp\n\
//!             mtu 1492\n\
//!             mru 1492\n\
//!             defaultroute\n\
//!             usepeerdns\n\
//!             persist\n";
//! assert!(izanagi_kit::pppdconf::detect(cfg));
//! let c = izanagi_kit::pppdconf::parse(cfg).unwrap();
//! assert!(c.flags >= 4);
//! ```

/// pppd オプションとして既知のキー/フラグ。
const OPTION_KEYS: &[&str] = &[
    "auth",
    "noauth",
    "user",
    "password",
    "pty",
    "connect",
    "disconnect",
    "welcome",
    "init",
    "mtu",
    "mru",
    "netmask",
    "name",
    "remotename",
    "persist",
    "demand",
    "idle",
    "holdoff",
    "maxconnect",
    "maxfail",
    "connect-delay",
    "lcp-echo-interval",
    "lcp-echo-failure",
    "lcp-echo-adaptive",
    "lcp-restart",
    "lcp-max-terminate",
    "lcp-max-configure",
    "lcp-max-failure",
    "lcp-delay",
    "ipcp-accept-local",
    "ipcp-accept-remote",
    "ipcp-max-configure",
    "ipcp-max-failure",
    "ipcp-max-terminate",
    "ipcp-restart",
    "ipcp-negotiate-addresses",
    "ipcp-no-addresses",
    "ipcp-address",
    "ipcp-remote-address",
    "ipparam",
    "ms-dns",
    "ms-wins",
    "defaultroute",
    "nodefaultroute",
    "defaultroute6",
    "nodefaultroute6",
    "replacedefaultroute",
    "proxyarp",
    "noproxyarp",
    "usepeerdns",
    "usepeerwins",
    "debug",
    "dump",
    "dryrun",
    "logfd",
    "logfile",
    "nologfd",
    "show-password",
    "logdevice",
    "updetach",
    "master_detach",
    "noremoteip",
    "remoteip",
    "localip",
    "ipaddr",
    "ktune",
    "crtscts",
    "nocrtscts",
    "xonxoff",
    "rtscts",
    "cdtrcts",
    "modem",
    "local",
    "sync",
    "asyncmap",
    "escape",
    "passive",
    "silent",
    "nopersist",
    "nodetach",
    "detached",
    "nodetach_delay",
    "up_holdoff",
    "lock",
    "nolock",
    "lock-timeout",
    "domain",
    "domain-suffix",
    "kdebug",
    "child-timeout",
    "endpoint",
    "epdisc",
    "multilink",
    "nomultilink",
    "mp",
    "mrru",
    "multilink-mrru",
    "mppe-stateful",
    "nomppe-stateful",
    "nomppe",
    "nomppe-40",
    "nomppe-128",
    "nomppe-56",
    "refuse-eap",
    "refuse-pap",
    "refuse-chap",
    "refuse-mschap",
    "refuse-mschap-v2",
    "refuse-mschapv2",
    "require-eap",
    "require-pap",
    "require-chap",
    "require-mschap",
    "require-mschap-v2",
    "allow-ip",
    "allow-number",
    "remotenumber",
    "chap-interval",
    "chap-restart",
    "chap-max-challenge",
    "chapms-strip-domain",
    "pap-restart",
    "pap-max-authreq",
    "pap-timeout",
    "papcrypt",
    "ipv6",
    "ipv6cp-use-ipaddr",
    "ipv6cp-use-persistent",
    "ipv6cp-accept-local",
    "ipv6cp-accept-remote",
    "ipv6cp-max-configure",
    "ipv6cp-max-failure",
    "ipv6cp-max-terminate",
    "ipv6cp-restart",
    "ipv6cp-use-remotenumber",
    "killed",
    "linkname",
    "call",
    "plugin",
    "pluginpath",
    "noipx",
    "ipxcp-accept-local",
    "ipxcp-accept-network",
    "ipxcp-accept-remote",
    "ipxcp-restart",
    "ipx-network",
    "ipx-node",
    "ipx-peer-name",
    "ipx-routing",
    "ipx-router-name",
    "bsdcomp",
    "nobsdcomp",
    "deflate",
    "nodeflate",
    "nopredictor1",
    "predictor1",
    "novj",
    "novjccomp",
    "novjcid",
    "vj-max-slots",
    "noccp",
    "bsdcomp-min",
    "bsdcomp-max",
    "deflate-level",
    "ac",
    "pc",
    "am",
    "ap",
    "as",
    "default-asyncmap",
    "accm",
    "acfc",
    "pfc",
    "receive-all",
    "vj",
    "vjccomp",
    "ccp",
    "ccp-max-configure",
    "ccp-max-failure",
    "ccp-max-terminate",
    "ccp-restart",
    "ccp-method",
    "ccp-stateful",
    "cpw",
    "dns",
    "wins",
    "netbios-ns",
    "netbios-nbns",
    "eap-interval",
    "eap-max-sreq",
    "eap-restart",
    "eap-timeout",
    "hide-password",
    "show_pwd",
    "pass-filter",
    "active-filter",
    "socket",
    "unit",
    "ifname",
    "set",
    "unset",
    "file",
    "callfile",
    "options",
    "linkstats",
    "loopback",
    "bandwidth",
    "max_baud",
    "serial_speed",
    "speed",
    "baud",
    "modem_chat",
    "modemscript",
    "dial",
    "hangup",
    "abort",
    "connect-delay2",
    "initchat",
    "prechat",
    "redial",
    "redialtimeout",
    "inactivity",
    "idle-timeout",
    "disconnected",
    "noipdefault",
    "ipdefault",
    "ipcp-addresses",
    "maxoctets",
    "maxoctets_dir",
    "maxoctets_timeout",
    "unobtainable",
    "stop-bits",
    "parity",
    "xonxoff_flow",
    "xon-xoff",
    "cdtrcts-flow",
    "rtscts-flow",
    "crlf",
    "crtscts_flow",
    "remote_name",
    "sessid",
    "pty_socket",
    "pty_program",
    "socket_timeout",
    "record",
    "pin-file",
    "pin",
    "stop_after_ppp",
    "max_baud_rate",
];

/// secrets 欄パターン判定用 — `client server "secret" ip` 型 4 欄行。
fn is_secret_line(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let a = parts.next();
    let b = parts.next();
    let c = parts.next();
    match (a, b, c) {
        (Some(_), Some(_), Some(_)) => {
            // 3 番目以降が `"` か `*`、または第 4 欄がある
            let third = line.split_whitespace().nth(2).unwrap_or("");
            third.starts_with('"') || third == "*"
        }
        _ => false,
    }
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ディレクティブ行・シークレット行総数。
    pub entries: usize,
    /// 裸フラグ (`debug`, `persist` 等) 数。
    pub flags: usize,
    /// `key value` / `key=value` 代入数。
    pub options: usize,
    /// `client server secret ip` 4 欄シークレット行数 (pap/chap-secrets)。
    pub secrets: usize,
    /// `no*`/`refuse-*`/`require-*`/`allow-*` 否定/許可フラグ数。
    pub negations: usize,
    /// `lcp-*`/`ipcp-*`/`ccp-*`/`ipv6cp-*`/`ipx*`/`eap-*`/`chap*`/`pap*` 等プロトコルプレフィックス数。
    pub proto: usize,
    /// `connect`/`pty`/`plugin`/`file`/`call`/`logfile`/`lock` 等フック/パス系数。
    pub hooks: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が pppd オプション/secrets 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 3 || c.secrets >= 1)
}

/// `b` を pppd 設定ファイルとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        flags: 0,
        options: 0,
        secrets: 0,
        negations: 0,
        proto: 0,
        hooks: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if is_secret_line(line) {
            c.secrets += 1;
            known += 1;
            continue;
        }
        let head = line.split([' ', '\t', '=']).next().unwrap_or("");
        if head.is_empty() {
            c.misc += 1;
            continue;
        }
        let known_opt = OPTION_KEYS.contains(&head);
        if known_opt {
            known += 1;
        }
        let has_value = line.contains('=')
            || line
                .split_whitespace()
                .nth(1)
                .is_some_and(|v| !v.starts_with('#'));
        if (head.starts_with("no") && OPTION_KEYS.contains(&head))
            || head.starts_with("refuse-")
            || head.starts_with("require-")
            || head.starts_with("allow-")
        {
            c.negations += 1;
        } else if head.starts_with("lcp-")
            || head.starts_with("ipcp-")
            || head.starts_with("ccp-")
            || head.starts_with("ipv6")
            || head.starts_with("ipx")
            || head.starts_with("eap-")
            || head.starts_with("chap")
            || head.starts_with("pap")
            || head.starts_with("bsdcomp")
            || head.starts_with("deflate")
            || head.starts_with("mppe")
            || head.starts_with("nomppe")
            || head.starts_with("ms-")
            || head.starts_with("predictor")
            || head.starts_with("nopredictor")
            || head.starts_with("vj")
            || head.starts_with("novj")
        {
            c.proto += 1;
        } else if matches!(
            head,
            "connect"
                | "disconnect"
                | "pty"
                | "plugin"
                | "pluginpath"
                | "file"
                | "call"
                | "callfile"
                | "options"
                | "init"
                | "welcome"
                | "logfile"
                | "lock"
                | "set"
                | "unset"
                | "socket"
                | "record"
                | "pin"
                | "pin-file"
                | "pty_program"
                | "pty_socket"
                | "domain"
                | "script"
                | "dial"
                | "hangup"
                | "abort"
                | "modemscript"
                | "redial"
                | "initchat"
                | "prechat"
        ) {
            c.hooks += 1;
        } else if known_opt && has_value {
            c.options += 1;
        } else if known_opt {
            c.flags += 1;
        } else {
            c.misc += 1;
        }
    }
    if known >= 2 || c.secrets >= 1 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# pppd options\n\
        debug\n\
        lock\n\
        name myhost\n\
        remotename isp\n\
        mtu 1492\n\
        mru 1492\n\
        defaultroute\n\
        usepeerdns\n\
        persist\n\
        maxfail 0\n\
        lcp-echo-interval 30\n\
        lcp-echo-failure 4\n\
        require-chap\n\
        refuse-pap\n\
        connect /usr/sbin/chat -f /etc/ppp/chatscript\n\
        plugin rp-pppoe.so\n";

    #[test]
    fn detects_pppd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 16);
        assert_eq!(c.flags, 4);
        assert_eq!(c.options, 5);
        assert_eq!(c.negations, 2);
        assert_eq!(c.proto, 2);
        assert_eq!(c.hooks, 3);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn detects_pap_secrets() {
        let secrets = b"\"client1\"  *  \"secret1\"  192.168.1.0/24\n\"client2\"  *  \"secret2\"\n";
        assert!(detect(secrets));
        let c = parse(secrets).unwrap();
        assert_eq!(c.secrets, 2);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"debug\nlock\n"));
    }
}
