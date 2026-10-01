//! Poptop `pptpd.conf`/`options.pptpd` census.
//!
//! Whitespace-separated `key value` options (`option`,
//! `debug`, `stimeout`, `localip`, `remoteip`,
//! `listen`, `pidfile`, `speed`, `noipparam`,
//! `logwtmp`, `bcrelay`, `delegate`, `connections`,
//! `freebind`) plus ppp options-file directives
//! (`name`, `refuse-pap`, `refuse-chap`, `refuse-mschap`,
//! `refuse-mschap-v2`, `refuse-eap`, `require-mschap-v2`,
//! `require-mppe-128`, `require-mppe`, `mppe-stateful`,
//! `nomppe`, `ms-dns`, `ms-wins`, `proxyarp`,
//! `nodefaultroute`, `noreplacedefaultroute`,
//! `debug`, `lock`, `nobsdcomp`, `novj`, `novjccomp`,
//! `novj-ccomp`, `nopcomp`, `noaccomp`, `noproxyarp`,
//! `default-asyncmap`, `mtu`, `mru`, `lcp-echo-interval`,
//! `lcp-echo-failure`, `auth`, `chap-secrets`,
//! `chapms-strip-domain`, `ipparam`, `plugin`,
//! `multilink`, `mp`, `endpoint`, `logfile`,
//! `logfd`, `idle`, `holdoff`, `connect-delay`,
//! `maxfail`, `persist`, `demand`, `ktune`,
//! `defaultroute`, `replacedefaultroute`,
//! `usepeerdns`, `domain`, `local`, `pap`,
//! `chap`, `mschap`, `mschap-v2`, `eap`,
//! `papcrypt`, `bsdcomp`, `deflate`,
//! `pppoe`, `pppoe-interval`, `netmask`,
//! `ipv6`, `ipv6cp-use-ipaddr`, `ip-up-script`,
//! `ip-down-script`, `ip-pre-up-script`, `linkname`,
//! `maxconnect`, `remotenumber`, `allow-ip`,
//! `remap-ipv6`, `ipv6cp-accept-local`,
//! `ipv6cp-accept-remote`, `dryrun`, `dump`,
//! `updetach`, `init`, `welcome`, `pty`,
//! `notty`, `record`, `ifname`, `ipx-*`,
//! `chap-interval`, `chap-max-challenge`,
//! `chap-restart`, `eap-restart`, `eap-max-auths`,
//! `pap-restart`, `pap-max-authreq`, `pap-timeout`,
//! `lcp-max-configure`, `lcp-max-failure`,
//! `lcp-max-terminate`, `lcp-restart`,
//! `ccp-restart`, `ccp-max-configure`,
//! `ccp-max-failure`, `ccp-max-terminate`,
//! `ipcp-restart`, `ipcp-max-configure`,
//! `ipcp-max-failure`, `ipcp-max-terminate`,
//! `ipcp-accept-local`, `ipcp-accept-remote`,
//! `kdebug`, `pass-filter`, `active-filter`,
//! `log-max-size`, `session-timeout`,
//! `receive-all`, `nopredictor1`, `predictor1`).
//!
//! ```rust
//! let p = "option /etc/ppp/options.pptpd\nlocalip 10.0.0.1\nremoteip 10.0.0.100-200\n";
//! let c = izanagi_kit::pptpd::Pptpd::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.settings, 3);
//! ```

