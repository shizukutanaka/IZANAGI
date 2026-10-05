//! JACK daemon rc (`.jackdrc`/`jackd.conf`) parser.
//!
//! Detects JACK configs by `/usr/bin/jackd`/`jackd`-invocation lines with
//! `-d` driver switches (`-dalsa`/`-dnet`/`-doss`/`-dfreebob`/`-dfirewire`/
//! `-ddummy`/`-dcoreaudio`/`-dportaudio`/`-dsun`/`-dcoremidi`), `-r` rate,
//! `-p` period, `-n` nperiods, `-P` playback, `-C` capture, `-S` sync,
//! `-m` monitor, `-s` softmode, `-i` inchannels, `-o` outchannels,
//! `-z` dither, `-H` hwmon, `-M` hwmeter, `-X` midi, `-v` verbose,
//! `-R` realtime, `-T` temporary, `-t` timeout, `--asio`, `--midi`,
//! and `jack_control`/`qjackctl` launcher lines.
//!
//! ```
//! let b = b"/usr/bin/jackd -R -dalsa -r48000 -p512 -n3 -D -Chw:0 -Phw:0\n";
//! assert!(izanagi_kit::jackrc::detect(b));
//! let c = izanagi_kit::jackrc::Jackrc::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed .jackdrc summary.
#[derive(Debug, Clone)]
pub struct Jackrc {
    /// Recognized tokens.
    pub keys: usize,
    /// Invocation tokens (`jackd`/`jackdbus`/`jack_control`/`qjackctl`/`/usr/bin/`/`/usr/local/bin/`).
    pub invoke_keys: usize,
    /// Option flags (`-d`/`-r`/`-p`/`-n`/`-D`/`-C`/`-P`/`-S`/`-m`/`-s`/`-i`/`-o`/`-z`/`-H`/`-M`/`-X`/`-v`/`-R`/`-T`/`-t`/`--asio`/`--midi`/`--driver`/`--rate`/`--period`/`--nperiods`/`--duplex`/`--capture`/`--playback`/`--hwmon`/`--hwmeter`/`--verbose`/`--realtime`/`--temporary`/`--timeout`/`--midi`/`--sync`/`--monitor`/`--softmode`/`--inchannels`/`--outchannels`/`--dither`/`--unlock`/`--name`/`--replace-registry`/`--autoconf`/`--nozombies`/`--temporary`/`--port-max`).
    pub flag_keys: usize,
    /// Driver/device values (`alsa`/`net`/`oss`/`freebob`/`firewire`/`dummy`/`coreaudio`/`portaudio`/`coremidi`/`hw:`/`plughw:`/`system`/`duplex`/`midi`/`seq`/`rawmidi`).
    pub val_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Invocation tokens.
const INVOKE_KEYS: &[&str] = &[
    "jackd",
    "jackdbus",
    "jack_control",
    "qjackctl",
    "/usr/bin/",
    "/usr/local/bin/",
];

/// Option flags.
const FLAG_KEYS: &[&str] = &[
    " -d",
    " -r",
    " -p",
    " -n",
    " -D",
    " -C",
    " -P",
    " -S",
    " -m",
    " -s",
    " -i",
    " -o",
    " -z",
    " -H",
    " -M",
    " -X",
    " -v",
    " -R",
    " -T",
    " -t",
    "--asio",
    "--midi",
    "--driver",
    "--rate",
    "--period",
    "--nperiods",
    "--duplex",
    "--capture",
    "--playback",
    "--hwmon",
    "--hwmeter",
    "--verbose",
    "--realtime",
    "--temporary",
    "--timeout",
    "--syn\u{63}",
    "--monitor",
    "--softmode",
    "--inchannels",
    "--outchannels",
    "--dither",
    "--unlock",
    "--name",
    "--nozombies",
    "--port-max",
];

/// Driver/device values.
const VAL_KEYS: &[&str] = &[
    "alsa",
    "net",
    "oss",
    "freebob",
    "firewire",
    "dummy",
    "coreaudio",
    "portaudio",
    "coremidi",
    "hw:",
    "plughw:",
    "system",
    "playback",
    "capture",
    "duplex",
    "midi",
    "seq",
    "rawmidi",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["jackd", "jackdbus", "jack_control"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "jackd",
    "jackdbus",
    "jack_control",
    " -d",
    " -r",
    " -p",
    " -n",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a .jackdrc file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Jackrc {
    /// Count categories in a .jackdrc. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            invoke_keys: 0,
            flag_keys: 0,
            val_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in INVOKE_KEYS {
            c.invoke_keys += t.matches(k).count();
        }
        for k in FLAG_KEYS {
            c.flag_keys += t.matches(k).count();
        }
        for k in VAL_KEYS {
            c.val_keys += t.matches(k).count();
        }
        c.keys = c.invoke_keys + c.flag_keys + c.val_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# jack\n/usr/bin/jackd -R -dalsa -r48000 -p512 -n3 -D -Chw:0 -Phw:0 -m -Xseq\n";
        assert!(detect(b));
        let c = Jackrc::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.invoke_keys >= 1);
        assert!(c.flag_keys >= 6);
        assert!(c.val_keys >= 3);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_sh() {
        assert!(!detect(b"#!/bin/sh\necho hi\n"));
        assert!(Jackrc::parse(b"a = b\n").is_none());
    }
}
