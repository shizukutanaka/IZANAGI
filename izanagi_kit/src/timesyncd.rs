//! systemd-timesyncd `timesyncd.conf` の検出・カウント。
//!
//! INI: `[Time]` セクション + `NTP`/`FallbackNTP`/`RootDistanceMaxSec`/
//! `PollIntervalMinSec`/`PollIntervalMaxSec`/`ConnectionRetrySec`/`SaveIntervalSec`/
//! `SampleTimeoutSec`/`Boltar`/`Trusted` 等キー。
//!
//! ```
//! let cfg = b"[Time]\n\
//!             NTP=ntp.example.com\n\
//!             FallbackNTP=0.debian.pool.ntp.org 1.debian.pool.ntp.org\n\
//!             RootDistanceMaxSec=5\n\
//!             PollIntervalMinSec=32\n\
//!             PollIntervalMaxSec=2048\n";
//! assert!(izanagi_kit::timesyncd::detect(cfg));
//! let c = izanagi_kit::timesyncd::parse(cfg).unwrap();
//! assert_eq!(c.sections, 1);
//! ```

/// `[Time]` セクション既知キー。
const TIME_KEYS: &[&str] = &[
    "NTP",
    "FallbackNTP",
    "RootDistanceMaxSec",
    "RootDistanceMax",
    "PollIntervalMinSec",
    "PollIntervalMaxSec",
    "PollIntervalMin",
    "PollIntervalMax",
    "ConnectionRetrySec",
    "SaveIntervalSec",
    "SampleTimeoutSec",
    "Boltar",
    "BoltarTimeoutSec",
    "Trusted",
    "NTPTrusted",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション総数。
    pub sections: usize,
    /// `key=value` エントリ総数。
    pub entries: usize,
    /// `NTP`/`FallbackNTP`/`Trusted`/`NTPTrusted` サーバ指定数。
    pub servers: usize,
    /// `RootDistanceMaxSec`/`PollIntervalMinSec`/`PollIntervalMaxSec`/`ConnectionRetrySec`/`SaveIntervalSec`/`SampleTimeoutSec`/`BoltarTimeoutSec` 間隔・閾値数。
    pub intervals: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が `timesyncd.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 1 && (c.servers >= 1 || c.intervals >= 1))
}

/// `b` を `timesyncd.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        servers: 0,
        intervals: 0,
        misc: 0,
    };
    let mut in_time = false;
    let mut has_time = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                let name = &rest[..end];
                in_time = name == "Time";
                has_time |= in_time;
                c.sections += 1;
                continue;
            }
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        c.entries += 1;
        if in_time {
            if matches!(key, "NTP" | "FallbackNTP" | "Trusted" | "NTPTrusted") {
                c.servers += 1;
            } else if TIME_KEYS.contains(&key) {
                c.intervals += 1;
            } else {
                c.misc += 1;
            }
        } else {
            c.misc += 1;
        }
    }
    if has_time && c.entries >= 1 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# /etc/systemd/timesyncd.conf\n\
        [Time]\n\
        NTP=ntp1.example.com ntp2.example.com\n\
        FallbackNTP=0.debian.pool.ntp.org 1.debian.pool.ntp.org\n\
        RootDistanceMaxSec=5\n\
        PollIntervalMinSec=32\n\
        PollIntervalMaxSec=2048\n\
        ConnectionRetrySec=30\n\
        SaveIntervalSec=60\n";

    #[test]
    fn detects_timesyncd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.entries, 7);
        assert_eq!(c.servers, 2);
        assert_eq!(c.intervals, 5);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[General]\nx=1\n"));
        assert!(!detect(b"[Time]\n"));
    }
}
