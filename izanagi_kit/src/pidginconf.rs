//! Pidgin/libpurple `prefs.xml`・`accounts.xml` — `<purple>`/`<account>` ルートの
//! `<pref>`/`<setting>`/`<protocol>prpl-*` XML 設定形式。
//!
//! ```
//! let cfg = b"<purple version='1.0'>\n  <pref name='away' type='int' value='1'/>\n  <pref name='core'>\n    <pref name='idle_reporting' type='string' value='system'/>\n  </pref>\n</purple>\n";
//! assert!(izanagi_kit::pidginconf::detect(cfg));
//! let c = izanagi_kit::pidginconf::parse(cfg).unwrap();
//! assert_eq!(c.prefs, 3);
//! ```

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `<name` 開始要素数 (宣言・終了タグ除く)。
    pub elements: usize,
    /// `<pref` 要素数。
    pub prefs: usize,
    /// `<setting` 要素数。
    pub settings: usize,
    /// `<account>` ブロック数。
    pub accounts: usize,
    /// `prpl-` プロトコル行数。
    pub protocols: usize,
}

/// pidgin/libpurple XML 設定かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(s) = core::str::from_utf8(b) else {
        return false;
    };
    let has_root = s.contains("<purple") || s.contains("<account") || s.contains("<blist");
    has_root && (s.contains("<pref") || s.contains("<setting") || s.contains("prpl-"))
}

/// 行を走査して集計する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    if !s.contains('<') {
        return None;
    }
    let mut c = Counts {
        lines: 0,
        elements: 0,
        prefs: 0,
        settings: 0,
        accounts: 0,
        protocols: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        // 開始要素: `<x` で `</` `<?` `<!` でないもの。
        c.elements += count_open_elements(t);
        c.prefs += t.matches("<pref").count();
        c.settings += t.matches("<setting ").count() + t.matches("<setting>").count();
        if t.contains("<account>") {
            c.accounts += 1;
        }
        c.protocols += t.matches("prpl-").count();
    }
    if c.elements == 0 {
        return None;
    }
    Some(c)
}

/// 行内の開始要素 `<name` 数 (`</`、`<?`、`<!` を除外)。
fn count_open_elements(t: &str) -> usize {
    let mut n = 0usize;
    let bytes = t.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'<' && bytes[i + 1].is_ascii_alphabetic() {
            n += 1;
        }
        i += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREFS: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<purple version='1.0'>\n  <pref name='away' type='int' value='1'/>\n  <pref name='core'>\n    <pref name='idle_reporting' type='string' value='system'/>\n    <pref name='report_idle' type='string' value='system'/>\n  </pref>\n</purple>\n";

    const ACCOUNTS: &[u8] = b"<account version='1.0'>\n  <account>\n    <protocol>prpl-irc</protocol>\n    <name>ume@libera</name>\n    <settings>\n      <setting name='port' type='int'>6697</setting>\n      <setting name='ssl' type='bool'>1</setting>\n    </settings>\n  </account>\n</account>\n";

    #[test]
    fn detects_prefs() {
        assert!(detect(PREFS));
        let c = parse(PREFS).unwrap();
        assert_eq!(c.prefs, 4);
        assert_eq!(c.elements, 5);
    }

    #[test]
    fn detects_accounts() {
        assert!(detect(ACCOUNTS));
        let c = parse(ACCOUNTS).unwrap();
        assert_eq!(c.accounts, 1);
        assert_eq!(c.settings, 2);
        assert_eq!(c.protocols, 1);
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"<html><body>hi</body></html>"));
        assert!(!detect(b"key = value"));
        assert!(parse(b"no markup").is_none());
    }
}