/// pptpd.conf census.
#[derive(Debug, Clone)]
pub struct Pptpd {
    /// `key value` lines.
    pub settings: usize,
    /// Recognised pptpd/pppd option names.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "option",
    "debug",
    "stimeout",
    "localip",
    "remoteip",
    "listen",
    "pidfile",
    "speed",
    "noipparam",
    "logwtmp",
    "bcrelay",
    "delegate",
    "connections",
    "freebind",
    "ppp",
    "name",
    "refuse-pap",
    "refuse-chap",
    "refuse-mschap",
    "refuse-mschap-v2",
    "refuse-eap",
    "require-chap",
    "require-mschap",
    "require-mschap-v2",
    "require-mppe",
    "require-mppe-128",
    "mppe-stateful",
    "mppe-stateless",
    "nomppe",
    "ms-dns",
    "ms-wins",
    "proxyarp",
    "nodefaultroute",
    "noreplacedefaultroute",
    "defaultroute",
    "replacedefaultroute",
    "usepeerdns",
    "lock",
    "nobsdcomp",
    "novj",
    "novjccomp",
    "novj-ccomp",
    "nopcomp",
    "noaccomp",
    "noproxyarp",
    "default-asyncmap",
    "asyncmap",
    "mtu",
    "mru",
    "mrru",
    "lcp-echo-interval",
    "lcp-echo-failure",
    "auth",
    "noauth",
    "chap-secrets",
    "chapms-strip-domain",
    "ipparam",
    "plugin",
    "multilink",
    "mp",
    "endpoint",
    "logfile",
    "logfd",
    "idle",
    "holdoff",
    "connect-delay",
    "maxfail",
    "persist",
    "demand",
    "ktune",
    "domain",
    "local",
    "pap",
    "chap",
    "mschap",
    "mschap-v2",
    "eap",
    "papcrypt",
    "bsdcomp",
    "deflate",
    "pppoe",
    "netmask",
    "ipv6",
    "ip-up-script",
    "ip-down-script",
    "ip-pre-up-script",
    "linkname",
    "maxconnect",
    "remotenumber",
    "allow-ip",
    "chap-interval",
    "chap-max-challenge",
    "chap-restart",
    "eap-restart",
    "eap-max-auths",
    "pap-restart",
    "pap-max-authreq",
    "pap-timeout",
    "lcp-max-configure",
    "lcp-max-failure",
    "lcp-max-terminate",
    "lcp-restart",
    "ccp-restart",
    "ccp-max-configure",
    "ccp-max-failure",
    "ccp-max-terminate",
    "ipcp-restart",
    "ipcp-max-configure",
    "ipcp-max-failure",
    "ipcp-max-terminate",
    "ipcp-accept-local",
    "ipcp-accept-remote",
    "kdebug",
    "pass-filter",
    "active-filter",
    "receive-all",
    "nopredictor1",
    "predictor1",
    "crtscts",
    "nocrtscts",
    "xonxoff",
    "cdtrcts",
    "modem",
    "local",
    "sync",
    "passive",
    "silent",
    "dtr",
    "detach",
    "nodetach",
    "updetach",
    "init",
    "welcome",
    "pty",
    "notty",
    "record",
    "ifname",
    "dump",
    "dryrun",
    "hide-password",
    "show-password",
    "passwordfd",
    "socket",
    "unit",
    "child-timeout",
];

/// Detect pptpd.conf/options content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let head = s.split_whitespace().next().unwrap_or("");
        if KEYS.contains(&head) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Pptpd {
    /// Census a pptpd.conf/options buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            named: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if head.is_empty() {
                continue;
            }
            c.settings += 1;
            if KEYS.contains(&head) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_conf() {
        let b = b"localip 10.0.0.1\nremoteip 10.0.0.2-9\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_conf() {
        let b = concat!(
            "# pptpd\n",
            "option /etc/ppp/options.pptpd\n",
            "debug\n",
            "localip 10.0.0.1\n",
            "remoteip 10.0.0.100-150\n",
            "stimeout 10\n",
            "name pptpd\n",
            "refuse-pap\n",
            "refuse-chap\n",
            "refuse-mschap\n",
            "require-mschap-v2\n",
            "require-mppe-128\n",
            "ms-dns 8.8.8.8\n",
            "ms-dns 1.1.1.1\n",
            "ms-wins 10.0.0.2\n",
            "proxyarp\n",
            "nodefaultroute\n",
            "nobsdcomp\n",
            "novj\n",
            "lcp-echo-interval 30\n",
            "lcp-echo-failure 4\n",
            "lock\n",
        );
        let c = Pptpd::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 21);
        assert_eq!(c.named, 21);
        assert_eq!(c.comments, 1);
    }
}
