//! WezTerm `wezterm.lua` パーサ。
//!
//! `local wezterm = require 'wezterm'` ヘッダ、`config.<opt> =`/`config.<opt>(...)`
//! 呼び出し、`return config` を計数する。
//!
//! ```
//! use izanagi_kit::weztermconf;
//! let conf = b"local wezterm = require 'wezterm'\nlocal config = wezterm.config_builder()\nconfig.font_size = 12\nconfig.color_scheme = 'nord'\nreturn config\n";
//! assert!(weztermconf::detect(conf));
//! let c = weztermconf::parse(conf).unwrap();
//! assert_eq!(c.config_entries, 2);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `local wezterm = require` 行数。
    pub requires: usize,
    /// `config.<opt> =` 代入数。
    pub config_entries: usize,
    /// `wezterm.<fn>(` 呼び出し数。
    pub wezterm_calls: usize,
    /// `return <ident>` 行数。
    pub returns: usize,
}

/// `wezterm.lua` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.requires >= 1 && (c.config_entries >= 1 || c.wezterm_calls >= 1),
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
        requires: 0,
        config_entries: 0,
        wezterm_calls: 0,
        returns: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with("--") {
            continue;
        }
        if (t.starts_with("local ") || t.starts_with("wezterm."))
            && t.contains("require")
            && t.contains("wezterm")
        {
            c.requires += 1;
            continue;
        }
        if t.starts_with("config.") {
            c.config_entries += 1;
            continue;
        }
        if t.contains("wezterm.") && t.contains('(') {
            c.wezterm_calls += 1;
            continue;
        }
        if t.starts_with("return") {
            c.returns += 1;
        }
    }
    (c.requires > 0 || c.config_entries > 0 || c.wezterm_calls > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"local wezterm = require 'wezterm'\nlocal config = wezterm.config_builder()\nconfig.font_size = 12\nconfig.color_scheme = 'nord'\nconfig.font = wezterm.font('Fira Code')\nconfig.keys = {}\nconfig.enable_tab_bar = false\nreturn config\n";

    #[test]
    fn detects_wezterm_lua() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.requires, 1);
        assert_eq!(c.wezterm_calls, 1);
        assert_eq!(c.config_entries, 5);
        assert_eq!(c.returns, 1);
    }

    #[test]
    fn rejects_other_lua() {
        let lua = b"local m = require 'mylib'\nprint(m.x)\nreturn m\n";
        assert!(!detect(lua));
    }
}
