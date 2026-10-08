//! OpenVPN `.ovpn`/`openvpn.conf` census.
//!
//! Whitespace-separated directive lines (`client`, `dev tun`,
//! `proto udp`, `remote host port`, `resolv-retry infinite`,
//! `nobind`, `persist-key`, `persist-tun`, `ca`, `cert`, `key`,
//! `tls-auth`, `tls-crypt`, `cipher`, `data-ciphers`, `auth`,
//! `comp-lzo`, `compress`, `verb`, `remote-cert-tls`,
//! `auth-user-pass`, `redirect-gateway`, `dhcp-option`,
//! `ping-restart`, `keepalive`, `explicit-exit-notify`,
//! `key-direction`, `setenv`, `script-security`, `up`, `down`,
//! `management`, `log`, `status`, `port`, `server`,
//! `ifconfig-pool-persist`, `client-config-dir`, `client-connect`,
//! `duplicate-cn`, `max-clients`, `push`, `route`,
//! `route-gateway`, `topology`, `sndbuf`, `rcvbuf`, `tun-mtu`,
//! `mssfix`, `fragment`, `fast-io`, `float`, `mute`,
//! `reneg-sec`, `handshake-window`, `tls-timeout`, `ping`,
//! `ping-exit`, `inactive`, `socks-proxy`, `http-proxy`,
//! `connect-retry`, `connect-timeout`, `remote-random`,
//! `verify-x509-name`, `remote-random-hostname`,
//! `allow-pull-fqdn`, `block-outside-dns`, `pull`,
//! `push-peer-info`, `auth-nocache`, `tls-version-min`,
//! `tls-version-max`, `tls-groups`, `providers`,
//! `client-nat`, `tun-ipv6`, `route-ipv6`, `server-ipv6`,
//! `ifconfig-ipv6-push`, `tmp-dir`, `cd`, `chroot`, `user`,
//! `group`, `writepid`, `daemon`, `inetd`, `up-delay`,
//! `down-pre`, `up-restart`, `learn-address`,
//! `auth-user-pass-verify`, `client-cert-not-required`,
//! `username-as-common-name`, `verify-client-cert`,
//! `tls-verify`, `tls-export-cert`, `plugin`,
//! `management-client-auth`, `verify-hash`,
//! `session-keyder-dyn`, `engine`, `cap`, `ipchange`,
//! `mode`, `tls-server`, `tls-client`, `multihome`,
//! `mtu-test`, `ifconfig`, `ifconfig-noexec`,
//! `ifconfig-nowarn`, `iroute`, `iroute-ipv6`,
//! `stale-routes-check`, `disable-occ`, `ccd-exclusive`,
//! `opt-verify`, `auth-retry`, `cr-response`,
//! `crl-verify`, `bcast-buffers`, `tcp-queue-limit`,
//! `tcp-nodelay`, `shaper`, `nice`, `echo`,
//! `cryptoapicert`, `pkcs11-*`, `management-hold`,
//! `management-log-cache`, `management-query-passwords`,
//! `management-forget-disconnect`, `management-signal`,
//! `management-client`, `management-external-key`,
//! `management-external-cert`, `management-up-down`,
//! `token-renewal`, `secret`, `x509-username-field`,
//! `x509-track`, `ns-cert-type`, `remote-cert-eku`,
//! `remote-cert-ku`, `remote-cert-tls`,
//! `keying-material-exporter`, `tran-window`,
//! `single-session`, `tls-exit`, `replay-window`,
//! `replay-persist`, `test-crypto`, `key-method`,
//! `compat-mode`, `compat-names`, `no-name-remapping`,
//! `tls-crypt-v2`, `tls-crypt-v2-verify`,
//! `peer-fingerprint`, `peer-id`, `ip-win32`,
//! `dhcp-renew`, `dhcp-release`, `register-dns`,
//! `win-sys`, `route-method`, `route-delay`,
//! `allow-nonadmin`, `show-net-up`, `dhcp-pre-release`,
//! `tap-sleep`, `pause-exit`, `service`,
//! `show-adapters`, `show-valid-subnets`,
//! `show-net`, `show-gateway`, `show-pkcs11`,
//! `show-ciphers`, `show-digests`, `show-engines`,
//! `show-groups`, `show-tls`, `askpass`,
//! `auth-gen-token`, `auth-gen-token-secret`,
//! `cert-eku`, `data-ciphers-fallback`,
//! `disable`, `down-root`, `echo`,
//! `gremlin`, `ip-remote-hint`, `knock`,
//! `push-continuation`, `remote-cert-is`,
//! `route-pre-down`, `rrate-limit`,
//! `static-challenge`, `suppress-timestamps`,
//! `windows-driver`, `caand`, `keysize`,
//! `prng`, `ns-cert`, `shared-secret`).
//!
//! Inline blocks `<ca>…</ca>`, `<cert>`, `<key>`, `<tls-auth>`,
//! `<tls-crypt>`, `<extra-certs>`, `<pkcs12>`, `<secret>`,
//! `<dh>`, `<connection>` embed material.
//!
//! ```rust
//! let o = "client\ndev tun\nproto udp\nremote vpn.example.org 1194\n<ca>\nDATA\n</ca>\n";
//! let c = izanagi_kit::openvpn::Openvpn::parse(o.as_bytes()).unwrap();
//! assert_eq!(c.directives, 4);
//! assert_eq!(c.inline_blocks, 1);
//! ```

