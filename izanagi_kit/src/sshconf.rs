//! OpenSSH `ssh_config` / `sshd_config` client-server configuration format.
//!
//! ssh_config lines are `Keyword value` or `Keyword=value`; `Host pattern`
//! opens a host block, `Match` opens a criteria block, `Include` pulls
//! other files, `#` comments.
//!
//! ```
//! let b = concat!(
//!     "Host *\n",
//!     "  ServerAliveInterval 60\n",
//!     "Host jump\n",
//!     "  HostName 192 0 2 1\n",
//!     "  User deploy\n",
//!     "  Port 2222\n",
//!     "Match user admin\n",
//!     "  PasswordAuthentication yes\n"
//! ).as_bytes();
//! assert!(izanagi_kit::sshconf::detect(b));
//! let c = izanagi_kit::sshconf::Sshconf::parse(b).unwrap();
//! assert_eq!(c.hosts, 2);
//! ```

/// Parsed ssh_config summary.
#[derive(Debug, Clone)]
pub struct Sshconf {
    /// `Host` blocks (host patterns count separately in `patterns`).
    pub hosts: usize,
    /// Total host patterns across `Host` lines.
    pub patterns: usize,
    /// `Match` blocks.
    pub matches: usize,
    /// `Include` directives.
    pub includes: usize,
    /// Setting lines (keyword value / keyword=value).
    pub settings: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
    /// Forward/tunnel options (`LocalForward`, `RemoteForward`, `DynamicForward`, `Tunnel`, `PermitLocalCommand`, `ClearAllForwardings`, `SendEnv`, `SetEnv`, `ProxyCommand`, `ProxyJump`, `ProxyUseFdpass`, `IdentityAgent`, `PKCS11Provider`, `SecurityKeyProvider`, `EscapeChar`, `LogLevel`).
    pub tunnels: usize,
    /// Auth options (`IdentityFile`, `CertificateFile`, `User`, `PasswordAuthentication`, `PubkeyAuthentication`, `KbdInteractiveAuthentication`, `GSSAPIAuthentication`, `HostbasedAuthentication`, `PreferredAuthentications`, `IdentitiesOnly`, `ChallengeResponseAuthentication`, `UseBlacklistedKeys`, `BatchMode`, `NumberOfPasswordPrompts`, `EnableSSHKeysign`).
    pub auth: usize,
    /// Crypto options (`Ciphers`, `MACs`, `KexAlgorithms`, `HostKeyAlgorithms`, `PubkeyAcceptedKeyTypes`, `CASignatureAlgorithms`, `RekeyLimit`, `FipsMode`, `CheckHostIP`, `VerifyHostKeyDNS`, `HashKnownHosts`, `VisualHostKey`, `UpdateHostKeys`, `StrictHostKeyChecking`, `GlobalKnownHostsFile`, `UserKnownHostsFile`, `RevokedHostKeys`).
    pub crypto: usize,
    /// Conn options (`Port`, `HostName`, `AddressFamily`, `BindAddress`, `BindInterface`, `ConnectTimeout`, `ConnectionAttempts`, `TCPKeepAlive`, `IPQoS`, `RequestTTY`, `SessionType`, `RemoteCommand`, `LocalCommand`, `PermitLocalCommand`, `Compression`, `ServerAliveInterval`, `ServerAliveCountMax`, `ControlMaster`, `ControlPath`, `ControlPersist`, `ForwardAgent`, `ForwardX11`, `ForwardX11Trusted`, `ForwardX11Timeout`, `GatewayPorts`, `ExitOnForwardFailure`, `RemoteForward`, `LocalForward`, `DynamicForward`, `Tunnel`, `TunnelDevice`, `CanonicalizeHostname`, `CanonicalDomains`, `CanonicalizeFallbackLocal`, `CanonicalizeMaxDots`, `CanonicalizePermittedCNAMEs`, `CheckHostIP`, `FingerprintHash`, `AddKeysToAgent`, `ForwardAgent`, `IgnoreUnknown`, `SyslogFacility`, `LogVerbose`, `NoHostAuthenticationForLocalhost`, `StreamLocalBindMask`, `StreamLocalBindUnlink`, `IgnoreUnknown`, `SendEnv`, `SetEnv`, `EscapeChar`, `PermitRemoteOpen`, `RequiredRSASize`, `EnableEscapeCommandline`, `Match`).
    pub connection: usize,
}

