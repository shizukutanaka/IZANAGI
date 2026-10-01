//! ntpd `ntp.conf` (クラシック ntpd/NTPsec 共通ディレクティブ) の検出・カウント。
//!
//! `server`/`pool`/`peer`/`broadcast`/`multicastclient`/`manycastclient` ソース、
//! `restrict`/`fudge`/`interface`/`discard` 制御、`driftfile`/`includefile`/`keys`/
//! `trustedkey`/`crypto`/`leapfile`/`tos`/`tinker`/`enable`/`disable`/`statistics`/
//! `filegen`/`logconfig`/`logfile`/`rlimit`/`mru` 等を分類。
//!
//! ```
//! let cfg = b"server ntp.example.com iburst\n\
//!             restrict default nomodify nopeer\n\
//!             restrict -6 default nomodify\n\
//!             driftfile /var/lib/ntp/drift\n\
//!             logconfig =syncstatus\n";
//! assert!(izanagi_kit::ntpconf::detect(cfg));
//! let c = izanagi_kit::ntpconf::parse(cfg).unwrap();
//! assert_eq!(c.sources, 1);
//! ```

/// アソシエーション・ソース系ディレクティブ。
const SOURCE_KEYS: &[&str] = &[
    "server",
    "pool",
    "peer",
    "broadcast",
    "multicastclient",
    "manycastclient",
    "manycastserver",
    "broadcastclient",
    "broadcastdelay",
    "mdnsserver",
    "mdnsclient",
    "unpeer",
    "unconfig",
];

/// アクセス制御・制限系。
const RESTRICT_KEYS: &[&str] = &[
    "restrict",
    "interface",
    "discard",
    "mru",
    "monlist",
    "mon_age",
    "mru_initalloc",
    "mru_initdepth",
    "mru_incalloc",
    "mru_incdepth",
    "mru_maxalloc",
    "mru_maxdepth",
    "mru_maxmem",
    "mru_mindepth",
    "nonvolatile",
    "rw",
];

/// 認証・暗号系。
const CRYPTO_KEYS: &[&str] = &[
    "keys",
    "trustedkey",
    "requestkey",
    "controlkey",
    "crypto",
    "revoke",
    "cert",
    "leapfile",
    "ident",
    "pw",
    "randfile",
    "host",
    "gqpar",
    "iffpar",
    "mvpar",
    "sign",
    "ntpsigndsocket",
    "keysdir",
];

/// ファイル・ログ系。
const FILE_KEYS: &[&str] = &[
    "driftfile",
    "includefile",
    "statistics",
    "filegen",
    "logconfig",
    "logfile",
    "statsdir",
    "peerstats",
    "loopstats",
    "clockstats",
    "sysstats",
    "protostats",
    "rawstats",
    "saveconfigdir",
    "config",
    "dumpdir",
    "pidfile",
    "peerconfig",
];

