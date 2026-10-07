//! `mpv.conf` 検出モジュール。
//!
//! mpv の設定は `key=value`(または `--key=value`)形式で、
//! `vo`/`ao`/`hwdec`/`profile`/`volume`/`speed`/`slang`/`alang`/
//! `sub-auto`/`ytdl-format`/`cache`/`demuxer-max-bytes`/
//! `interpolation`/`video-sync`/`deband`/`scale`/`dscale`/`cscale`/
//! `sigmoid-upscaling`/`correct-downscaling`/`hdr-compute-peak`/
//! `tone-mapping`/`fs`/`border`/`ontop`/`save-position-on-quit`/
//! `watch-later-directory`/`screenshot-format`/
//! `screenshot-directory`/`input-ipc-server`/`osd-level`/
//! `osd-duration`/`sub-font-size`/`ass-force-style`/`idle`/
//! `keep-open`/`cursor-autohide`/`osc`/`volume-max`/`af`/`vf`/
//! `gpu-context`/`gpu-api`/`fbo-format`/`blend-subtitles` 等の
//! キーで構成される。
//!
//! ```
//! let b = b"vo=gpu\n\
//!           hwdec=auto-safe\n\
//!           profile=high-quality\n\
//!           scale=ewa_lanczossharp\n\
//!           video-sync=display-resample\n\
//!           save-position-on-quit\n";
//! let c = izanagi_kit::mpvconf::parse(b);
//! assert!(izanagi_kit::mpvconf::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "af",
    "alang",
    "ao",
    "ao-pcm",
    "ar",
    "ass-force-style",
    "ass-shaper",
    "audio-channels",
    "audio-device",
    "audio-pitch-correction",
    "audio-spdif",
    "auto-window-resize",
    "blend-subtitles",
    "border",
    "cache",
    "cache-secs",
    "chapter-seek-threshold",
    "config",
    "correct-downscaling",
    "cscale",
    "cursor-autohide",
    "deband",
    "deband-grain",
    "deband-iterations",
    "deband-range",
    "deband-threshold",
    "demuxer-max-back-bytes",
    "demuxer-max-bytes",
    "demuxer-readahead-secs",
    "display-fps",
    "dscale",
    "dsize",
    "dvbin",
    "end",
    "external-files",
    "field-dominance",
    "fbo-format",
    "force-window",
    "framedrop",
    "fs",
    "fs-screen",
    "gamma",
    "gamma-auto",
    "geometry",
    "gpu-api",
    "gpu-context",
    "gpu-sw",
    "hdr-compute-peak",
    "hr-seek",
    "hwdec",
    "idle",
    "input-ipc-server",
    "interpolation",
    "keep-open",
    "length",
    "loop",
    "loop-file",
    "loop-playlist",
    "monitorpixelaspect",
    "no-keepaspect",
    "ontop",
    "osc",
    "osd-align-x",
    "osd-align-y",
    "osd-bar-align-x",
    "osd-bar-align-y",
    "osd-duration",
    "osd-font",
    "osd-font-size",
    "osd-level",
    "osd-margin-x",
    "osd-margin-y",
    "osd-playing-msg",
    "osd-scale",
    "osd-scale-by-window",
    "osd-status-msg",
    "pause",
    "profile",
    "panscan",
    "really-quiet",
    "save-position-on-quit",
    "scale",
    "scale-window",
    "screen",
    "screenshot-directory",
    "screenshot-format",
    "screenshot-high-bit-depth",
    "screenshot-jpeg-quality",
    "screenshot-png-compression",
    "screenshot-template",
    "secondary-sid",
    "shuffle",
    "sid",
    "sigmoid-upscaling",
    "slang",
    "speed",
    "spirv-compiler",
    "stop-screensaver",
    "stream-dump",
    "stretch-dvd-subs",
    "sub-ass",
    "sub-auto",
    "sub-codepage",
    "sub-color",
    "sub-delay",
    "sub-demuxer",
    "sub-file",
    "sub-font",
    "sub-font-size",
    "sub-forced-only",
    "sub-fps",
    "sub-gauss",
    "sub-gray",
    "sub-pos",
    "sub-scale",
    "sub-scale-by-window",
    "sub-scale-with-window",
    "sub-speed",
    "sub-visibility",
    "sws-scaler",
    "target-prim",
    "term-osd",
    "term-playing-msg",
    "term-status-msg",
    "title",
    "tone-mapping",
    "tone-mapping-param",
    "tscale",
    "tscale-clamp",
    "tscale-radius",
    "use-filedir-conf",
    "vf",
    "video-aspect",
    "video-aspect-method",
    "video-pan-x",
    "video-pan-y",
    "video-rotate",
    "video-sync",
    "video-sync-max-factor",
    "video-sync-max-video-change",
    "video-timing-offset",
    "video-unscaled",
    "video-zoom",
    "vo",
    "volume",
    "volume-max",
    "watch-later-directory",
    "wid",
    "x11-bypass-compositor",
    "ytdl",
    "ytdl-format",
    "ytdl-raw-options",
    "ytdl-exclude",
    "zoom",
];

fn is_key(t: &str) -> bool {
    let t = t.strip_prefix("--").unwrap_or(t);
    let k = t.split('=').next().unwrap_or(t).trim();
    let k = k.strip_prefix("no-").unwrap_or(k);
    KEYS.contains(&k)
}

/// `b` が mpv.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_key(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// mpv.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct MpvConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
    /// 総行数。
    pub lines: usize,
}

/// `b` を mpv.conf として統計する。
pub fn parse(b: &[u8]) -> MpvConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = MpvConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        c.lines += 1;
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_key(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"vo=gpu\nhwdec=auto\nscale=ewa_lanczossharp\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"vo=gpu\nhwdec=auto\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# vo=gpu\n# hwdec=auto\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
