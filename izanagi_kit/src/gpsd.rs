//! gpsd `/etc/default/gpsd` / `gpsd.conf` / systemd unit 上書きの検出・カウント。
//!
//! `KEY="value"` / `KEY=value` 形式。`DEVICES`/`GPSD_OPTIONS`/`USBAUTO`/
//! `START_DAEMON`/`GPSD_SOCKET`/`GPSD_GROUP`/`GPSD_USER`/`GPSD_HOME` 等。
//!
//! ```
//! let cfg = b"# gpsd defaults\n\
//!             START_DAEMON=\"true\"\n\
//!             USBAUTO=\"true\"\n\
//!             DEVICES=\"/dev/ttyUSB0\"\n\
//!             GPSD_OPTIONS=\"-n\"\n\
//!             GPSD_SOCKET=\"/var/run/gpsd.sock\"\n";
//! assert!(izanagi_kit::gpsd::detect(cfg));
//! let c = izanagi_kit::gpsd::parse(cfg).unwrap();
//! assert_eq!(c.devices, 1);
//! ```

/// gpsd defaults 既知キー。
const KNOWN_KEYS: &[&str] = &[
    "DEVICES",
    "GPSD_DEVICES",
    "GPSD_OPTIONS",
    "GPSD_SOCKET",
    "USBAUTO",
    "START_DAEMON",
    "GPSD_GROUP",
    "GPSD_USER",
    "GPSD_HOME",
    "GPSD_ARGS",
    "GPSD_OPTS",
    "OTHER_GPSD_OPTIONS",
    "GPSD_PORT",
    "BAUDRATE",
    "SERIALDEVICES",
    "SERIAL_DEVICES",
    "SPEED",
    "GPSDCONTROL",
    "GPSD_CONTROL",
    "GPSD_DEBUG",
    "GPSD_PIPE",
    "GPSD_SHARE",
    "GPSD_LISTEN",
    "GPSD_EXPORT",
    "GPSD_BADTIME",
    "GPSD_DEVICE",
    "NTPD_SHM",
    "NTPDOPTS",
    "GPS_OPTIONS",
    "CONTROL_SOCKET",
    "CONTROL_SOCKET_PATH",
    "OPTIONS",
    "DAEMON_OPTS",
    "GPSD_ENABLE",
    "GPSD_ENABLED",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `KEY=value` エントリ総数。
    pub entries: usize,
    /// `DEVICES`/`GPSD_DEVICES`/`SERIAL_DEVICES`/`GPSD_DEVICE`/`BAUDRATE` デバイス数。
    pub devices: usize,
    /// `GPSD_OPTIONS`/`GPSD_ARGS`/`GPSD_OPTS`/`OPTIONS`/`DAEMON_OPTS`/`GPS_OPTIONS`/`GPSD_DEBUG` オプション数。
    pub options: usize,
    /// `USBAUTO`/`START_DAEMON`/`GPSD_ENABLE`/`GPSD_ENABLED` 等 bool トグル数。
    pub toggles: usize,
    /// `GPSD_SOCKET`/`CONTROL_SOCKET*`/`GPSD_PIPE`/`GPSD_SHARE`/`GPSD_LISTEN`/`GPSD_EXPORT`/`GPSD_PORT`/`NTPD_*` ソケット/共有数。
    pub sockets: usize,
    /// `GPSD_GROUP`/`GPSD_USER`/`GPSD_HOME`/`GPSD_HOME` 等環境数。
    pub env: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が gpsd 設定形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を gpsd 設定ファイルとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        devices: 0,
        options: 0,
        toggles: 0,
        sockets: 0,
        env: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            known += 1;
        }
        if matches!(
            key,
            "DEVICES"
                | "GPSD_DEVICES"
                | "SERIAL_DEVICES"
                | "SERIALDEVICES"
                | "GPSD_DEVICE"
                | "BAUDRATE"
                | "SPEED"
        ) {
            c.devices += 1;
        } else if matches!(
            key,
            "GPSD_OPTIONS"
                | "GPSD_ARGS"
                | "GPSD_OPTS"
                | "OPTIONS"
                | "DAEMON_OPTS"
                | "GPS_OPTIONS"
                | "OTHER_GPSD_OPTIONS"
                | "GPSD_DEBUG"
                | "NTPDOPTS"
        ) {
            c.options += 1;
        } else if matches!(
            key,
            "USBAUTO" | "START_DAEMON" | "GPSD_ENABLE" | "GPSD_ENABLED" | "GPSD_BADTIME"
        ) {
            c.toggles += 1;
        } else if matches!(
            key,
            "GPSD_SOCKET"
                | "CONTROL_SOCKET"
                | "CONTROL_SOCKET_PATH"
                | "GPSD_PIPE"
                | "GPSD_SHARE"
                | "GPSD_LISTEN"
                | "GPSD_EXPORT"
                | "GPSD_PORT"
                | "NTPD_SHM"
                | "GPSDCONTROL"
                | "GPSD_CONTROL"
        ) {
            c.sockets += 1;
        } else if matches!(key, "GPSD_GROUP" | "GPSD_USER" | "GPSD_HOME") {
            c.env += 1;
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

    const SAMPLE: &[u8] = b"# /etc/default/gpsd\n\
        START_DAEMON=\"true\"\n\
        USBAUTO=\"true\"\n\
        DEVICES=\"/dev/ttyUSB0 /dev/ttyUSB1\"\n\
        GPSD_DEVICES=\"/dev/ttyS0\"\n\
        GPSD_OPTIONS=\"-n -N\"\n\
        GPSD_SOCKET=\"/var/run/gpsd.sock\"\n\
        GPSD_GROUP=\"dialout\"\n\
        GPSD_USER=\"gpsd\"\n";

    #[test]
    fn detects_gpsd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 8);
        assert_eq!(c.devices, 2);
        assert_eq!(c.options, 1);
        assert_eq!(c.toggles, 2);
        assert_eq!(c.sockets, 1);
        assert_eq!(c.env, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"FOO=1\nBAR=2\n"));
        assert!(!detect(b"DEVICES=\"/dev/ttyUSB0\"\n"));
    }
}
