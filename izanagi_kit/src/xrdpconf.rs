//! xrdp `xrdp.ini`/`sesman.ini` パーサ。
//!
//! `[Globals]`/`[Logging]`/`[LoggingPerLogger]`/`[Channels]`/`[SessionTypes]`/
//! `[Xorg]`/`[Xvnc]`/`[X11rdp]`/`[Security]`/`[Sessions]`/`[Chansrv]`/`[SessionEnv]`
//! セクションと `port`/`crypt_level`/`fork`/`bitmap_cache`/`security_layer`/
//! `type`/`username`/`password`/`param` 等既知キーを計数する。
//!
//! ```
//! use izanagi_kit::xrdpconf;
//! let conf = b"[Globals]\nport=3389\ncrypt_level=high\nbitmap_cache=true\n[Xorg]\nname=Xorg\nparam=Xorg\n";
//! assert!(xrdpconf::detect(conf));
//! let c = xrdpconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.known_keys, 5);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数。
    pub known_sections: usize,
    /// `key=value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "globals",
    "logging",
    "loggingperlogger",
    "channels",
    "sessiontypes",
    "session types",
    "xorg",
    "xvnc",
    "x11rdp",
    "vnc",
    "neutrinordp",
    "rdpany",
    "security",
    "sessions",
    "chansrv",
    "sessionenv",
    "xrdp1",
    "xrdp2",
    "xrdp3",
    "xrdp4",
    "xrdp5",
    "xrdp6",
    "xrdp7",
    "xrdp8",
    "xrdp9",
];

const KNOWN_KEYS: &[&str] = &[
    "port",
    "use_vsock",
    "crypt_level",
    "certificate",
    "key_file",
    "ssl_protocols",
    "tls_ciphers",
    "autorun",
    "allow_channels",
    "allow_multimon",
    "bitmap_cache",
    "bitmap_compression",
    "bulk_compression",
    "new_cursors",
    "use_fastpath",
    "blue",
    "grey",
    "dark_grey",
    "ls_top_window_bg_color",
    "ls_width",
    "ls_height",
    "ls_bg_color",
    "ls_logo_filename",
    "ls_logo_x_pos",
    "ls_logo_y_pos",
    "ls_title",
    "ls_btn_ok_x_pos",
    "ls_btn_ok_y_pos",
    "ls_btn_cancel_x_pos",
    "ls_btn_cancel_y_pos",
    "security_layer",
    "max_bpp",
    "fork",
    "tcp_nodelay",
    "tcp_keepalive",
    "tcp_send_buffer_bytes",
    "tcp_recv_buffer_bytes",
    "tcp_rmem",
    "tcp_wmem",
    "hidelogwindow",
    "physical_keyboard_layout_subtype",
    "enable_token_login",
    "domain_user_separator",
    "pamerrortxt",
    "errortxt",
    "name",
    "lib",
    "username",
    "password",
    "ip",
    "type",
    "param",
    "code",
    "session_relationship",
    "channel_name",
    "disabled",
    "enable_dynamic_resizing",
    "enable_rdpdr",
    "enable_rdpsnd",
    "enable_cliprdr",
    "enable_rdpdis",
    "enabled",
    "allow_channels_per_session_type",
    "runtime_port",
    "param_xorg",
    "param_xvnc",
    "param_x11rdp",
    "param_vnc_any",
    "param_neutrinordp_any",
    "listen_address",
    "listen_port",
    "enable_user_window_manager",
    "default_wm",
    "user_window_manager",
    "reconnect_sh",
    "file_to_run",
    "keymap_file",
    "keymap_load",
    "keymap_dump",
    "xorg_params",
    "xvnc_params",
    "x11rdp_params",
    "sessvc_items",
    "su_to_uid",
    "no_show_users",
    "terminal_server",
    "ts_user_enable",
    "vnc_any",
    "neutrinordp",
    "console",
    "f1",
    "f2",
    "chan_param",
    "lbound",
    "ubound",
    "xserverbpp",
    "ls_title_x_pos",
    "input_username",
    "input_password",
    "input_combination",
    "delay_ms",
];

/// `xrdp.ini`/`sesman.ini` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_sections >= 1 && c.known_keys >= 3) || c.known_keys >= 5,
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
            if KNOWN_SECTIONS.contains(&name.as_str()) || name.starts_with("xrdp") {
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
        if KNOWN_KEYS.contains(&key.as_str()) || key.starts_with("param-") {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.sections > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[Globals]\nport=3389\ncrypt_level=high\nbitmap_cache=true\nfork=yes\nsecurity_layer=negotiate\n[Logging]\nLogFile=xrdp.log\nLogLevel=INFO\n[Xorg]\nname=Xorg\nparam=Xorg\nparam=-config\n[Xvnc]\nname=Xvnc\nparam=Xvnc\n";

    #[test]
    fn detects_xrdp() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.known_sections, 4);
        assert_eq!(c.entries, 12);
        assert_eq!(c.known_keys, 10);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[a]\nx=1\ny=2\n";
        assert!(!detect(ini));
    }
}
