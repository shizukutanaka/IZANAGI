//! `picom.conf` 検出モジュール。
//!
//! picom (旧 compton) の設定ファイルは `key = value;` 行が主体の
//! フラットな形式。`--` 始まりの CLI フラグ風記述も可能。
//!
//! ```
//! let b = br#"backend = "glx";
//! vsync = true;
//! shadow = true;
//! shadow-radius = 12;
//! opacity-rule = [
//!   "80:class_g = 'URxvt'"
//! ];
//! "#;
//! let c = izanagi_kit::picom::parse(b);
//! assert!(izanagi_kit::picom::detect(b));
//! assert_eq!(c.assignments, 6);
//! assert!(c.array_values >= 1);
//! ```

const KEYS: &[&str] = &[
    "active-opacity",
    "animations",
    "backend",
    "blur-background",
    "blur-method",
    "blur-size",
    "blur-strength",
    "clip-shadow-above",
    "corner-radius",
    "detect-client-leader",
    "detect-client-opacity",
    "detect-rounded-corners",
    "detect-transient",
    "dithered-present",
    "fade-delta",
    "fade-exclude",
    "fade-in-step",
    "fade-out-step",
    "fading",
    "focus-exclude",
    "frame-opacity",
    "glx-copy-from-front",
    "glx-no-rebind-pixmap",
    "glx-no-stencil",
    "inactive-dim",
    "inactive-opacity",
    "inactive-opacity-override",
    "invert-color-include",
    "log-level",
    "mark-ovredir-focused",
    "mark-wmwin-focused",
    "max-brightness",
    "menu-opacity",
    "no-fading-destroyed-argb",
    "no-fading-openclose",
    "opacity-rule",
    "paint-on-overlay",
    "refresh-rate",
    "rounded-corners-exclude",
    "shadow",
    "shadow-color",
    "shadow-exclude",
    "shadow-ignore-shaped",
    "shadow-offset-x",
    "shadow-offset-y",
    "shadow-opacity",
    "shadow-radius",
    "shadow-red",
    "shadow-green",
    "shadow-blue",
    "transparent-clipping",
    "unredir-if-possible",
    "use-damage",
    "vsync",
    "wintypes",
    "xrender-sync-fence",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with("//")
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が picom.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
        }
    }
    keys >= 3
}

/// picom.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct PicomConf {
    /// 既知オプションの代入行数。
    pub keys: usize,
    /// 全代入行数。
    pub assignments: usize,
    /// `= [` 配列値を持つ行数。
    pub array_values: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を picom.conf として統計する。
pub fn parse(b: &[u8]) -> PicomConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = PicomConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if let Some(eq) = tr.find('=') {
            c.assignments += 1;
            if KEYS.iter().any(|k| key_present(tr, k)) {
                c.keys += 1;
            }
            if tr[eq..]
                .trim_start_matches('=')
                .trim_start()
                .starts_with('[')
            {
                c.array_values += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"backend = "glx";
vsync = true;
shadow = true;
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
        assert_eq!(c.assignments, 3);
    }

    #[test]
    fn detects_arrays() {
        let b = br#"backend = "xrender";
opacity-rule = [
  "80:class_g = 'URxvt'",
  "90:name *= 'popup'"
];
shadow-exclude = [
  "name = 'Notification'"
];
"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.array_values >= 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"backend = \"glx\"\n"));
        assert!(!detect(b"foo = 1\nbar = 2\nbaz = 3\n"));
        assert!(!detect(b"[section]\nkey = 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.assignments, 0);
    }
}
