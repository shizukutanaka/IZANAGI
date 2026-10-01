//! Irssi `~/.irssi/config` — `name = (`/`name = {` トップブロック内に
//! `key = value;` 設定と `Name = { ... };` 辞書エントリが並ぶ形式。
//!
//! ```
//! let cfg = b"servers = (\n  {\n    address = \"irc.libera.chat\";\n    chatnet = \"Libera\";\n    port = \"6697\";\n    use_tls = \"yes\";\n  }\n);\nsettings = {\n  core = {\n    real_name = \"shizuku\";\n    nick = \"shizuku\";\n  };\n};\nhilights = ( { text = \"shizuku\"; nick = \"yes\"; } );\n";
//! assert!(izanagi_kit::irssi::detect(cfg));
//! let c = izanagi_kit::irssi::parse(cfg).unwrap();
//! assert_eq!(c.top_blocks, 3);
//! assert_eq!(c.assignments, 6);
//! ```

/// Irssi config の既知トップレベルブロック名。
const TOP_BLOCKS: &[&str] = &[
    "servers",
    "chatnets",
    "channels",
    "hilights",
    "ignores",
    "settings",
    "aliases",
    "keyboard",
    "statusbar",
    "logs",
    "windows",
    "scripts",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// 列0の `name = (`/`name = {` ブロック数。
    pub top_blocks: usize,
    /// 既知名のトップブロック数。
    pub known_blocks: usize,
    /// インデントされた `Name = {` 辞書エントリ数。
    pub dict_entries: usize,
    /// `key = value;` 代入行数。
    pub assignments: usize,
    /// `;` 終端を持つ行数。
    pub semicolon_lines: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// Irssi config らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_blocks >= 2 && c.assignments >= 3
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
        top_blocks: 0,
        known_blocks: 0,
        dict_entries: 0,
        assignments: 0,
        semicolon_lines: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.ends_with(';') {
            c.semicolon_lines += 1;
        }
        let indented = raw.len() != raw.trim_start().len();
        if let Some((name, rhs)) = t.split_once('=') {
            let name = name.trim();
            let rhs = rhs.trim();
            let name_ok = !name.is_empty()
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '"'));
            if !name_ok {
                continue;
            }
            if rhs.starts_with('{') {
                if indented {
                    c.dict_entries += 1;
                } else {
                    c.top_blocks += 1;
                    if TOP_BLOCKS.contains(&name) {
                        c.known_blocks += 1;
                    }
                }
            } else if rhs.starts_with('(') {
                if !indented {
                    c.top_blocks += 1;
                    if TOP_BLOCKS.contains(&name) {
                        c.known_blocks += 1;
                    }
                }
            } else if rhs.ends_with(';') {
                c.assignments += 1;
            }
        }
    }
    if c.top_blocks == 0 || c.assignments == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# irssi config\nservers = (\n  {\n    address = \"irc.libera.chat\";\n    chatnet = \"Libera\";\n    port = \"6697\";\n    autoconnect = \"yes\";\n  },\n  {\n    address = \"irc.oftc.net\";\n    chatnet = \"OFTC\";\n    port = \"6697\";\n  }\n);\nchatnets = {\n  Libera = { type = \"IRC\"; };\n};\nsettings = {\n  core = {\n    real_name = \"shizuku\";\n    nick = \"ume\";\n  };\n  \"fe-common/core\" = { autocomplete = \"off\"; };\n};\n";

    #[test]
    fn detects_irssi() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.top_blocks, 3);
        assert_eq!(c.known_blocks, 3);
        assert_eq!(c.dict_entries, 3);
        assert_eq!(c.assignments, 9);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[server]\nhost = irc.libera.chat\nport = 6697\n"));
        assert!(parse(b"hello = world\n").is_none());
    }
}
