//! HexChat `hexchat.conf` — セクション無しの平坦な `key = value` 群
//! (`nick_1`, `irc_user_name`, `gui_win_height`, `dcc_ip` …)。
//!
//! ```
//! let cfg = b"version = 2.16.1\nauto_connect = Libera\ncompletion_amount = 5\nirc_user_name = shizuku\nnick_1 = ume\nnick_2 = ume_\nstamp_log = 1\ntext_quiet = 1\n";
//! assert!(izanagi_kit::hexchat::detect(cfg));
//! let c = izanagi_kit::hexchat::parse(cfg).unwrap();
//! assert_eq!(c.entries, 8);
//! assert_eq!(c.known_entries, 8);
//! ```

/// HexChat の既知設定キー (完全一致)。
const KNOWN_KEYS: &[&str] = &[
    "version",
    "away_show_once",
    "away_size_max",
    "completion_auto",
    "identd",
    "last_save",
    "nick_alternate",
    "nick_suffix",
    "perl_warnings",
    "proxy_auto",
    "timestamps",
    "url_grabber",
];

/// 設定キーの既知プレフィックス群。
const PREFIXES: &[&str] = &[
    "auto_connect",
    "away_",
    "completion_",
    "dcc_",
    "dns_",
    "gui_",
    "identd_",
    "input_",
    "irc_",
    "logging_",
    "notify_",
    "perc_",
    "proxy_",
    "sound_",
    "tabs_",
    "text_",
    "timestamp_",
    "url_",
    "warn_",
    "nick_",
    "stamp_",
    "ssl_",
];

fn is_known(key: &str) -> bool {
    KNOWN_KEYS.contains(&key) || PREFIXES.iter().any(|p| key.starts_with(p))
}

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キーの行数。
    pub known_entries: usize,
    /// 整数値の行数。
    pub numeric_values: usize,
    /// `0`/`1` ブール値の行数。
    pub bool_values: usize,
}

/// HexChat conf らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.entries >= 3 && c.known_entries >= 3
}

/// 行を走査して集計する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        entries: 0,
        known_entries: 0,
        numeric_values: 0,
        bool_values: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with('[') {
            return None; // セクションを持たない形式
        }
        let Some((key, val)) = t.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.')
        {
            continue;
        }
        c.entries += 1;
        if is_known(key) {
            c.known_entries += 1;
        }
        let v = val.trim();
        if v.bytes().all(|ch| ch.is_ascii_digit()) && !v.is_empty() {
            c.numeric_values += 1;
            if v == "0" || v == "1" {
                c.bool_values += 1;
            }
        }
    }
    if c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"version = 2.16.1\nauto_connect = Libera\ncompletion_amount = 5\ndcc_get_nick = 0\ngui_win_height = 603\nirc_user_name = ume\nlogging_open = 0\nnick_1 = shizuku\nproxy_host = localhost\n";

    #[test]
    fn detects_hexchat() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.known_entries, 9);
        assert_eq!(c.numeric_values, 4);
        assert_eq!(c.bool_values, 2);
    }

    #[test]
    fn rejects_generic() {
        assert!(!detect(b"[ui]\ncolor = red\nsize = 3\n"));
        assert!(!detect(b"version = 1.0\nfoo = bar\nbaz = qux\n"));
    }
}
