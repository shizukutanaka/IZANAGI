//! Moonraker API サーバ設定 (`moonraker.conf`) の検出・カウント。
//!
//! Klipper 同様 `key: value` だがセクションは `[server]`/`[authorization]`/
//! `[update_manager name]`/`[power device]` 等の Moonraker コンポーネント名。
//!
//! ```
//! let cfg = b"[server]\nhost: 0.0.0.0\nport: 7125\nklippy_uds_address: /tmp/klippy_uds\n\n\
//!             [authorization]\ntrusted_clients:\n  10.0.0.0/8\ncors_domains:\n  *.lan\n\n\
//!             [update_manager]\nrefresh_interval: 168\n";
//! assert!(izanagi_kit::moonrakerconf::detect(cfg));
//! let c = izanagi_kit::moonrakerconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.entries, 6);
//! assert_eq!(c.update_manager_sections, 1);
//! ```

/// セクション先頭語として知られる Moonraker コンポーネント名。
const KNOWN_SECTIONS: &[&str] = &[
    "server",
    "authorization",
    "octoprint_compat",
    "history",
    "update_manager",
    "announcements",
    "machine",
    "data_sync",
    "file_manager",
    "database",
    "job_queue",
    "spoolman",
    "power",
    "wled",
    "timelapse",
    "simplyprint",
    "mqtt",
    "notifier",
    "secrets",
    "remote",
    "shell_command",
    "sensor",
    "hue",
    "tasmota",
    "tplink",
    "tplink_smartplug",
    "homeassistant",
    "loxonev1",
    "rf",
    "zeroconf",
    "http",
    "webrtc",
    "agent",
    "moonraker",
    "extension",
];

/// デバイス系セクション (第2語がデバイス名)。
fn is_device_section(head: &str) -> bool {
    matches!(
        head,
        "power"
            | "wled"
            | "notifier"
            | "neopixel"
            | "dotstar"
            | "led"
            | "hue"
            | "tasmota"
            | "tplink"
            | "rf"
            | "sensor"
            | "shell_command"
    )
}

fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find(':')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
    {
        return None;
    }
    Some((k, s[i + 1..].trim()))
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Moonraker コンポーネント型を持つセクション数。
    pub sections: usize,
    /// セクション内の `key: value` 行数。
    pub entries: usize,
    /// `[update_manager …]` 系セクション数。
    pub update_manager_sections: usize,
    /// `[power …]`/`[wled …]`/`[notifier …]` 等のデバイス系セクション数。
    pub device_sections: usize,
    /// 真偽値 (`True`/`False`/`true`/`false`) の行数。
    pub bool_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

/// `b` が Moonraker `moonraker.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.sections >= 2 && c.entries >= 2
}

/// `b` を Moonraker `moonraker.conf` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        update_manager_sections: 0,
        device_sections: 0,
        bool_entries: 0,
        comments: 0,
    };
    let mut in_known = false;
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') {
            let inner = s
                .strip_prefix('[')
                .unwrap_or(s)
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            let head = inner.split(' ').next().unwrap_or("");
            in_known = KNOWN_SECTIONS.contains(&head);
            if in_known {
                c.sections += 1;
                if head == "update_manager" {
                    c.update_manager_sections += 1;
                }
                if is_device_section(head) {
                    c.device_sections += 1;
                }
            }
            continue;
        }
        if !in_known {
            continue;
        }
        if let Some((_k, v)) = key_of(s) {
            c.entries += 1;
            if matches!(v, "True" | "False" | "true" | "false") {
                c.bool_entries += 1;
            }
        }
    }
    if c.sections == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_moonraker_conf() {
        let cfg = b"[server]\nhost: 0.0.0.0\nport: 7125\n\n\
                    [authorization]\nforce_logins: True\ntrusted_clients:\n  192.168.1.0/24\n\n\
                    [update_manager fluidd]\ntype: web\nrepo: fluidd-core/fluidd\n\n\
                    [power printer]\ntype: tplink_smartplug\naddress: 192.168.1.10\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.update_manager_sections, 1);
        assert_eq!(c.device_sections, 1);
        assert_eq!(c.bool_entries, 1);
    }

    #[test]
    fn rejects_klipper() {
        // Klipper 型セクションは Moonraker コンポーネントではない
        assert!(!detect(
            b"[stepper x]\nstep_pin: PA0\ndir_pin: PC1\n[extruder]\nmax_temp: 270\n"
        ));
    }
}
