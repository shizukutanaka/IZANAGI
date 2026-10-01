//! ISC `dhclient.conf` の検出・カウント。
//!
//! `send`/`request`/`require`/`supersede`/`prepend`/`append`/`default`/`option`
//! 等ステートメント (`;` 終端)、`interface`/`alias`/`lease`/`script` ブロック、
//! `timeout`/`retry`/`backoff-cutoff`/`initial-interval`/`reboot`/`select-timeout`
//! タイミング系を分類する。
//!
//! ```
//! let cfg = b"timeout 60;\n\
//!             retry 300;\n\
//!             request subnet-mask, broadcast-address, routers;\n\
//!             send host-name \"myhost\";\n\
//!             interface \"eth0\" {\n\
//!               supersede domain-name-servers 127.0.0.1;\n\
//!             }\n";
//! assert!(izanagi_kit::dhclientconf::detect(cfg));
//! let c = izanagi_kit::dhclientconf::parse(cfg).unwrap();
//! assert_eq!(c.requests, 1);
//! ```

/// request/require/also-request 系。
const REQUEST_HEADS: &[&str] = &["request", "require", "also", "reject"];

/// send/option 系。
const SEND_HEADS: &[&str] = &["send", "option"];

/// supersede/prepend/append/default 系。
const OVERRIDE_HEADS: &[&str] = &["supersede", "prepend", "append", "default"];

/// interface/alias/lease/script/media/fixed-address ブロック・宣言系。
const DECL_HEADS: &[&str] = &[
    "interface",
    "alias",
    "lease",
    "script",
    "media",
    "fixed-address",
    "pseudo",
    "failover",
    "key",
    "zone",
    "db",
    "set",
];

/// タイミング/制御系。
const TIMING_HEADS: &[&str] = &[
    "timeout",
    "retry",
    "backoff-cutoff",
    "initial-interval",
    "reboot",
    "select-timeout",
    "initial-delay",
    "expire",
    "preferred-lifetime",
    "dhcp-lease-time",
    "bootp-broadcast-always",
    "do-forward-updates",
    "dhcp-cache-threshold",
    "check-timeout",
    "db-time-format",
    "pid-file",
    "lease-file-name",
    "log-facility",
    "use-host-decl-names",
    "ddns-domainname",
    "ddns-rev-domainname",
    "ddns-update-style",
    "ddns-updates",
    "omapi-port",
    "delayed-ack",
    "max-ack-delay",
    "max-response-delay",
    "mclt",
    "split",
    "load",
    "hba",
    "auto Partner Down",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ステートメント総数。
    pub entries: usize,
    /// `request`/`require`/`also`/`reject` 数。
    pub requests: usize,
    /// `send`/`option` 数。
    pub sends: usize,
    /// `supersede`/`prepend`/`append`/`default` 数。
    pub overrides: usize,
    /// `interface`/`alias`/`lease`/`script`/`media`/`failover` 等ブロック・宣言数。
    pub declarations: usize,
    /// `timeout`/`retry`/`reboot`/`select-timeout`/`backoff-cutoff`/`initial-interval` 等タイミング数。
    pub timing: usize,
    /// `{`/`}` ブロック開始・終了数。
    pub braces: usize,
    /// `option <name> code <n> = <type>` オプション定義数。
    pub option_defs: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が `dhclient.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| {
        c.entries - c.misc >= 2
            && (c.requests >= 1 || c.sends >= 1 || c.overrides >= 1 || c.declarations >= 1)
    })
}

/// `b` を `dhclient.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        requests: 0,
        sends: 0,
        overrides: 0,
        declarations: 0,
        timing: 0,
        braces: 0,
        option_defs: 0,
        misc: 0,
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "{" || line == "}" || line.ends_with('{') && line.starts_with('}') {
            c.braces += 1;
            continue;
        }
        if line.starts_with('{') || line.starts_with('}') {
            c.braces += 1;
        }
        let head = line.split([' ', '\t', ';']).next().unwrap_or("");
        if head.is_empty() {
            continue;
        }
        c.entries += 1;
        if REQUEST_HEADS.contains(&head) {
            c.requests += 1;
        } else if SEND_HEADS.contains(&head) {
            if head == "option" && line.contains("code") && line.contains('=') {
                c.option_defs += 1;
            } else {
                c.sends += 1;
            }
        } else if OVERRIDE_HEADS.contains(&head) {
            c.overrides += 1;
        } else if DECL_HEADS.contains(&head) {
            c.declarations += 1;
        } else if TIMING_HEADS.contains(&head) {
            c.timing += 1;
        } else {
            c.misc += 1;
        }
    }
    if c.entries >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# dhclient.conf\n\
        option rfc3442-classless-static-routes code 121 = array of unsigned integer 8;\n\
        \n\
        send host-name = gethostname();\n\
        request subnet-mask, broadcast-address, time-offset, routers,\n\
                domain-name, domain-name-servers, host-name;\n\
        require subnet-mask, domain-name-servers;\n\
        \n\
        timeout 60;\n\
        retry 300;\n\
        reboot 10;\n\
        select-timeout 5;\n\
        initial-interval 2;\n\
        \n\
        interface \"eth0\" {\n\
          supersede domain-name-servers 127.0.0.1, 8.8.8.8;\n\
          prepend domain-search \"corp.local\";\n\
          send dhcp-client-identifier \"myclient\";\n\
        }\n";

    #[test]
    fn detects_dhclient() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.option_defs, 1);
        assert_eq!(c.sends, 2);
        assert_eq!(c.requests, 2);
        assert_eq!(c.overrides, 2);
        assert_eq!(c.declarations, 1);
        assert_eq!(c.timing, 5);
        assert_eq!(c.braces, 1);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"option x 1;\n"));
    }
}