/// 動作チューニング系。
const TUNE_KEYS: &[&str] = &[
    "tos",
    "tinker",
    "enable",
    "disable",
    "rlimit",
    "automax",
    "maxdist",
    "maxhop",
    "minclock",
    "minsane",
    "orphan",
    "orphanwait",
    "beacon",
    "ceiling",
    "cohort",
    "floor",
    "mindist",
    "monitor",
    "freq",
    "huffpuff",
    "panic",
    "step",
    "stepback",
    "stepfwd",
    "stepout",
    "calibrate",
    "kernel",
    "ntp",
    "peer",
    "pps",
    "stats",
    "setvar",
    "timer",
    "source",
    "peer_clear",
    "reset",
    "auth",
    "bclient",
    "calibration",
    "monitor2",
    "multicast",
    "unprivileged",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ディレクティブ行総数。
    pub entries: usize,
    /// `server`/`pool`/`peer`/`broadcast`/`multicastclient`/`manycast*` 等ソース数。
    pub sources: usize,
    /// `restrict`/`interface`/`discard`/`mru*` 制限数。
    pub restrict: usize,
    /// `keys`/`trustedkey`/`crypto`/`cert`/`leapfile` 認証数。
    pub crypto: usize,
    /// `driftfile`/`statistics`/`filegen`/`logconfig`/`logfile`/`includefile` ファイル・ログ数。
    pub files: usize,
    /// `tos`/`tinker`/`enable`/`disable`/`rlimit`/`automax`/`orphan` チューニング数。
    pub tune: usize,
    /// `iburst`/`burst`/`prefer`/`minpoll`/`maxpoll`/`key`/`mode`/`ttl`/`version`/`autokey`/`true`/`noselect` 等ソース内オプション数。
    pub source_opts: usize,
    /// `restrict` 行内のフラグ数 (`nomodify`/`nopeer`/`notrap`/`kod`/`limited` 等)。
    pub restrict_flags: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が `ntp.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を `ntp.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        sources: 0,
        restrict: 0,
        crypto: 0,
        files: 0,
        tune: 0,
        source_opts: 0,
        restrict_flags: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        let head = line.split([' ', '\t']).next().unwrap_or("");
        if SOURCE_KEYS.contains(&head) {
            c.sources += 1;
            known += 1;
            for opt in line.split_whitespace().skip(2) {
                if matches!(
                    opt,
                    "iburst"
                        | "burst"
                        | "prefer"
                        | "true"
                        | "noselect"
                        | "autokey"
                        | "key"
                        | "mode"
                        | "ttl"
                        | "version"
                        | "minpoll"
                        | "maxpoll"
                        | "maxrate"
                        | "minrate"
                        | "xmtnonce"
                        | "ipv4"
                        | "ipv6"
                        | "suspend"
                        | "poll"
                ) || opt.starts_with("minpoll")
                    || opt.starts_with("maxpoll")
                {
                    c.source_opts += 1;
                }
            }
        } else if RESTRICT_KEYS.contains(&head) {
            c.restrict += 1;
            known += 1;
            if head == "restrict" {
                for flag in line.split_whitespace().skip(2) {
                    if matches!(
                        flag,
                        "kod"
                            | "limited"
                            | "nomodify"
                            | "nopeer"
                            | "noquery"
                            | "noserve"
                            | "notrap"
                            | "notrust"
                            | "ntpport"
                            | "version"
                            | "ignore"
                            | "mssntp"
                            | "nomrulist"
                            | "non_query"
                            | "flake"
                            | "noselect"
                            | "peer"
                            | "default"
                    ) {
                        c.restrict_flags += 1;
                    }
                }
            }
        } else if CRYPTO_KEYS.contains(&head) {
            c.crypto += 1;
            known += 1;
        } else if FILE_KEYS.contains(&head) {
            c.files += 1;
            known += 1;
        } else if TUNE_KEYS.contains(&head) {
            c.tune += 1;
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

    const SAMPLE: &[u8] = b"# /etc/ntp.conf\n\
        driftfile /var/lib/ntp/drift\n\
        statistics loopstats peerstats clockstats\n\
        filegen loopstats file loopstats type day enable\n\
        \n\
        server 0.debian.pool.ntp.org iburst\n\
        server 1.debian.pool.ntp.org iburst\n\
        pool pool.ntp.org iburst\n\
        \n\
        restrict -4 default kod notrap nomodify nopeer noquery limited\n\
        restrict -6 default kod notrap nomodify nopeer noquery limited\n\
        restrict 127.0.0.1\n\
        restrict ::1\n\
        restrict source notrap nomodify noquery\n\
        \n\
        keys /etc/ntp/keys\n\
        trustedkey 1 2 3\n\
        logconfig =syncstatus\n\
        tos minclock 4 minsane 3\n\
        tinker panic 0\n";

    #[test]
    fn detects_ntpconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 16);
        assert_eq!(c.sources, 3);
        assert_eq!(c.restrict, 5);
        assert_eq!(c.crypto, 2);
        assert_eq!(c.files, 4);
        assert_eq!(c.tune, 2);
        assert_eq!(c.source_opts, 3);
        assert_eq!(c.restrict_flags, 17);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"server ntp.example.com\n"));
    }
}
