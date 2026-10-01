//! OpenNTPD `ntpd.conf` の検出・カウント。
//!
//! `listen on <addr> [port <n>]`、`servers <hosts> [weight <n>]`、`server <host>`、
//! `pool <host>`、`sensor <dev> [correction <n>] [weight <n>] [refid <s>] [stratum <n>]`、
//! `constraints from <url>`、`constraint from <url>`、`trusted`。
//!
//! ```
//! let cfg = b"listen on 127.0.0.1\n\
//!             listen on ::1\n\
//!             servers pool.ntp.org\n\
//!             sensor nmea0\n\
//!             constraints from \"https://www.google.com\"\n";
//! assert!(izanagi_kit::openntpd::detect(cfg));
//! let c = izanagi_kit::openntpd::parse(cfg).unwrap();
//! assert_eq!(c.servers, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ディレクティブ行総数。
    pub entries: usize,
    /// `listen on` 数。
    pub listen: usize,
    /// `server`/`servers`/`pool` 数。
    pub servers: usize,
    /// `sensor` 数。
    pub sensors: usize,
    /// `constraints from`/`constraint from` 数。
    pub constraints: usize,
    /// `trusted`/`query`/`include`/`log` 等その他既知数。
    pub misc_known: usize,
    /// ソース/センサ行内オプション数 (`weight`/`correction`/`refid`/`stratum`/`port`/`trusted`)。
    pub options: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が OpenNTPD `ntpd.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を OpenNTPD `ntpd.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        listen: 0,
        servers: 0,
        sensors: 0,
        constraints: 0,
        misc_known: 0,
        options: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if let Some(rest) = line.strip_prefix("listen on") {
            c.listen += 1;
            known += 1;
            for opt in rest.split_whitespace().skip(1) {
                if opt == "port" {
                    c.options += 1;
                }
            }
            continue;
        }
        if line.starts_with("server ") || line.starts_with("servers ") || line.starts_with("pool ")
        {
            c.servers += 1;
            known += 1;
            for opt in line.split_whitespace().skip(2) {
                if matches!(opt, "weight" | "trusted" | "query" | "port") {
                    c.options += 1;
                }
            }
            continue;
        }
        if line.starts_with("sensor ") {
            c.sensors += 1;
            known += 1;
            for opt in line.split_whitespace().skip(2) {
                if matches!(
                    opt,
                    "correction" | "weight" | "refid" | "stratum" | "trusted"
                ) {
                    c.options += 1;
                }
            }
            continue;
        }
        if line.starts_with("constraints from") || line.starts_with("constraint from") {
            c.constraints += 1;
            known += 1;
            continue;
        }
        if matches!(
            line.split_whitespace().next().unwrap_or(""),
            "trusted"
                | "query"
                | "include"
                | "log"
                | "rtable"
                | "ntpd"
                | "run"
                | "constraint"
                | "sensor"
                | "servers"
                | "listen"
                | "pool"
                | "daemon"
        ) {
            c.misc_known += 1;
            known += 1;
        } else {
            c.misc += 1;
        }
    }
    if known >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# $OpenBSD: ntpd.conf,v 1.16\n\
        listen on 127.0.0.1\n\
        listen on ::1\n\
        listen on 192.168.1.1 port 123\n\
        \n\
        servers pool.ntp.org\n\
        servers ntp1.example.com weight 5\n\
        server ntp2.example.com trusted\n\
        \n\
        sensor nmea0\n\
        sensor ucf0 correction 40700 weight 2\n\
        \n\
        constraints from \"https://www.google.com\"\n\
        constraint from \"https://www.apple.com\"\n";

    #[test]
    fn detects_openntpd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 10);
        assert_eq!(c.listen, 3);
        assert_eq!(c.servers, 3);
        assert_eq!(c.sensors, 2);
        assert_eq!(c.constraints, 2);
        assert_eq!(c.options, 5);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"listen on 127.0.0.1\n"));
    }
}
