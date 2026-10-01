//! chrony `chrony.conf` の検出・カウント。
//!
//! 空白区切りディレクティブ: `server`/`pool`/`peer`/`refclock` ソース、
//! `allow`/`deny`/`local`/`cmdallow`/`cmddeny` アクセス、
//! `makestep`/`rtcsync`/`leapsectz`/`driftfile` 時刻系、
//! `log`/`logdir`/`logchange`/`noclientlog` ログ系、
//! `keyfile`/`ntsdumpdir`/`nts*` NTS 系、`bindaddress`/`bindcmdaddress`/`port`/`cmdport` バインド系を分類。
//!
//! ```
//! let cfg = b"server ntp.example.com iburst\n\
//!             pool pool.ntp.org iburst\n\
//!             driftfile /var/lib/chrony/drift\n\
//!             makestep 1.0 3\n\
//!             rtcsync\n\
//!             logdir /var/log/chrony\n\
//!             allow 192.168.0.0/16\n";
//! assert!(izanagi_kit::chronyconf::detect(cfg));
//! let c = izanagi_kit::chronyconf::parse(cfg).unwrap();
//! assert_eq!(c.sources, 2);
//! ```

/// ソース系ディレクティブ。
const SOURCE_KEYS: &[&str] = &["server", "pool", "peer", "refclock", "broadcast"];

/// アクセス制御系ディレクティブ。
const ACCESS_KEYS: &[&str] = &[
    "allow",
    "deny",
    "local",
    "cmdallow",
    "cmddeny",
    "cmdport",
    "bindcmdaddress",
    "bindcmddevice",
    "manual",
    "initstepslew",
    "initstepslewfile",
    "initstepslewthreshold",
];

/// 時刻・ドリフト系ディレクティブ。
const TIME_KEYS: &[&str] = &[
    "driftfile",
    "makestep",
    "makestethreshold",
    "rtcsync",
    "rtcfile",
    "rtconutc",
    "leapsectz",
    "leapsecmode",
    "leapsecmode2",
    "maxdistance",
    "maxjitter",
    "minsources",
    "reselectdist",
    "stratumweight",
    "corrtimeratio",
    "maxclockerror",
    "maxdrift",
    "maxslewrate",
    "maxupdateskew",
    "smoothtime",
    "tempcomp",
    "freq",
    "offset",
    "refresh",
    "combine",
    "combining",
    "dscard",
    "ntsdrift",
    "hwtimestamp",
    "hwdebjitter",
    "precision",
    "sourcename",
    "sourcestats",
    "minsamples",
    "maxsamples",
];

/// ログ・デバッグ系ディレクティブ。
const LOG_KEYS: &[&str] = &[
    "log",
    "logdir",
    "logchange",
    "mailonchange",
    "noclientlog",
    "clientloglimit",
    "logbanner",
    "dumptable",
    "dumpdir",
    "pidfile",
    "user",
    "sched_priority",
    "lock_all",
    "writesubsys",
];

/// NTS/認証系ディレクティブ。
const AUTH_KEYS: &[&str] = &[
    "keyfile",
    "authselectmode",
    "ntsdumpdir",
    "ntsserverkey",
    "ntsservercert",
    "ntsserverchain",
    "ntsntpserver",
    "ntsprocesses",
    "ntsratelimit",
    "maxntsconns",
    "ntsport",
    "certtimethreshold",
    "nocerttimecheck",
    "ntsdumpdir2",
    "ntsrefresh",
    "ntstrustedcerts",
    "ntsdh",
    "ntsrotate",
];

/// ネットワーク・バインド系ディレクティブ。
const NET_KEYS: &[&str] = &[
    "port",
    "bindaddress",
    "bindacqaddress",
    "acquisitionport",
    "noclientlog2",
    "includefile",
    "include",
    "confdir",
    "sourcedir",
    "ntsdumpdir3",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ディレクティブ行総数。
    pub entries: usize,
    /// `server`/`pool`/`peer`/`refclock`/`broadcast` 数。
    pub sources: usize,
    /// `allow`/`deny`/`local`/`cmdallow`/`bindcmd*` 等アクセス数。
    pub access: usize,
    /// `driftfile`/`makestep`/`rtcsync`/`leapsectz` 等時刻系数。
    pub time: usize,
    /// `log`/`logdir`/`logchange`/`noclientlog`/`pidfile`/`user` 等ログ・制御数。
    pub log: usize,
    /// `keyfile`/`nts*`/`authselectmode` NTS/認証数。
    pub auth: usize,
    /// `port`/`bind*`/`include`/`confdir`/`sourcedir` ネット/パス数。
    pub net: usize,
    /// `iburst`/`prefer`/`trust`/`minpoll`/`maxpoll`/`nts`/`xleave` 等ソース内オプション数。
    pub source_opts: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が `chrony.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を `chrony.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        sources: 0,
        access: 0,
        time: 0,
        log: 0,
        auth: 0,
        net: 0,
        source_opts: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
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
                        | "trust"
                        | "require"
                        | "xleave"
                        | "nts"
                        | "offline"
                        | "auto_offline"
                        | "presend"
                        | "minpoll"
                        | "maxpoll"
                        | "polltarget"
                        | "maxdelay"
                        | "maxdelayratio"
                        | "maxdelaydevratio"
                        | "mindelay"
                        | "asymmetry"
                        | "offset"
                        | "minsamples"
                        | "maxsamples"
                        | "filter"
                        | "port"
                        | "copy"
                        | "extfield"
                        | "ipv4"
                        | "ipv6"
                        | "certset"
                        | "maxdelayquant"
                        | "version"
                ) || opt.starts_with("minpoll")
                    || opt.starts_with("maxpoll")
                    || opt.starts_with("polltarget")
                {
                    c.source_opts += 1;
                }
            }
        } else if ACCESS_KEYS.contains(&head) {
            c.access += 1;
            known += 1;
        } else if TIME_KEYS.contains(&head) {
            c.time += 1;
            known += 1;
        } else if LOG_KEYS.contains(&head) {
            c.log += 1;
            known += 1;
        } else if AUTH_KEYS.contains(&head) || head.starts_with("nts") {
            c.auth += 1;
            known += 1;
        } else if NET_KEYS.contains(&head) {
            c.net += 1;
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

    const SAMPLE: &[u8] = b"# chrony.conf\n\
        server ntp.example.com iburst\n\
        pool pool.ntp.org iburst maxsources 4\n\
        peer peer.example.net minpoll 6\n\
        refclock SHM 0 poll 3 refid GPS precision 1e-1\n\
        driftfile /var/lib/chrony/drift\n\
        makestep 1.0 3\n\
        rtcsync\n\
        allow 192.168.0.0/16\n\
        deny all\n\
        local stratum 10\n\
        logdir /var/log/chrony\n\
        log measurements statistics tracking\n\
        keyfile /etc/chrony.keys\n\
        ntsdumpdir /var/lib/chrony\n\
        ntsservercert /etc/pki/tls/certs/server.crt\n\
        ntsserverkey /etc/pki/tls/private/server.key\n\
        bindaddress 0.0.0.0\n\
        port 123\n";

    #[test]
    fn detects_chrony() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 18);
        assert_eq!(c.sources, 4);
        assert_eq!(c.access, 3);
        assert_eq!(c.time, 3);
        assert_eq!(c.log, 2);
        assert_eq!(c.auth, 4);
        assert_eq!(c.net, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"server ntp.example.com\n"));
    }
}
