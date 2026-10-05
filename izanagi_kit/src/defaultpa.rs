//! PulseAudio `default.pa` startup script parser.
//!
//! Detects default.pa files by their `load-module module-* args` commands
//! (`module-detect`/`module-alsa-*`/`module-jack-*`/`module-null-sink`/
//! `module-native-protocol-unix`/`module-default-device-restore`/
//! `module-rescue-streams`/`module-always-sink`/`module-suspend-on-idle`/
//! `module-console-kit`/`module-systemd-login`/`module-udev-detect`/
//! `module-bluetooth-*`/`module-x11-*`/`module-switch-on-*`/
//! `module-role-*`/`module-filter-*`/`module-stream-*`/`module-pipe-*`/
//! `module-rtp-*`/`module-echo-cancel`/`module-ladspa-sink`/
//! `module-virtual-*`/`module-combine*`/`module-remap-*`/
//! `module-match`/`module-cli`/`module-http-*`/`module-zeroconf-*`/
//! `module-raop-*`/`module-gsettings`/`module-dbus-protocol`/
//! `module-oss`/`module-solaris`/`module-augment-properties`/
//! `module-device-manager`/`module-intended-roles`/
//! `module-position-event-sounds`), `.ifexists`/`.endif` conditionals,
//! `.fail`/`.nofail`, `set-default-*`/`set-sink-*`/`set-source-*`/
//! `suspend`/`load-default-script-file`/`include` and `###` comments.
//!
//! ```
//! let b = b".fail\nload-module module-device-restore\nload-module module-stream-restore\nload-module module-alsa-sink device=hw:0\n.ifexists module-udev-detect.so\nload-module module-udev-detect\n.endif\nset-default-sink output\n";
//! assert!(izanagi_kit::defaultpa::detect(b));
//! let c = izanagi_kit::defaultpa::Defaultpa::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed default.pa summary.
#[derive(Debug, Clone)]
pub struct Defaultpa {
    /// Recognized command occurrences.
    pub keys: usize,
    /// `load-module`/`unload-module`/`load-default-script-file`/`include` lines.
    pub module_keys: usize,
    /// `module-*` module names.
    pub modname_keys: usize,
    /// Control/set commands (`.fail`/`.nofail`/`.ifexists`/`.endif`/`set-default-`/`set-sink-`/`set-source-`/`suspend`/`set-card-*`/`update-sink-proplist`/`update-source-proplist`/`exit`/`respawn`).
    pub ctrl_keys: usize,
    /// `###`/`#` comment lines.
    pub comments: usize,
}

/// Module commands.
const MODULE_KEYS: &[&str] = &[
    "load-module",
    "unload-module",
    "load-default-script-file",
    "include",
];

/// Module names.
const MODNAME_KEYS: &[&str] = &[
    "module-",
    "module-detect",
    "module-alsa-",
    "module-jack-",
    "module-null-",
    "module-native-protocol-",
    "module-default-device-restore",
    "module-rescue-streams",
    "module-always-sink",
    "module-suspend-on-idle",
    "module-console-kit",
    "module-systemd-login",
    "module-udev-detect",
    "module-bluetooth-",
    "module-x11-",
    "module-switch-on-",
    "module-role-",
    "module-filter-",
    "module-stream-",
    "module-pipe-",
    "module-rtp-",
    "module-echo-cancel",
    "module-ladspa-sink",
    "module-virtual-",
    "module-combine",
    "module-remap-",
    "module-match",
    "module-cli",
    "module-http-",
    "module-zeroconf-",
    "module-raop-",
    "module-gsettings",
    "module-dbus-protocol",
    "module-oss",
    "module-solaris",
    "module-augment-properties",
    "module-device-manager",
    "module-intended-roles",
    "module-position-event-sounds",
];

/// Control/set commands.
const CTRL_KEYS: &[&str] = &[
    ".fail",
    ".nofail",
    ".ifexists",
    ".endif",
    "set-default-",
    "set-sink-",
    "set-source-",
    "suspend",
    "set-card-",
    "update-sink-proplist",
    "update-source-proplist",
    "exit",
    "respawn",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["module-", "load-module", "set-default-"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "load-module",
    "module-",
    ".ifexists",
    ".endif",
    "set-default-",
    ".fail",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a default.pa file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Defaultpa {
    /// Count categories in a default.pa. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            module_keys: 0,
            modname_keys: 0,
            ctrl_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in MODULE_KEYS {
            c.module_keys += t.matches(k).count();
        }
        for k in MODNAME_KEYS {
            c.modname_keys += t.matches(k).count();
        }
        for k in CTRL_KEYS {
            c.ctrl_keys += t.matches(k).count();
        }
        c.keys = c.module_keys + c.modname_keys + c.ctrl_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"### pa\n.fail\nload-module module-device-restore\nload-module module-stream-restore\nload-module module-alsa-sink device=hw:0\nload-module module-alsa-source device=hw:0\n.ifexists module-udev-detect.so\nload-module module-udev-detect\n.else\nload-module module-detect\n.endif\nload-module module-native-protocol-unix\nset-default-sink output\nset-default-source input\n";
        assert!(detect(b));
        let c = Defaultpa::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.module_keys >= 6);
        assert!(c.modname_keys >= 7);
        assert!(c.ctrl_keys >= 4);
        assert!(c.keys >= 17);
    }

    #[test]
    fn rejects_sh() {
        assert!(!detect(b"#!/bin/sh\necho hi\n"));
        assert!(Defaultpa::parse(b"a = b\n").is_none());
    }
}
