//! `settings.xml` (openMSX Emulator) 検出モジュール。
//!
//! openMSX の設定は XML 形式で、`<settings>` ルート配下に
//! `<setting id="renderer">SDLGL-PP</setting>` のような
//! `<setting id="…">` 要素が並ぶ。既知 id には `renderer`/
//! `scale_algorithm`/`scale_factor`/`vsync`/`scanline`/`blur`/
//! `glow`/`deinterlace`/`limitsprites`/`savestates_on_quit`/
//! `throttle`/`speed`/`resampler`/`master_volume`/`mute`/
//! `firmwareswitch`/`kbd_rom_keymap`/`osd_keyboard`/
//! `horizontal_stretch`/`display_deform`/`min_frameskip`/
//! `max_frameskip`/`fullspeedwhenloading`/`accuracy`/
//! `reactor_residency`/`power_management` 等がある。
//!
//! ```
//! let b = br##"<settings>
//! <setting id="renderer">SDLGL-PP</setting>
//! <setting id="scale_factor">3</setting>
//! <setting id="scale_algorithm">simple</setting>
//! <setting id="vsync">true</setting>
//! </settings>"##;
//! let c = izanagi_kit::openmsx::parse(b);
//! assert!(izanagi_kit::openmsx::detect(b));
//! assert_eq!(c.settings, 4);
//! ```

const IDS: &[&str] = &[
    "accuracy",
    "blur",
    "brightness",
    "contrast",
    "deinterlace",
    "display_deform",
    "fullspeedwhenloading",
    "gamma",
    "glow",
    "horizontal_stretch",
    "kbd_rom_keymap",
    "limitsprites",
    "master_volume",
    "max_frameskip",
    "min_frameskip",
    "mute",
    "osd_keyboard",
    "power_management",
    "reactor_residency",
    "renderer",
    "resampler",
    "savestates_on_quit",
    "scale_algorithm",
    "scale_factor",
    "scanline",
    "speed",
    "throttle",
    "vsync",
];

fn setting_id(t: &str) -> Option<&str> {
    let a = t.find("<setting id=\"")? + 13;
    let e = t[a..].find('"')?;
    Some(&t[a..a + e])
}

fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `b` が openMSX settings.xml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = strip_comments(std::str::from_utf8(b).unwrap_or(""));
    let mut root = 0usize;
    let mut known = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("<!--") || tr.starts_with("<?") {
            continue;
        }
        if tr.starts_with("<settings") {
            root += 1;
        }
        if let Some(id) = setting_id(tr) {
            if IDS.contains(&id) {
                known += 1;
            }
        }
    }
    (root >= 1 && known >= 2) || known >= 4
}

/// openMSX settings.xml の統計。
#[derive(Debug, Default, Clone)]
pub struct OpenmsxConf {
    /// `<settings>` ルート行数。
    pub root: usize,
    /// 既知 id を持つ `<setting>` 要素数。
    pub settings: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を openMSX settings.xml として統計する。
pub fn parse(b: &[u8]) -> OpenmsxConf {
    let t = strip_comments(std::str::from_utf8(b).unwrap_or(""));
    let mut c = OpenmsxConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with("<!--") {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("<settings") {
            c.root += 1;
        }
        if let Some(id) = setting_id(tr) {
            if IDS.contains(&id) {
                c.settings += 1;
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
        let b = br##"<settings>
<setting id="renderer">SDLGL-PP</setting>
<setting id="vsync">true</setting>
</settings>"##;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.root, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(br##"<settings></settings>"##));
        assert!(!detect(
            br##"<root>
<setting id="x">1</setting>
<setting id="y">2</setting>
</root>"##
        ));
        assert!(!detect(
            b"<settings>\n<setting id=\"renderer\">x</setting>\n"
        ));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = br##"<!-- <setting id="renderer">x</setting> -->"##;
        assert!(!detect(b));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.settings, 0);
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
