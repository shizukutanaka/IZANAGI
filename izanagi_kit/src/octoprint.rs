//! OctoPrint `config.yaml` の検出・カウント。
//!
//! YAML のトップレベルに `server`/`webcam`/`appearance`/`accessControl`/`serial`
//! `plugins` 等の既知キーが並び、`plugins:` 配下にプラグイン ID がネストする。
//!
//! ```
//! let cfg = b"server:\n  host: 0.0.0.0\n  port: 5000\nwebcam:\n  stream: /webcam/?action=stream\n\
//!             plugins:\n  tracking:\n    enabled: true\n  discovery:\n    enabled: false\n";
//! assert!(izanagi_kit::octoprint::detect(cfg));
//! let c = izanagi_kit::octoprint::parse(cfg).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.plugin_ids, 2);
//! assert_eq!(c.bool_entries, 2);
//! ```

/// 既知のトップレベルキー。
const KNOWN_TOP: &[&str] = &[
    "server",
    "webcam",
    "appearance",
    "accessControl",
    "devel",
    "plugins",
    "serial",
    "temperature",
    "feature",
    "folder",
    "logging",
    "pluginmanager",
    "printer",
    "system",
    "api",
    "onlineCheck",
    "pluginBlacklist",
    "slicing",
    "announcements",
    "backup",
    "events",
    "gcodeViewer",
    "estimation",
    "files",
    "printerProfiles",
    "scripts",
    "softwareupdate",
    "tracking",
    "virtual_printer",
    "watchdog",
    "coreWizard",
    "discovery",
    "errortracking",
    "loginui",
    "settings",
    "terminalfilters",
];

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find(':')?;
    let k = s[..i].trim();
    if k.is_empty()
        || k.contains(' ')
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
    /// トップレベルキー数。
    pub sections: usize,
    /// トップレベルのうち既知キー数。
    pub known_sections: usize,
    /// `key:` 行の総数 (全階層)。
    pub keys: usize,
    /// `plugins:` 直下のプラグイン ID 数。
    pub plugin_ids: usize,
    /// `enabled:`/`true`/`false` 等の真偽値行数。
    pub bool_entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// `b` が OctoPrint `config.yaml` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 2 || (c.plugin_ids >= 1 && c.known_sections >= 1)
}

/// `b` を OctoPrint `config.yaml` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        keys: 0,
        plugin_ids: 0,
        bool_entries: 0,
        comments: 0,
    };
    // plugins ブロック追跡: (plugins 行の indent, プラグイン ID の indent)
    let mut plugins_ctx: Option<(usize, Option<usize>)> = None;
    for line in text.lines() {
        let s = line.trim_end();
        if s.trim().is_empty() {
            continue;
        }
        if s.trim_start().starts_with('#') {
            c.comments += 1;
            continue;
        }
        let ind = indent_of(s);
        let Some((k, v)) = key_of(s.trim_start()) else {
            continue;
        };
        c.keys += 1;
        if ind == 0 {
            c.sections += 1;
            if KNOWN_TOP.contains(&k) {
                c.known_sections += 1;
            }
            if k == "plugins" {
                plugins_ctx = Some((ind, None));
            } else {
                plugins_ctx = None;
            }
        } else if let Some((pind, cid)) = plugins_ctx {
            if ind <= pind {
                plugins_ctx = None;
            } else {
                match cid {
                    None => {
                        // plugins 直下最初の子: プラグイン ID の階層を記録
                        plugins_ctx = Some((pind, Some(ind)));
                        c.plugin_ids += 1;
                    }
                    Some(cid) if ind == cid => c.plugin_ids += 1,
                    _ => {}
                }
            }
        }
        if matches!(v, "true" | "false" | "True" | "False") {
            c.bool_entries += 1;
        }
    }
    if c.keys == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_config() {
        let cfg = b"# OctoPrint config\nserver:\n  host: 0.0.0.0\n  port: 5000\n\
                    serial:\n  port: /dev/ttyACM0\n  baudrate: 250000\n\
                    plugins:\n  discovery:\n    enabled: true\n  tracking:\n    enabled: false\n  403stop:\n    enabled: true\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.plugin_ids, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"name: app\nversion: 1\nitems:\n  - a\n  - b\n"));
    }
}
