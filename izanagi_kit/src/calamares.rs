//! Calamares インストーラ `settings.conf`/`netinstall.yaml`/`*.conf` の
//! 認識と計数。
//!
//! `settings.conf` は YAML で、トップレベルに `modules-search`、`sequence`、
//! `branding`、`prompt-install`、`dont-chroot`、`oem-setup`、
//! `disable-cancel`、`disable-cancel-during-exec`、`quit-at-end`、
//! `hide-back-and-next-during-exec` 等を持つ。`sequence:` は
//! `- show:`/`- exec:` グループのリストで、要素はモジュール名
//! (`welcome`/`partition`/`users`/`bootloader`/`finished` 等)または
//! `id: name`/`module: name@id`/`config:`/`weight:`/`timeout:` の
//! インスタンス指定。`netinstall.yaml` は `groups:`+`packages:` 系。
//!
//! ```
//! let b = b"---\nmodules-search: [ local, nonzero ]\nsequence:\n- show:\n  - welcome\n  - finished\n- exec:\n  - partition\n  - users\n  - bootloader\nbranding: default\nprompt-install: true\ndont-chroot: false\n";
//! assert!(izanagi_kit::calamares::detect(b));
//! let c = izanagi_kit::calamares::parse(b).unwrap();
//! assert_eq!(c.sequence_groups, 2); // show + exec
//! assert_eq!(c.sequence_modules, 5); // welcome/finished/partition/users/bootloader
//! assert_eq!(c.top_keys, 3); // branding/prompt-install/dont-chroot(sequence・modules-search 除く)
//! assert_eq!(c.bools, 2);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルスカラーキー(`branding`/`prompt-install`/`dont-chroot` 等)
    /// の個数。`sequence:`/`modules-search:` は含めない。
    pub top_keys: usize,
    /// `sequence:` 直下の `- show:`/`- exec:` グループの個数。
    pub sequence_groups: usize,
    /// sequence 内モジュール名の総数(`- welcome` 等、重複含む)。
    pub sequence_modules: usize,
    /// `true`/`false`/`yes`/`no` 値の個数。
    pub bools: usize,
    /// `key:`/`key: value` キー総数(任意の深さ)。
    pub keys: usize,
    /// `- ` リストアイテム総数(sequence 内・外問わず)。
    pub list_items: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "modules-search",
    "sequence",
    "branding",
    "prompt-install",
    "dont-chroot",
    "oem-setup",
    "disable-cancel",
    "disable-cancel-during-exec",
    "quit-at-end",
    "hide-back-and-next-during-exec",
    "allow-cancel-during-exec",
    "window-show",
    "chroot",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Calamares らしさを返す。`sequence:`+`- show:`/`- exec:` か
/// `branding:`+calamares キー。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("- show:") || s.starts_with("- exec:") {
            return true;
        }
        if s.ends_with(':') && s.len() > 1 && TOP_KEYS.contains(&s.trim_end_matches(':')) {
            hits += 1;
        }
        if let Some(eq) = s.find(':') {
            let k = s[..eq].trim_end();
            if TOP_KEYS.contains(&k) && k != "sequence" && k != "modules-search" {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        top_keys: 0,
        sequence_groups: 0,
        sequence_modules: 0,
        bools: 0,
        keys: 0,
        list_items: 0,
        comments: 0,
    };
    let mut in_seq = false;
    let mut in_group = false;
    for l in t.lines() {
        let s = l.trim_end();
        let st = s.trim_start();
        if st.is_empty() || st == "---" || st == "..." {
            continue;
        }
        if st.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let ind = s.len() - st.len();
        if ind == 0 && !st.starts_with('-') {
            in_seq = st.starts_with("sequence");
            in_group = false;
        }
        if st.starts_with("- show:") || st.starts_with("- exec:") {
            if in_seq {
                c.sequence_groups += 1;
            }
            c.list_items += 1;
            in_group = true;
            continue;
        }
        if st.starts_with("- ") || st == "-" {
            c.list_items += 1;
            if in_seq && in_group {
                let name = st.trim_start_matches('-').trim();
                if !name.is_empty() {
                    c.sequence_modules += 1;
                }
            }
            continue;
        }
        if let Some(col) = st.find(':') {
            let k = st[..col].trim_end();
            if !k.is_empty()
                && k.chars()
                    .all(|x| x.is_ascii_alphanumeric() || x == '-' || x == '_' || x == '.')
            {
                c.keys += 1;
                let v = st[col + 1..].trim();
                if matches!(v, "true" | "false" | "yes" | "no") {
                    c.bools += 1;
                }
                if ind == 0 && TOP_KEYS.contains(&k) && k != "sequence" && k != "modules-search" {
                    c.top_keys += 1;
                }
            }
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(b"sequence:\n- show:\n  - welcome\n"));
        assert!(detect(b"branding: default\nprompt-install: true\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo: bar\nbaz: qux\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"sequence:\n- show:\n  - welcome\n- exec:\n  - users\nbranding: default\ndisable-cancel: true\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sequence_groups, 2);
        assert_eq!(c.sequence_modules, 2);
        assert_eq!(c.top_keys, 2);
        assert_eq!(c.bools, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"branding: x\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
