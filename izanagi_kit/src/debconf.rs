//! `debconf-set-selections` 事前設定ファイルの認識と計数。
//!
//! debconf selections は `<owner> <question> <type> <value>` の4トークン形式:
//! `locales locales/locales_to_be_generated multiselect en_US.UTF-8 UTF-8`、
//! `tzdata tzdata/Areas select Asia`、`console-data console-data/keymap/policy select Select keymap from full list`。
//! owner はパッケージ名(`d-i`/`anna` を除く — それは `preseed` モジュール)、
//! question は `<package>/<name>`、`type` は `string`/`boolean`/`select`/
//! `multiselect`/`passwd`/`password`/`note`/`title`/`text`/`error`/`seen`/
//! `select` の既知集合。
//!
//! ```
//! let b = b"# debconf selections\nlocales locales/locales_to_be_generated multiselect en_US.UTF-8 UTF-8, ja_JP.UTF-8 UTF-8\nlocales locales/default_environment_locale select ja_JP.UTF-8\ntzdata tzdata/Areas select Asia\ntzdata tzdata/Zones/Asia select Tokyo\nkeyboard-configuration keyboard-configuration/model select SKIP\nconsole-setup console-setup/charmap47 select UTF-8\n";
//! assert!(izanagi_kit::debconf::detect(b));
//! let c = izanagi_kit::debconf::parse(b).unwrap();
//! assert_eq!(c.entries, 6);
//! assert_eq!(c.selects, 5);
//! assert_eq!(c.multiselects, 1);
//! assert_eq!(c.owners, 4); // locales/tzdata/keyboard-configuration/console-setup
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<owner> <question> <type> <value>` エントリ行の個数。
    pub entries: usize,
    /// 先頭トークン(パッケージ名)の種類数。
    pub owners: usize,
    /// `select` 型の個数。
    pub selects: usize,
    /// `multiselect` 型の個数。
    pub multiselects: usize,
    /// `boolean` 型の個数。
    pub booleans: usize,
    /// `string`/`note`/`title`/`text`/`error`/`passwd`/`password` 型の個数。
    pub strings: usize,
    /// `seen` 型の個数。
    pub seens: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const TYPES: &[&str] = &[
    "string",
    "boolean",
    "select",
    "multiselect",
    "passwd",
    "password",
    "note",
    "title",
    "text",
    "error",
    "seen",
];

fn fields(s: &str) -> Option<(&str, &str, &str)> {
    let mut it = s.split_whitespace();
    let owner = it.next()?;
    let q = it.next()?;
    let ty = it.next()?;
    Some((owner, q, ty))
}

/// debconf selections らしさを返す。非 `d-i` owner の型付き行が複数。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if let Some((owner, q, ty)) = fields(s) {
            if owner != "d-i"
                && owner != "anna"
                && owner != "tasksel"
                && q.contains('/')
                && TYPES.contains(&ty)
            {
                hits += 1;
            }
        }
    }
    hits >= 3
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        owners: 0,
        selects: 0,
        multiselects: 0,
        booleans: 0,
        strings: 0,
        seens: 0,
        comments: 0,
    };
    let mut owners: std::vec::Vec<&str> = std::vec::Vec::new();
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let Some((owner, q, ty)) = fields(s) else {
            continue;
        };
        if !q.contains('/') || !TYPES.contains(&ty) {
            continue;
        }
        c.entries += 1;
        match ty {
            "select" => c.selects += 1,
            "multiselect" => c.multiselects += 1,
            "boolean" => c.booleans += 1,
            "seen" => c.seens += 1,
            _ => c.strings += 1,
        }
        if !owners.contains(&owner) {
            owners.push(owner);
            c.owners += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(
            b"locales a/b multiselect x\ntzdata t/Areas select Asia\nkeyboard k/model select SKIP\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"d-i a/b string x\nd-i c/d string y\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"locales a/b multiselect x\ntzdata t/A select Asia\ntzdata t/Z select Tokyo\nkbd k/m boolean true\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 4);
        assert_eq!(c.selects, 2);
        assert_eq!(c.multiselects, 1);
        assert_eq!(c.booleans, 1);
        assert_eq!(c.owners, 3);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"a x/y string z\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}
