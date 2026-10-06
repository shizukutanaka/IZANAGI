//! `ipsec.conf` (strongSwan/libreswan) 検出モジュール。
//!
//! IPsec VPN の設定は `config setup`/`conn 名前` セクションヘッダと
//! `left`/`right`/`leftsubnet`/`rightsubnet`/`keyexchange`/`ike`/
//! `esp`/`authby`/`auto`/`leftid`/`rightid`/`leftcert`/`dpdaction`/
//! `type`/`keylife`/`lifetime`/`aggrmode`/`rekey`/`mobike` 等の
//! `key=value` パラメータ(継続行はインデント)で構成される。
//!
//! ```
//! let b = br#"config setup
//!     charondebug="ike 1, knl 1"
//! conn site-to-site
//!     left=192.0.2.1
//!     leftsubnet=10.1.0.0/16
//!     right=198.51.100.1
//!     rightsubnet=10.2.0.0/16
//!     auto=start
//! "#;
//! let c = izanagi_kit::strongswanconf::parse(b);
//! assert!(izanagi_kit::strongswanconf::detect(b));
//! assert_eq!(c.connections, 2);
//! ```

const PARAMS: &[&str] = &[
    "aggrmode",
    "authby",
    "auto",
    "ca",
    "cert",
    "compress",
    "connmark",
    "dpdaction",
    "dpddelay",
    "dpdtimeout",
    "esp",
    "espdhgroup",
    "espfrag",
    "espheader",
    "forceencaps",
    "fragmentation",
    "ike",
    "ikedhgroup",
    "ikelifetime",
    "ikev1",
    "ikev2",
    "installpolicy",
    "keychannel",
    "keyexchange",
    "keylife",
    "keyingtries",
    "left",
    "leftauth",
    "leftauth2",
    "leftca",
    "leftcert",
    "leftcertpolicy",
    "leftdns",
    "leftfirewall",
    "leftgroups",
    "lefthostaccess",
    "leftid",
    "leftikeport",
    "leftnexthop",
    "leftprotoport",
    "leftsigkey",
    "leftsourceip",
    "leftsubnet",
    "leftsubnetwithin",
    "leftupdown",
    "lifetime",
    "margintime",
    "mark",
    "mobike",
    "modecfgpull",
    "narrowing",
    "passthrough",
    "phase2",
    "phase2alg",
    "pfs",
    "pfsgroup",
    "reauth",
    "rekey",
    "rekeyfuzz",
    "replay_window",
    "reqid",
    "right",
    "rightauth",
    "rightca",
    "rightcert",
    "rightdns",
    "rightfirewall",
    "rightgroups",
    "righthostaccess",
    "rightid",
    "rightikeport",
    "rightnexthop",
    "rightprotoport",
    "rightsourceip",
    "rightsubnet",
    "rightsubnetwithin",
    "rightupdown",
    "salifetime",
    "sec_label",
    "type",
    "xauth",
];

fn is_conn_line(t: &str) -> bool {
    t.starts_with("conn ") || t == "conn"
}

fn is_config_setup(t: &str) -> bool {
    t == "config setup" || t == "config"
}

fn is_param(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    PARAMS.contains(&k)
}

/// `b` が ipsec.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut conns = 0usize;
    let mut params = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_conn_line(tr) || is_config_setup(tr) {
            conns += 1;
        } else if is_param(tr) {
            params += 1;
        }
    }
    (conns >= 1 && params >= 2) || params >= 5
}

/// ipsec.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct StrongswanConf {
    /// `conn`/`config` セクションヘッダ数。
    pub connections: usize,
    /// パラメータ行数。
    pub params: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を ipsec.conf として統計する。
pub fn parse(b: &[u8]) -> StrongswanConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = StrongswanConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_conn_line(tr) || is_config_setup(tr) {
            c.connections += 1;
        } else if is_param(tr) {
            c.params += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"config setup
conn site
    left=1.2.3.4
    right=5.6.7.8
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.connections, 2);
    }

    #[test]
    fn detects_params_only() {
        let b = br#"conn a
    left=1
    right=2
    leftsubnet=10.0.0.0/8
    rightsubnet=10.1.0.0/8
    ike=aes256-sha256-modp2048!
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"conn site\n    left=1\n"));
        assert!(!detect(b"left = 1\nright = 2\nike = x\nesp = y\n"));
        assert!(!detect(b"[section]\nkey = value\nfoo = bar\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.connections, 0);
    }
}
