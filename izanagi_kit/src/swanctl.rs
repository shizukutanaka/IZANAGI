//! strongSwan `swanctl.conf` census.
//!
//! Nested `key = value`/`name { … }` structure under top-level
//! groups: `connections { <conn> { local { … } remote { … }
//! children { <child> { … } } } }`, `pools { <pool> { addrs = } }`,
//! `secrets { ike-<name> { … } eap-<name> { … } }`,
//! `authorities { <ca> { cacert = } }`, `pools`/`secrets`/`authorities`.
//! Keys: `version`, `addrs`, `local_addrs`, `remote_addrs`,
//! `local_port`, `remote_port`, `proposals`, `vips`, `mobike`,
//! `dscp`, `encap`, `reauth_time`, `rekey_time`, `dpd_delay`,
//! `dpd_timeout`, `fragmentation`, `send_certreq`, `send_cert`,
//! `keyingtries`, `unique`, `aggressive`, `pull`, `mediation`,
//! `mediated_by`, `peer_id`, `ppk_id`, `ppk_required`,
//! `auth`, `id`, `eap_id`, `aaa_id`, `xauth_id`, `certs`,
//! `pubkeys`, `cacerts`, `revocation`, `round`, `local_ts`,
//! `remote_ts`, `updown`, `esp_proposals`, `ah_proposals`,
//! `lifetime`, `rekey_bytes`, `rekey_packets`, `rand_time`,
//! `life_packets`, `life_bytes`, `inactivity`, `mode`,
//! `policies`, `hw_offload`, `if_id_in`, `if_id_out`,
//! `mark_in`, `mark_out`, `mark_in_sa`, `set_in`, `set_out`,
//! `priority`, `interface`, `man_id`, `start_action`,
//! `close_action`, `dpd_action`, `trap_policy`,
//! `copy_ts`, `net`, `subnet`, `dns`, `nbns`, `wins`,
//! `split_include`, `split_exclude`, `24843`, `28671`,
//! `28672`, `28673`, `28674`, `unity_*`, `xauth_*`,
//! `eap_*`, `ike_*`, `ppp_*`, `rsa-*`, `ecp-*`, `cert-*`,
//! `pubkey-*`, `bliss-*`, `pin-*`, `ike_name`,
//! `cacert`, `cert_uri_base`, `crl_uris`, `ocsp_uris`,
//! `aaa_cert`, `imca`/`imcv`.
//!
//! ```rust
//! let s = "connections {\n  home {\n    remote_addrs = gw.example.org\n    local { auth = pubkey }\n    children { net { local_ts = 10.0.0.0/24 } }\n  }\n}\n";
//! let c = izanagi_kit::swanctl::Swanctl::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.connections, 1);
//! ```

use crate::textutil::strip_bom;
/// swanctl.conf census.
#[derive(Debug, Clone)]
pub struct Swanctl {
    /// `connections {` blocks.
    pub connections: usize,
    /// Named sub-blocks inside `connections` (`local`/`remote`/`children`/conn name).
    pub sub_blocks: usize,
    /// `pools {`/`secrets {`/`authorities {` other groups.
    pub other_groups: usize,
    /// `key = value` lines.
    pub settings: usize,
    /// Recognised swanctl keys.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_GROUPS: &[&str] = &[
    "connections",
    "pools",
    "secrets",
    "authorities",
    "include",
    "lib",
];

