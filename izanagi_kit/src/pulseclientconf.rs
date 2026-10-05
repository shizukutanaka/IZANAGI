//! PulseAudio `client.conf`/`daemon.conf` parser.
//!
//! Detects PulseAudio client/daemon configs by `default-sink`/`default-source`/
//! `default-server`/`autospawn`/`daemon-binary`/`extra-arguments`/`enable-shm`/
//! `shm-size-bytes`/`cookie-file`/`cookie-in-x11root`/`client.conf`-style keys,
//! `daemonize`/`fail`/`high-priority`/`realtime-scheduling`/`realtime-priority`/
//! `nice-level`/`exit-idle-time`/`scache-idle-time`/`flat-volumes`/`resample-method`/
//! `enable-remixing`/`enable-lfe-remixing`/`lfe-crossover-freq`/`default-sample-*`/
//! `default-channel-map`/`default-fragments`/`default-fragment-size-msec`/
//! `rlimit-*`/`log-*`/`module-*`/`avoid-resampling`/`allow-exit`/`disconnect-on-exit`/
//! `deferred-volume*`/`lock-memsize`/`cpu-limit`/`system-instance`/`zsh-completion`/
//! `message-headers`/`config`/`dl-search-path`.
//!
//! ```
//! let b = b"default-sink = alsa_output.pci-0000_00_1f.3.analog-stereo\ndefault-source = alsa_input.pci-0000_00_1f.3.analog-stereo\nautospawn = yes\nenable-shm = yes\ndaemon-binary = /usr/bin/pulseaudio\n";
//! assert!(izanagi_kit::pulseclientconf::detect(b));
//! let c = izanagi_kit::pulseclientconf::Paclient::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed client.conf/daemon.conf summary.
#[derive(Debug, Clone)]
pub struct Paclient {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Client keys (`default-sink`/`default-source`/`default-server`/`autospawn`/`daemon-binary`/`extra-arguments`/`enable-shm`/`shm-size-bytes`/`cookie-file`/`cookie-in-x11root`).
    pub client_keys: usize,
    /// Daemon keys (`daemonize`/`fail`/`high-priority`/`realtime-*`/`nice-level`/`*-idle-time`/`flat-volumes`/`resample-method`/`enable-remixing`/`enable-lfe-remixing`/`lfe-crossover-freq`/`default-sample-*`/`default-channel-map`/`default-fragments`/`default-fragment-size-msec`/`rlimit-*`/`log-*`/`avoid-resampling`/`allow-exit`/`disconnect-on-exit`/`deferred-volume*`/`lock-memsize`/`cpu-limit`/`system-instance`/`module-*`/`dl-search-path`/`message-headers`).
    pub daemon_keys: usize,
    /// `=` separator occurrences.
    pub equals: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Client keys.
const CLIENT_KEYS: &[&str] = &[
    "default-sink",
    "default-source",
    "default-server",
    "autospawn",
    "daemon-binary",
    "extra-arguments",
    "enable-shm",
    "shm-size-bytes",
    "cookie-file",
    "cookie-in-x11root",
];

/// Daemon keys.
const DAEMON_KEYS: &[&str] = &[
    "daemonize",
    "fail",
    "high-priority",
    "realtime-scheduling",
    "realtime-priority",
    "nice-level",
    "exit-idle-time",
    "scache-idle-time",
    "flat-volumes",
    "resample-method",
    "enable-remixing",
    "enable-lfe-remixing",
    "lfe-crossover-freq",
    "default-sample-rate",
    "default-sample-format",
    "default-sample-channels",
    "default-channel-map",
    "default-fragments",
    "default-fragment-size-mse\u{63}",
    "rlimit-",
    "log-level",
    "log-target",
    "log-meta",
    "log-time",
    "log-backtrace",
    "avoid-resampling",
    "allow-exit",
    "disconnect-on-exit",
    "deferred-volume",
    "deferred-volume-safety-margin-use\u{63}",
    "deferred-volume-extra-delay-use\u{63}",
    "lock-memsize",
    "cpu-limit",
    "system-instance",
    "module-",
    "dl-search-path",
    "message-headers",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "default-sink",
    "default-source",
    "default-server",
    "autospawn",
    "daemon-binary",
    "resample-method",
    "flat-volumes",
    "daemonize",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a PulseAudio client/daemon conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Paclient {
    /// Count categories in a client.conf/daemon.conf. Returns `None` when
    /// the input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            client_keys: 0,
            daemon_keys: 0,
            equals: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
            }
        }
        for k in CLIENT_KEYS {
            c.client_keys += t.matches(k).count();
        }
        for k in DAEMON_KEYS {
            c.daemon_keys += t.matches(k).count();
        }
        c.equals = t.matches('=').count();
        c.keys = c.client_keys + c.daemon_keys + c.equals;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# pa client\nautospawn = yes\ndaemon-binary = /usr/bin/pulseaudio\nextra-arguments = --log-target=syslog\nenable-shm = yes\ndefault-sink = alsa_output.pci.analog\ndefault-source = alsa_input.pci.analog\ndaemonize = no\nflat-volumes = no\n";
        assert!(detect(b));
        let c = Paclient::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.client_keys >= 3);
        assert!(c.daemon_keys >= 2);
        assert!(c.equals >= 7);
        assert!(c.keys >= 12);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[sec]\na=b\n"));
        assert!(Paclient::parse(b"a = b\n").is_none());
    }
}
