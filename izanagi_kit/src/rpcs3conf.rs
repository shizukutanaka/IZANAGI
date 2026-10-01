//! RPCS3 `config.yml` の認識と計数。
//!
//! RPCS3 の設定はトップレベルに `Core:`/`VFS:`/`Video:`/`Audio:`/`Input/Output:`/
//! `System:`/`Net:`/`Miscellaneous:`/`Log:`/`Debug:`/`Compatibility:` の
//! セクションキーを持ち、下位に空白を含むサブキー(`PPU Decoder:`/`SPU Decoder:`/
//! `Renderer:`/`Write Color Buffers:`/`$(EmulatorDir):`/`Enable /host_root:`/
//! `License Area:`/`Internet Status:` 等)をネストする。
//!
//! ```
//! let b = b"Core:\n  PPU Decoder: Recompiler (LLVM)\n  SPU Decoder: Recompiler (LLVM)\n  Enable TSX: Enabled\nVFS:\n  $(EmulatorDir): ''\n  /dev_hdd0: $(EmulatorDir)dev_hdd0/\n  Enable /host_root: false\nVideo:\n  Renderer: Vulkan\n  Resolution: 1280x720\n  Write Color Buffers: true\nAudio:\n  Renderer: Cubeb\nSystem:\n  License Area: SCEA\n";
//! assert!(izanagi_kit::rpcs3conf::detect(b));
//! let c = izanagi_kit::rpcs3conf::parse(b).unwrap();
//! assert_eq!(c.top_sections, 5);
//! assert_eq!(c.entries, 16);
//! assert_eq!(c.known_sections, 5);
//! assert_eq!(c.bool_entries, 3); // Enabled + false + true
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// インデント 0 のセクションキー(`Core:`/`VFS:`/…)数。
    pub top_sections: usize,
    /// 既知 RPCS3 セクション名の数。
    pub known_sections: usize,
    /// `Key: value`/`Key:` 行の総数(サブキー含む)。
    pub entries: usize,
    /// 値が `true`/`false`/`Enabled`/`Disabled` の行数。
    pub bool_entries: usize,
    /// 値が `'…'`/`"…"` 引用の行数。
    pub quoted_entries: usize,
}

const KNOWN_TOP: &[&str] = &[
    "Core",
    "VFS",
    "Video",
    "Audio",
    "Input/Output",
    "System",
    "Net",
    "Miscellaneous",
    "Log",
    "Debug",
    "Compatibility",
    "Loader",
    "Current Libraries",
    "Netplay",
    "Global",
];

fn indent_of(s: &str) -> usize {
    s.bytes().take_while(|c| *c == b' ').count()
}

/// `Key:` / `Key: value` のキー部分を返す。キーは空白・記号を含み得るが
/// 末尾 `:` までとし、YAML コメント・フロー値は除外する。
fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find(':')?;
    let k = s[..i].trim();
    if k.is_empty()
        || k.starts_with('-')
        || k.starts_with('#')
        || k.starts_with('\'')
        || k.starts_with('"')
    {
        return None;
    }
    if !k.bytes().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                b' ' | b'/'
                    | b'('
                    | b')'
                    | b'-'
                    | b'_'
                    | b'.'
                    | b'$'
                    | b'@'
                    | b'%'
                    | b'+'
                    | b'!'
                    | b'&'
                    | b'*'
                    | b'='
                    | b'<'
                    | b'>'
                    | b','
                    | b'?'
                    | b'~'
                    | b'`'
            )
    }) {
        return None;
    }
    let rest = &s[i + 1..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some((k, rest.trim()))
}

/// `rpcs3 config.yml` らしさを返す。既知トップセクション ≥2。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut known = 0usize;
    for line in t.lines() {
        if indent_of(line) == 0 {
            if let Some((k, _)) = key_of(line.trim_end()) {
                if KNOWN_TOP.contains(&k) {
                    known += 1;
                }
            }
        }
    }
    known >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let Ok(t) = std::str::from_utf8(b) else {
        return None;
    };
    let mut c = Counts {
        top_sections: 0,
        known_sections: 0,
        entries: 0,
        bool_entries: 0,
        quoted_entries: 0,
    };
    for line in t.lines() {
        let s = line.trim_end();
        if s.trim().is_empty() || s.trim_start().starts_with('#') {
            continue;
        }
        let Some((k, v)) = key_of(s) else {
            continue;
        };
        c.entries += 1;
        if indent_of(line) == 0 {
            c.top_sections += 1;
            if KNOWN_TOP.contains(&k) {
                c.known_sections += 1;
            }
        }
        if v == "true" || v == "false" || v == "Enabled" || v == "Disabled" {
            c.bool_entries += 1;
        }
        if v.starts_with('\'') || v.starts_with('"') {
            c.quoted_entries += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rpcs3() {
        let b = b"Core:\n  PPU Decoder: Interpreter (precise)\nNet:\n  Internet Status: Connected\nMiscellaneous:\n  Exit RPCS3 when process finishes: true\n";
        let c = parse(b).unwrap();
        assert_eq!(c.top_sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 6);
        assert_eq!(c.bool_entries, 1);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(parse(b"name: x\non: push\njobs:\n  test:\n").is_none());
    }
}
