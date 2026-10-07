//! `fceux.cfg` 検出モジュール。
//!
//! FCEUX(NES/FDS エミュレータ)の設定は `SDL.*` ドット区切りキーの
//! `key = value` 形式で、`SDL.VideoDriver`/`SDL.HideMouse`/
//! `SDL.Frameskip`/`SDL.DoubleBuffering`/`SDL.XResolution`/
//! `SDL.YResolution`/`SDL.Fullscreen`/`SDL.ClipSides`/`SDL.Sound`/
//! `SDL.Sound.Rate`/`SDL.Sound.BufSize`/`SDL.Sound.Quality`/
//! `SDL.Sound.Volume`/`SDL.Sound.LowPass`/`SDL.Vsync`/`SDL.ShowFPS`/
//! `SDL.OpenGL`/`SDL.OpenGLip`/`SDL.SpecialFilter`/`SDL.ScaleX`/
//! `SDL.ScaleY`/`SDL.NewPPU`/`SDL.NetworkServer`/`SDL.NetworkPort`/
//! `SDL.NetworkIP`/`SDL.NetworkGameKey`/`SDL.RecordHUD`/
//! `SDL.LagCounter`/`SDL.MovieReadOnly`/`SDL.GameGenie`/`SDL.Palette`/
//! `SDL.InputDisplay`/`SDL.FourScore`/`SDL.Arkanoid`/`SDL.AutoLoadState`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"SDL.VideoDriver = 0\n\
//!           SDL.HideMouse = 0\n\
//!           SDL.Frameskip = 0\n\
//!           SDL.DoubleBuffering = 1\n\
//!           SDL.XResolution = 1024\n\
//!           SDL.Sound.Rate = 44100\n";
//! let c = izanagi_kit::fceux::parse(b);
//! assert!(izanagi_kit::fceux::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

/// `b` が fceux.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.starts_with("SDL.") && tr.contains('=') {
            keys += 1;
        }
    }
    keys >= 3
}

/// fceux.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct FceuxConf {
    /// `SDL.*` キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
    /// 総行数。
    pub lines: usize,
}

/// `b` を fceux.cfg として統計する。
pub fn parse(b: &[u8]) -> FceuxConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = FceuxConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        c.lines += 1;
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("SDL.") && tr.contains('=') {
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
        let b = b"SDL.VideoDriver = 0\nSDL.HideMouse = 0\nSDL.Frameskip = 0\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"SDL.SDL = x\nfoo.bar = 1\nbaz.qux = 2\n"));
        assert!(!detect(b"SDL.VideoDriver = 0\nSDL.HideMouse = 0\n"));
        assert!(!detect(b"[SDL]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# SDL.VideoDriver = 0\n; SDL.HideMouse = 0\n";
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