const KEYS: &[&str] = &[
    "version",
    "addrs",
    "local_addrs",
    "remote_addrs",
    "local_port",
    "remote_port",
    "proposals",
    "vips",
    "mobike",
    "dscp",
    "encap",
    "reauth_time",
    "rekey_time",
    "dpd_delay",
    "dpd_timeout",
    "fragmentation",
    "send_certreq",
    "send_cert",
    "keyingtries",
    "unique",
    "aggressive",
    "pull",
    "mediation",
    "mediated_by",
    "peer_id",
    "ppk_id",
    "ppk_required",
    "auth",
    "id",
    "eap_id",
    "aaa_id",
    "xauth_id",
    "certs",
    "pubkeys",
    "cacerts",
    "revocation",
    "round",
    "local_ts",
    "remote_ts",
    "updown",
    "esp_proposals",
    "ah_proposals",
    "lifetime",
    "rekey_bytes",
    "rekey_packets",
    "rand_time",
    "life_packets",
    "life_bytes",
    "inactivity",
    "mode",
    "policies",
    "hw_offload",
    "if_id_in",
    "if_id_out",
    "mark_in",
    "mark_out",
    "mark_in_sa",
    "set_in",
    "set_out",
    "priority",
    "interface",
    "man_id",
    "start_action",
    "close_action",
    "dpd_action",
    "trap_policy",
    "net",
    "subnet",
    "dns",
    "nbns",
    "wins",
    "split_include",
    "split_exclude",
    "ike_name",
    "cacert",
    "cert_uri_base",
    "crl_uris",
    "ocsp_uris",
    "aaa_cert",
    "secret",
    "id1",
    "id2",
    "id3",
    "rsa_key",
    "ecp_key",
    "private_key",
    "bliss_key",
    "pkcs8",
    "pin",
    "ike",
    "esp",
    "ah",
];

/// Detect swanctl.conf content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut groups = 0usize;
    let mut named = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if s.ends_with('{') {
            let head = s.trim_end_matches('{').trim();
            if TOP_GROUPS.contains(&head) {
                groups += 1;
                continue;
            }
        }
        if let Some(eq) = s.find('=') {
            let key = s[..eq].trim();
            if KEYS.contains(&key) {
                named += 1;
            }
        }
    }
    groups >= 1 && named >= 1
}

impl Swanctl {
    /// Census a swanctl.conf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            connections: 0,
            sub_blocks: 0,
            other_groups: 0,
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
            if s == "}" || s == "}," {
                continue;
            }
            if s.ends_with('{') {
                let head = s.trim_end_matches('{').trim();
                if TOP_GROUPS.contains(&head) {
                    if head == "connections" {
                        c.connections += 1;
                    } else {
                        c.other_groups += 1;
                    }
                } else {
                    c.sub_blocks += 1;
                }
                continue;
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].trim();
                if !key.is_empty()
                    && key
                        .bytes()
                        .all(|x| x.is_ascii_alphanumeric() || x == b'_' || x == b'-')
                {
                    c.settings += 1;
                    if KEYS.contains(&key) {
                        c.named += 1;
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
    fn detects_conf() {
        let b = b"connections {\n  home {\n    remote_addrs = gw\n";
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
            "# swanctl\n",
            "connections {\n",
            "  home {\n",
            "    remote_addrs = gw.example.org\n",
            "    vips = 0.0.0.0\n",
            "    proposals = aes256gcm16-prfsha256-ecp256\n",
            "    version = 2\n",
            "    mobike = no\n",
            "    local {\n",
            "      auth = pubkey\n",
            "      certs = me.crt\n",
            "      id = me@example.org\n",
            "    }\n",
            "    remote {\n",
            "      auth = pubkey\n",
            "      id = gw@example.org\n",
            "    }\n",
            "    children {\n",
            "      net {\n",
            "        local_ts = 10.0.0.0/24\n",
            "        remote_ts = 0.0.0.0/0\n",
            "        esp_proposals = aes256gcm16-ecp256\n",
            "        start_action = start\n",
            "      }\n",
            "    }\n",
            "  }\n",
            "}\n",
            "pools {\n",
            "  rw {\n",
            "    addrs = 10.10.0.0/24\n",
            "    dns = 1.1.1.1\n",
            "  }\n",
            "}\n",
            "secrets {\n",
            "  ike-home {\n",
            "    secret = strong-secret\n",
            "  }\n",
            "}\n",
        );
        let c = Swanctl::parse(b.as_bytes()).unwrap();
        assert_eq!(c.connections, 1);
        assert_eq!(c.other_groups, 2);
        assert_eq!(c.sub_blocks, 7);
        assert_eq!(c.settings, 17);
        assert!(c.named >= 15);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