/// openvpn config census.
#[derive(Debug, Clone)]
pub struct Openvpn {
    /// Directive lines.
    pub directives: usize,
    /// Recognised directive names.
    pub named: usize,
    /// `<tag>` inline blocks.
    pub inline_blocks: usize,
    /// `remote` directives.
    pub remotes: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "client",
    "dev",
    "dev-type",
    "dev-node",
    "proto",
    "remote",
    "remote-random",
    "remote-random-hostname",
    "remote-cert-tls",
    "remote-cert-is",
    "remote-cert-eku",
    "remote-cert-ku",
    "resolv-retry",
    "nobind",
    "persist-key",
    "persist-tun",
    "ca",
    "cert",
    "key",
    "tls-auth",
    "tls-crypt",
    "tls-crypt-v2",
    "cipher",
    "data-ciphers",
    "data-ciphers-fallback",
    "auth",
    "comp-lzo",
    "compress",
    "verb",
    "auth-user-pass",
    "auth-nocache",
    "redirect-gateway",
    "dhcp-option",
    "ping-restart",
    "keepalive",
    "explicit-exit-notify",
    "key-direction",
    "setenv",
    "script-security",
    "up",
    "down",
    "up-delay",
    "down-pre",
    "up-restart",
    "management",
    "log",
    "log-append",
    "status",
    "status-version",
    "port",
    "server",
    "server-bridge",
    "server-ipv6",
    "ifconfig-pool",
    "ifconfig-pool-persist",
    "client-config-dir",
    "client-connect",
    "client-disconnect",
    "duplicate-cn",
    "max-clients",
    "push",
    "route",
    "route-gateway",
    "route-method",
    "route-delay",
    "route-nopull",
    "route-ipv6",
    "topology",
    "sndbuf",
    "rcvbuf",
    "tun-mtu",
    "tun-ipv6",
    "mssfix",
    "fragment",
    "fast-io",
    "float",
    "mute",
    "mute-replay-warnings",
    "reneg-sec",
    "handshake-window",
    "tran-window",
    "tls-timeout",
    "ping",
    "ping-exit",
    "inactive",
    "socks-proxy",
    "socks-proxy-retry",
    "http-proxy",
    "http-proxy-retry",
    "http-proxy-option",
    "http-proxy-timeout",
    "connect-retry",
    "connect-timeout",
    "connect-retry-max",
    "verify-x509-name",
    "allow-pull-fqdn",
    "block-outside-dns",
    "pull",
    "push-peer-info",
    "tls-version-min",
    "tls-version-max",
    "tls-groups",
    "tls-ciphersuites",
    "providers",
    "client-nat",
    "ifconfig-ipv6-push",
    "tmp-dir",
    "cd",
    "chroot",
    "user",
    "group",
    "writepid",
    "daemon",
    "inetd",
    "learn-address",
    "auth-user-pass-verify",
    "client-cert-not-required",
    "username-as-common-name",
    "verify-client-cert",
    "tls-verify",
    "tls-export-cert",
    "plugin",
    "verify-hash",
    "engine",
    "ipchange",
    "mode",
    "tls-server",
    "tls-client",
    "mtu-test",
    "ifconfig",
    "ifconfig-noexec",
    "ifconfig-nowarn",
    "iroute",
    "iroute-ipv6",
    "stale-routes-check",
    "disable-occ",
    "ccd-exclusive",
    "opt-verify",
    "auth-retry",
    "cr-response",
    "crl-verify",
    "bcast-buffers",
    "tcp-queue-limit",
    "tcp-nodelay",
    "shaper",
    "nice",
    "echo",
    "cryptoapicert",
    "askpass",
    "auth-gen-token",
    "cert-eku",
    "disable",
    "static-challenge",
    "suppress-timestamps",
    "windows-driver",
    "keysize",
    "prng",
    "ns-cert-type",
    "keying-material-exporter",
    "single-session",
    "tls-exit",
    "replay-window",
    "replay-persist",
    "test-crypto",
    "key-method",
    "compat-mode",
    "compat-names",
    "no-name-remapping",
    "peer-fingerprint",
    "peer-id",
    "ip-win32",
    "dhcp-renew",
    "dhcp-release",
    "register-dns",
    "win-sys",
    "secret",
    "x509-username-field",
    "management-hold",
    "management-client",
    "management-log-cache",
    "management-query-passwords",
    "management-forget-disconnect",
    "management-signal",
    "management-external-key",
    "management-external-cert",
    "management-client-auth",
    "token-renewal",
];