const TUNNEL_KW: &[&str] = &[
    "localforward",
    "remoteforward",
    "dynamicforward",
    "tunnel",
    "tunneldevice",
    "proxycommand",
    "proxyjump",
    "proxyusefdpass",
    "gatewayports",
    "exitonforwardfailure",
    "forwardagent",
    "forwardx11",
    "forwardx11trusted",
    "forwardx11timeout",
    "streamlocalbindmask",
    "streamlocalbindunlink",
    "permitremoteopen",
    "clearallforwardings",
];
const AUTH_KW: &[&str] = &[
    "identityfile",
    "certificatefile",
    "user",
    "passwordauthentication",
    "pubkeyauthentication",
    "kbdinteractiveauthentication",
    "gssapiauthentication",
    "hostbasedauthentication",
    "preferredauthentications",
    "identitiesonly",
    "challengeresponseauthentication",
    "batchmode",
    "numberofpasswordprompts",
    "enablesshkeysign",
    "identityagent",
    "pkcs11provider",
    "securitykeyprovider",
    "addkeystoagent",
    "ignoreunknown",
    "canonicalizehostname",
    "canonicaldomains",
];
const CRYPTO_KW: &[&str] = &[
    "ciphers",
    "macs",
    "kexalgorithms",
    "hostkeyalgorithms",
    "pubkeyacceptedkeytypes",
    "casignaturealgorithms",
    "rekeylimit",
    "fipsmode",
    "fingerprinthash",
    "requiredrsasize",
];
const CONN_KW: &[&str] = &[
    "port",
    "hostname",
    "addressfamily",
    "bindaddress",
    "bindinterface",
    "connecttimeout",
    "connectionattempts",
    "tcpkeepalive",
    "ipqos",
    "requesttty",
    "sessiontype",
    "remotecommand",
    "localcommand",
    "permitlocalcommand",
    "compression",
    "serveraliveinterval",
    "serveralivecountmax",
    "controlmaster",
    "controlpath",
    "controlpersist",
    "checkhostip",
    "verifyhostkeydns",
    "hashknownhosts",
    "visualhostkey",
    "updatehostkeys",
    "stricthostkeychecking",
    "globalknownhostsfile",
    "userknownhostsfile",
    "revokedhostkeys",
    "escapechar",
    "loglevel",
    "syslogfacility",
    "logverbose",
    "nohostauthenticationforlocalhost",
    "sendenv",
    "setenv",
    "enableescapecommandline",
    "canonicalizefallbacklocal",
    "canonicalizemaxdots",
    "canonicalizepermittedcnames",
];

fn kw_of(tr: &str) -> &str {
    tr.split(|c: char| c.is_whitespace() || c == '=')
        .next()
        .unwrap_or("")
}

/// Whether the buffer looks like ssh_config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        let kw = kw_of(tr).to_ascii_lowercase();
        if matches!(kw.as_str(), "host" | "match" | "include")
            || TUNNEL_KW.contains(&kw.as_str())
            || AUTH_KW.contains(&kw.as_str())
            || CRYPTO_KW.contains(&kw.as_str())
            || CONN_KW.contains(&kw.as_str())
        {
            score += 1;
        }
    }
    score >= 2
}

impl Sshconf {
    /// Parses an ssh_config summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            hosts: 0,
            patterns: 0,
            matches: 0,
            includes: 0,
            settings: 0,
            comments: 0,
            tunnels: 0,
            auth: 0,
            crypto: 0,
            connection: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let kw = kw_of(tr).to_ascii_lowercase();
            match kw.as_str() {
                "host" => {
                    c.hosts += 1;
                    c.patterns += tr[kw_of(tr).len()..].split_whitespace().count();
                }
                "match" => c.matches += 1,
                "include" => c.includes += 1,
                _ => {
                    c.settings += 1;
                    if TUNNEL_KW.contains(&kw.as_str()) {
                        c.tunnels += 1;
                    } else if AUTH_KW.contains(&kw.as_str()) {
                        c.auth += 1;
                    } else if CRYPTO_KW.contains(&kw.as_str()) {
                        c.crypto += 1;
                    } else if CONN_KW.contains(&kw.as_str()) {
                        c.connection += 1;
                    }
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
    fn parses_sshconf() {
        let b = concat!(
            "Host *\n",
            "  ServerAliveInterval 60\n",
            "  ControlMaster auto\n",
            "Host jump bastion\n",
            "  HostName 192 0 2 1\n",
            "  User deploy\n",
            "  Port 2222\n",
            "  IdentityFile ~/.ssh/id_ed25519\n",
            "  ProxyCommand ssh -W %h:%p bastion\n",
            "Match user admin\n",
            "  PasswordAuthentication yes\n",
            "Include ~/.ssh/extra\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Sshconf::parse(b).unwrap();
        assert_eq!(c.hosts, 2);
        assert_eq!(c.patterns, 3);
        assert_eq!(c.matches, 1);
        assert_eq!(c.includes, 1);
        assert_eq!(c.tunnels, 1);
        assert_eq!(c.auth, 3);
        assert_eq!(c.connection, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"foo bar\nbaz qux\n"));
        assert!(Sshconf::parse(b"x").is_none());
    }
}
