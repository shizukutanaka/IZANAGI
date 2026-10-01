//! Remmina `remmina.pref`/`*.remmina` プロファイル パーサ。
//!
//! `[remmina]`/`[remmina_pref]` セクションと `name`/`protocol`/`server`/`username`/
//! `domain`/`resolution`/`colourdepth`/`keymap`/`viewmode`/`screenshot_path` 等
//! 既知キーを計数する。
//!
//! ```
//! use izanagi_kit::remminaconf;
//! let conf = b"[remmina]\nname=work\nprotocol=RDP\nserver=192.168.1.10\nresolution=1920x1080\n";
//! assert!(remminaconf::detect(conf));
//! let c = remminaconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 1);
//! assert_eq!(c.known_keys, 4);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// `[remmina]`/`[remmina_pref]`/`[remmina_about]`/`[remmina_exec]` 数。
    pub known_sections: usize,
    /// `key=value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "remmina",
    "remmina_pref",
    "remmina_about",
    "remmina_exec",
    "remmina_stats",
    "remmina_news",
    "remmina_tip",
];

const KNOWN_KEYS: &[&str] = &[
    "name",
    "group",
    "protocol",
    "server",
    "port",
    "username",
    "password",
    "domain",
    "colordepth",
    "colourdepth",
    "resolution",
    "keymap",
    "exec",
    "execarg",
    "precommand",
    "postcommand",
    "sound",
    "sharefolder",
    "shareprinter",
    "sharesmartcard",
    "disablepasswordstoring",
    "disableautoreconnect",
    "console",
    "disableserverinput",
    "clientname",
    "loadbalanceinfo",
    "security",
    "gateway_server",
    "gateway_domain",
    "gateway_username",
    "gateway_password",
    "gateway_usage",
    "usereq",
    "ignore-tls-errors",
    "quality",
    "disableencryption",
    "showcursor",
    "viewmode",
    "scaler",
    "scale",
    "window_maximize",
    "window_height",
    "window_width",
    "toolbar_placement",
    "view_file",
    "preferipv6",
    "ssh_auth",
    "ssh_enabled",
    "ssh_server",
    "ssh_username",
    "ssh_password",
    "ssh_privatekey",
    "ssh_passphrase",
    "ssh_agent",
    "ssh_kex_algorithms",
    "ssh_ciphers",
    "ssh_hostkeytypes",
    "ssh_proxycommand",
    "ssh_stricthostkeycheck",
    "ssh_compression",
    "ssh_reconnect_tries",
    "ssh_tunnel_port",
    "cert_priority",
    "cert_password",
    "notes_text",
    "vc",
    "tls-sec-level",
    "websockets",
    "audio-input",
    "multimedia",
    "redirectcam",
    "redirectaudio",
    "restricted_admin",
    "rdp2tcp",
    "rdp_reconnect_attempts",
    "freerdp_log_level",
    "glyph-cache",
    "codec",
    "rfx",
    "nsc",
    "gfx",
    "gfx-h264",
    "network",
    "monitorids",
    "span_monitors",
    "multimon",
    "microphone",
    "drive",
    "sshfs",
    "detached_toolbar",
    "floating_toolbar_placement",
    "screenshot_path",
    "screenshot_name",
    "save_view_mode",
    "rdp_mouse_jitter",
    "usb",
    "websockify",
    "ssh_tunnel_enabled",
    "ssh_tunnel_server",
    "resolutions",
    "keystrokes",
    "keyboard_grab",
    "vcredirection",
];

/// `remmina.pref`/`*.remmina` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_sections >= 1 && c.known_keys >= 2) || c.known_keys >= 3,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            let name = t[1..t.len() - 1].trim().to_ascii_lowercase();
            if KNOWN_SECTIONS.contains(&name.as_str()) || name.starts_with("remmina") {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.sections > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[remmina]\nname=work-vpn\nprotocol=RDP\nserver=192.168.1.10:3389\nusername=admin\ncolourdepth=32\nresolution=1920x1080\nssh_enabled=1\nssh_username=tunnel\nwindow_maximize=1\n";

    #[test]
    fn detects_remmina() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.known_sections, 1);
        assert_eq!(c.entries, 9);
        assert_eq!(c.known_keys, 9);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[a]\nx=1\ny=2\n";
        assert!(!detect(ini));
    }
}