const BLOCK_TAGS: &[&str] = &[
    "ca",
    "cert",
    "key",
    "dh",
    "tls-auth",
    "tls-crypt",
    "tls-crypt-v2",
    "extra-certs",
    "pkcs12",
    "secret",
    "connection",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect openvpn config content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
            continue;
        }
        if s.starts_with('<') && s.ends_with('>') && !s.starts_with("</") {
            let tag = &s[1..s.len() - 1];
            if BLOCK_TAGS.contains(&tag) {
                hits += 1;
                continue;
            }
        }
        let head = s.split_whitespace().next().unwrap_or("");
        if KEYS.contains(&head) {
            hits += 1;
        }
    }
    hits >= 3
}

impl Openvpn {
    /// Census an openvpn config buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            directives: 0,
            named: 0,
            inline_blocks: 0,
            remotes: 0,
            comments: 0,
        };
        let mut in_block = false;
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if in_block {
                if s.starts_with("</") {
                    in_block = false;
                }
                continue;
            }
            if s.starts_with('<') && s.ends_with('>') && !s.starts_with("</") {
                let tag = &s[1..s.len() - 1];
                if BLOCK_TAGS.contains(&tag) {
                    c.inline_blocks += 1;
                    in_block = true;
                    continue;
                }
            }
            c.directives += 1;
            let head = s.split_whitespace().next().unwrap_or("");
            if head == "remote" {
                c.remotes += 1;
            }
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
    fn detects_ovpn() {
        let b = b"client\ndev tun\nproto udp\nremote vpn.example.org 1194\n";
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
            "# client conf\n",
            "client\n",
            "dev tun\n",
            "proto udp\n",
            "remote vpn.example.org 1194\n",
            "remote vpn2.example.org 1194\n",
            "resolv-retry infinite\n",
            "nobind\n",
            "persist-key\n",
            "persist-tun\n",
            "remote-cert-tls server\n",
            "cipher AES-256-GCM\n",
            "auth SHA256\n",
            "auth-user-pass\n",
            "comp-lzo no\n",
            "verb 3\n",
            "key-direction 1\n",
            "<ca>\n",
            "-----BEGIN CERTIFICATE-----\n",
            "DATA\n",
            "-----END CERTIFICATE-----\n",
            "</ca>\n",
            "<tls-auth>\n",
            "KEYDATA\n",
            "</tls-auth>\n",
        );
        let c = Openvpn::parse(b.as_bytes()).unwrap();
        assert_eq!(c.directives, 16);
        assert_eq!(c.named, 16);
        assert_eq!(c.inline_blocks, 2);
        assert_eq!(c.remotes, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
