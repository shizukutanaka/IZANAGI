//! NTPsec `ntp.conf` / `ntp.d/*.conf` の検出・カウント。
//!
//! クラシック ntp.conf に `nts`/`refclock`/`leapsmearinterval`/`mspps` 等
//! NTPsec 拡張ディレクティブを含む形式。
//!
//! ```
//! let cfg = b"server ntp.example.com iburst\n\
//!             refclock shm unit 0 refid GPS\n\
//!             nts enable\n\
//!             nts ca /etc/ssl/certs/ca.crt\n\
//!             driftfile /var/lib/ntp/drift\n";
//! assert!(izanagi_kit::ntpsec::detect(cfg));
//! let c = izanagi_kit::ntpsec::parse(cfg).unwrap();
//! assert_eq!(c.nts, 2);
//! ```

/// refclock ドライバ (`refclock <driver>`)。
const REFCLOCK_DRIVERS: &[&str] = &[
    "shm",
    "local",
    "generic",
    "gpsd",
    "nmea",
    "modem",
    "mxff",
    "jupiter",
    "palisade",
    "trimble",
    "oncore",
    "spectracom",
    "truetime",
    "arbiter",
    "arbitron",
    "as2201",
    "atom",
    "bancomm",
    "chrono",
    "chu",
    "datacut",
    "digi",
    "dsys",
    "fen",
    "gpsclock",
    "heath",
    "hopf",
    "hpj",
    "hpgps",
    "irig",
    "jjy",
    "leitch",
    "litronic",
    "logiscan",
    "magnavox",
    "mx4200",
    "ntpshm",
    "oncore2",
    "parse",
    "pcf",
    "pst",
    "rcc",
    "synchr",
    "tpro",
    "ulink",
    "wwvb",
    "z3801a",
    "zyfer",
];

/// `b` が NTPsec `ntp.conf` 形式かどうか。
///
/// `refclock`/`nts`/`leapsmearinterval`/`mspps` 等の拡張ディレクティブが
/// 1 件以上あり、かつ行構造が ntp.conf 風であること。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.nts + c.refclocks >= 1)
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ディレクティブ行総数。
    pub entries: usize,
    /// `server`/`pool`/`peer`/`broadcast`/`unpeer` ソース数。
    pub sources: usize,
    /// `restrict` 行数。
    pub restrict: usize,
    /// `nts`/`ntsconfig` 行数。
    pub nts: usize,
    /// `refclock` 行数。
    pub refclocks: usize,
    /// `leapsmearinterval`/`mspps` 等 NTPsec その他数。
    pub ext: usize,
    /// `driftfile`/`includefile`/`keys`/`logconfig`/`logfile`/`statistics`/`filegen` 等汎用 ntpd 系数。
    pub classic: usize,
    /// `refclock` 行内ドライバ+オプション語数。
    pub refclock_opts: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` を NTPsec `ntp.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        sources: 0,
        restrict: 0,
        nts: 0,
        refclocks: 0,
        ext: 0,
        classic: 0,
        refclock_opts: 0,
        misc: 0,
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        let mut words = line.split_whitespace();
        let head = words.next().unwrap_or("");
        match head {
            "server" | "pool" | "peer" | "broadcast" | "multicastclient" | "manycastclient"
            | "unpeer" | "unconfig" | "broadcastclient" | "manycastserver" => {
                c.sources += 1;
            }
            "restrict" | "interface" | "discard" | "mru" => {
                c.restrict += 1;
            }
            "nts" | "ntsconfig" => {
                c.nts += 1;
                if head == "nts" {
                    let _ = words.next();
                }
            }
            "refclock" => {
                c.refclocks += 1;
                for opt in line.split_whitespace().skip(1) {
                    if REFCLOCK_DRIVERS.contains(&opt)
                        || matches!(
                            opt,
                            "unit"
                                | "refid"
                                | "time1"
                                | "time2"
                                | "stratum"
                                | "flag1"
                                | "flag2"
                                | "flag3"
                                | "flag4"
                                | "mode"
                                | "minpoll"
                                | "maxpoll"
                                | "path"
                                | "ppspath"
                                | "baud"
                                | "mintc"
                        )
                        || opt.starts_with("flag")
                    {
                        c.refclock_opts += 1;
                    }
                }
            }
            "leapsmearinterval" | "leapsmear" | "mspps" | "mintc" | "maxclock" => {
                c.ext += 1;
            }
            "driftfile" | "includefile" | "keys" | "trustedkey" | "logconfig" | "logfile"
            | "statistics" | "filegen" | "statsdir" | "tos" | "tinker" | "enable" | "disable"
            | "rlimit" | "crypto" | "revoke" | "leapfile" | "pidfile" | "saveconfigdir"
            | "automax" | "tosm" | "peerstats" => {
                c.classic += 1;
            }
            _ => {
                c.misc += 1;
            }
        }
    }
    if c.entries >= 1 && (c.nts + c.refclocks + c.ext >= 1 || c.entries - c.misc >= 2) {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# ntpsec ntp.conf\n\
        driftfile /var/lib/ntp/ntp.drift\n\
        statsdir /var/log/ntpsec/\n\
        statistics loopstats peerstats clockstats\n\
        filegen loopstats file loopstats type day enable\n\
        \n\
        refclock shm unit 0 refid GPS\n\
        refclock shm unit 1 refid PPS flag1 1\n\
        \n\
        server ntp1.example.com iburst\n\
        server ntp2.example.com iburst minpoll 6\n\
        pool ntpsec.pool.ntp.org iburst\n\
        \n\
        restrict default nomodify nopeer noquery limited\n\
        restrict 127.0.0.1\n\
        restrict ::1\n\
        \n\
        nts enable\n\
        nts mintls 1.2\n\
        nts maxtls 1.3\n\
        nts ca /etc/ssl/certs/ca-certificates.crt\n\
        nts cert /etc/ntpsec/cert.pem\n\
        nts key /etc/ntpsec/key.pem\n\
        nts cookie /var/lib/ntpsec/cookies\n\
        leapsmearinterval 3600\n";

    #[test]
    fn detects_ntpsec() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 20);
        assert_eq!(c.sources, 3);
        assert_eq!(c.restrict, 3);
        assert_eq!(c.refclocks, 2);
        assert_eq!(c.nts, 7);
        assert_eq!(c.ext, 1);
        assert_eq!(c.classic, 4);
        assert_eq!(c.refclock_opts, 7);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_plain_ntp() {
        assert!(!detect(b"server ntp.example.com\nrestrict default\n"));
        assert!(!detect(b"foo=bar\n"));
    }
}
