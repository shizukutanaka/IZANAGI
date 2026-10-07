//! `zathurarc` 検出モジュール。
//!
//! zathura(PDF viewer)の設定は `set <option> <value>`/
//! `map <key> <command>`/`unmap <key>`/`include <file>` の
//! ディレクティブ形式で、`adjust-open`/`scroll-step`/`zoom-min`/
//! `zoom-max`/`recolor`/`recolor-lightcolor`/`recolor-darkcolor`/
//! `guioptions`/`statusbar-fg`/`statusbar-bg`/`completion-bg`/
//! `notification-error-bg`/`font`/`selection-clipboard`/
//! `render-loading`/`synctex`/`database`/`sandbox`/
//! `page-padding`/`first-page-column`/`zoom-step`/
//! `incremental-search`/`highlight-color`/`highlight-active-color`/
//! `index-fg`/`window-title-home-tilde`/`window-title-page`/
//! `vertical-center`/`selection-notification`/`open-first-page`
//! 等のオプションで構成される。
//!
//! ```
//! let b = b"set adjust-open width\n\
//!           set recolor true\n\
//!           set recolor-lightcolor rgba(0,0,0,0)\n\
//!           set recolor-darkcolor \"#93a1a1\"\n\
//!           map <C-i> recolor\n";
//! let c = izanagi_kit::zathurarc::parse(b);
//! assert!(izanagi_kit::zathurarc::detect(b));
//! assert_eq!(c.sets, 4);
//! ```

const OPTIONS: &[&str] = &[
    "abort-clear",
    "adjust-open",
    "advance-pages-per-row",
    "completion-bg",
    "completion-fg",
    "completion-group-bg",
    "completion-group-fg",
    "completion-highlight-bg",
    "completion-highlight-fg",
    "database",
    "default-bg",
    "default-fg",
    "exec-command",
    "first-page-column",
    "font",
    "guioptions",
    "highlight-active-color",
    "highlight-color",
    "highlight-fg",
    "highlight-transparency",
    "incremental-search",
    "index-active-bg",
    "index-active-fg",
    "index-bg",
    "index-fg",
    "inputbar-bg",
    "inputbar-fg",
    "jumplist-size",
    "n-completion-items",
    "notification-bg",
    "notification-error-bg",
    "notification-error-fg",
    "notification-fg",
    "notification-warning-bg",
    "notification-warning-fg",
    "open-first-page",
    "page-padding",
    "page-right-to-left",
    "recolor",
    "recolor-darkcolor",
    "recolor-keephue",
    "recolor-lightcolor",
    "recolor-reverse-video",
    "render-loading",
    "render-loading-bg",
    "render-loading-fg",
    "sandbox",
    "scroll-full-overlap",
    "scroll-hstep",
    "scroll-step",
    "scroll-wrap",
    "search-hadjust",
    "selection-clipboard",
    "selection-notification",
    "show-directories",
    "show-hidden",
    "show-recent",
    "smooth-scroll",
    "startup-command",
    "statusbar-bg",
    "statusbar-fg",
    "statusbar-h-padding",
    "statusbar-home-tilde",
    "statusbar-v-padding",
    "synctex",
    "synctex-editor-command",
    "vertical-center",
    "window-icon",
    "window-icon-document",
    "window-title-basename",
    "window-title-home-tilde",
    "window-title-page",
    "window-title-use-tmp-path",
    "zoom-center",
    "zoom-max",
    "zoom-min",
    "zoom-step",
];

fn is_set(t: &str) -> bool {
    let mut it = t.split_whitespace();
    if it.next() != Some("set") {
        return false;
    }
    it.next().is_some_and(|o| OPTIONS.contains(&o))
}

/// `b` が zathurarc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut sets = 0usize;
    let mut maps = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_set(tr) {
            sets += 1;
        } else if tr.starts_with("map ") || tr.starts_with("unmap ") || tr.starts_with("include ") {
            maps += 1;
        }
    }
    sets >= 3 || (sets >= 1 && maps >= 2)
}

/// zathurarc の統計。
#[derive(Debug, Default, Clone)]
pub struct Zathurarc {
    /// `set` 行数。
    pub sets: usize,
    /// `map`/`unmap`/`include` 行数。
    pub maps: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を zathurarc として統計する。
pub fn parse(b: &[u8]) -> Zathurarc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Zathurarc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_set(tr) {
            c.sets += 1;
        } else if tr.starts_with("map ") || tr.starts_with("unmap ") || tr.starts_with("include ") {
            c.maps += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"set adjust-open width\nset recolor true\nset zoom-min 10\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sets, 3);
    }

    #[test]
    fn detects_with_map() {
        let b = b"set recolor true\nmap <C-i> recolor\nunmap a\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"set key value\nset foo bar\nset baz quux\n"));
        assert!(!detect(b"set adjust-open width\nset recolor true\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sets, 0);
    }
}
