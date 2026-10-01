//! Windows `.rdp` (Remote Desktop Connection) ファイル パーサ。
//!
//! `name:type:value` 形式 (`s:`=文字列, `i:`=整数, `b:`=バイナリ) と
//! `full address`/`screen mode id`/`desktopwidth`/`username`/`authentication level`/
//! `audiomode`/`redirectclipboard` 等既知キーを計数する。
//!
//! ```
//! use izanagi_kit::rdpfile;
//! let f = b"full address:s:192.168.1.10\nscreen mode id:i:2\nusername:s:admin\n";
//! assert!(rdpfile::detect(f));
//! let c = rdpfile::parse(f).unwrap();
//! assert_eq!(c.entries, 3);
//! assert_eq!(c.known_keys, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `name:type:value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
    /// `i:` 型行数。
    pub int_values: usize,
    /// `s:` 型行数。
    pub str_values: usize,
    /// `b:` 型行数。
    pub bin_values: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "full address",
    "alternate full address",
    "server port",
    "screen mode id",
    "desktopwidth",
    "desktopheight",
    "session bpp",
    "winposstr",
    "compression",
    "keyboardhook",
    "displayconnectionbar",
    "showconnectionbar",
    "disable wallpaper",
    "disable full window drag",
    "disable menu anims",
    "disable themes",
    "disable cursor setting",
    "bitmapcachepersistenable",
    "bitmapcachesize",
    "allow desktop composition",
    "allow font smoothing",
    "audiomode",
    "redirectdrives",
    "redirectprinters",
    "redirectcomports",
    "redirectsmartcards",
    "redirectclipboard",
    "redirectposdevices",
    "redirectdirectx",
    "redirectlocation",
    "drivestoredirect",
    "audiocapturemode",
    "videoplaybackmode",
    "authentication level",
    "prompt for credentials",
    "negotiate security layer",
    "remoteapplicationmode",
    "remoteapplicationname",
    "remoteapplicationprogram",
    "remoteapplicationfile",
    "remoteapplicationicon",
    "remoteapplicationcmdline",
    "alternate shell",
    "shell working directory",
    "gatewayhostname",
    "gatewayusagemethod",
    "gatewaycredentialssource",
    "gatewayprofileusagemethod",
    "promptcredentialonce",
    "use redirection server name",
    "rdgiskdcproxy",
    "kdcproxyname",
    "username",
    "domain",
    "password",
    "connection type",
    "networkautodetect",
    "bandwidthautodetect",
    "enableworkspacereconnect",
    "disableconnectionsharing",
    "loadbalanceinfo",
    "use multimon",
    "span monitors",
    "selectedmonitors",
    "maximizetocurrentdisplays",
    "singlemoninwindowedmode",
    "smart sizing",
    "dynamic resolution",
    "desktop size id",
    "administrative session",
    "connect to console",
    "public mode",
    "bitmapcachepersistfilesize",
    "autoreconnection enabled",
    "autoreconnect max retry attempts",
    "remoteapplicationexpandcmdline",
    "remoteapplicationexpandworkingdir",
    "remoteapplicationguid",
    "remoteapplicationiconextensions",
    "remoteappdisablesnapping",
    "disableremoteappcapscheck",
    "support url",
    "prompt for credentials on client",
    "use multimon",
    "username:s",
    "devicestoredirect",
    "usbdevicestoredirect",
    "webauthn redirection",
    "enablerdsaadauth",
    "rdgatewayaadpurpose",
    "rdgwauthmethod",
    "camerastoredirect",
    "redirectlocation",
];

/// `.rdp` ファイルらしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.entries >= 2 && c.known_keys >= 2,
        None => false,
    }
}

/// `name:type:value` 行を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        known_keys: 0,
        int_values: 0,
        str_values: 0,
        bin_values: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() {
            continue;
        }
        let Some(c1) = t.find(':') else {
            continue;
        };
        let Some(c2) = t[c1 + 1..].find(':').map(|i| c1 + 1 + i) else {
            continue;
        };
        let ty = &t[c1 + 1..c2];
        if !matches!(ty, "s" | "i" | "b") {
            continue;
        }
        let key = t[..c1].trim().to_ascii_lowercase();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'-' | b'_'))
        {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
        match ty {
            "i" => c.int_values += 1,
            "s" => c.str_values += 1,
            _ => c.bin_values += 1,
        }
    }
    (c.entries > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"screen mode id:i:2\nuse multimon:i:0\ndesktopwidth:i:1920\ndesktopheight:i:1080\nsession bpp:i:32\nfull address:s:192.168.1.10\nusername:s:admin\naudiomode:i:0\nredirectclipboard:i:1\nprompt for credentials:i:0\n";

    #[test]
    fn detects_rdp() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 10);
        assert_eq!(c.known_keys, 10);
        assert_eq!(c.int_values, 8);
        assert_eq!(c.str_values, 2);
    }

    #[test]
    fn rejects_other() {
        let t = b"key=value\nother:value\n";
        assert!(!detect(t));
    }
}
