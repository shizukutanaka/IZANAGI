//! pjsua `pjsua.cfg` config census.
//!
//! Command-line-style config: one `--option`/`--option=value` flag per
//! line (`--local-port`, `--id`, `--registrar`, `--proxy`, `--username`,
//! `--null-audio`, `--use-srtp` …); `#` comments.
//!
//! ```rust
//! let p = b"--local-port=5060\n--id=sip:alice@example.com\n--registrar=sip:example.com\n--username=alice\n--password=secret\n--null-audio\n";
//! assert!(izanagi_kit::pjsua::detect(p));
//! let c = izanagi_kit::pjsua::Pjsua::parse(p).unwrap();
//! assert_eq!(c.flags, 6);
//! ```

/// pjsua.cfg census.
#[derive(Debug, Clone)]
pub struct Pjsua {
    /// `--flag`/`--key=value` lines matching a known pjsua option.
    pub flags: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// pjsua command-line options (with the `--` prefix).
const FLAGS: &[&str] = &[
    "--aac",
    "--account",
    "--add-buddy",
    "--add-codec",
    "--app-log-level",
    "--auto-answer",
    "--auto-callback",
    "--auto-conf",
    "--auto-hangup",
    "--auto-loop",
    "--auto-play",
    "--auto-update-nat",
    "--bound-addr",
    "--capture-dev",
    "--capture-lat",
    "--clock-rate",
    "--color",
    "--config-file",
    "--disable-ec",
    "--dis-codec",
    "--dns-server",
    "--duration",
    "--ec-opt",
    "--ec-tail",
    "--empty-audio",
    "--extra-audio",
    "--help",
    "--ice",
    "--id",
    "--ipv6",
    "--ip-addr",
    "--local-port",
    "--log-file",
    "--log-level",
    "--max-calls",
    "--next-url",
    "--no-colors",
    "--no-psrtt",
    "--no-tcp",
    "--no-tones",
    "--no-udp",
    "--no-vad",
    "--null-audio",
    "--outbound",
    "--password",
    "--playback-dev",
    "--playback-lat",
    "--play-file",
    "--play-tone",
    "--proxy",
    "--publish",
    "--quality",
    "--realm",
    "--rec-file",
    "--reg-timeout",
    "--registrar",
    "--rtp-port",
    "--set-media-addr",
    "--snd-auto-close",
    "--snd-clock-rate",
    "--srtp-secure",
    "--srtp-keying",
    "--stereo",
    "--stun-srv",
    "--time",
    "--transport-id",
    "--turn-conn-type",
    "--turn-password",
    "--turn-srv",
    "--turn-tcp",
    "--turn-user",
    "--use-compact-form",
    "--use-ice",
    "--use-ims",
    "--use-loopback",
    "--use-srtp",
    "--use-stun",
    "--use-stun2",
    "--use-timer",
    "--use-turn",
    "--username",
    "--version",
    "--video",
    "--vid-capture-dev",
    "--vid-in-autorotate",
    "--vid-playback-dev",
    "--vid-rc",
    "--vid-tx",
    "--wav-port",
    "--xml",
];

fn flag(t: &str) -> Option<&str> {
    if !t.starts_with("--") {
        return None;
    }
    let name = t.split(['=', ' ', '\t']).next().unwrap_or(t);
    Some(name)
}

/// Detect a `pjsua.cfg`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if let Some(f) = flag(tr) {
            if FLAGS.contains(&f) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Pjsua {
    /// Count flags. Returns `None` when the input does not look like a
    /// `pjsua.cfg`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            flags: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(f) = flag(tr) {
                if FLAGS.contains(&f) {
                    c.flags += 1;
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
    fn detects() {
        let b = b"# pjsua\n--local-port=5060\n--log-level=5\n--app-log-level=4\n--null-audio\n--id=sip:alice@example.com\n--registrar=sip:example.com\n--realm=*\n--username=alice\n--password=secret\n--use-ims\n--max-calls=4\n--use-srtp=2\n--add-codec=opus\n--stereo\n";
        assert!(detect(b));
        let c = Pjsua::parse(b).unwrap();
        assert_eq!(c.flags, 14);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"key=value\n"));
        assert!(!detect(b"# --foo\n--bar\n"));
        assert!(Pjsua::parse(b"").is_none());
    }
}
