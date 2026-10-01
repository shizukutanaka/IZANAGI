//! Debian `preseed.cfg` 自動インストール応答ファイルの認識と計数。
//!
//! preseed は `d-i <question> <type> <value>` の4トークン形式が主体:
//! `d-i netcfg/hostname string myhost`、`d-i pkgsel/include string vim`、
//! `d-i passwd/root-password-crypted password $6$…`。
//! 型は `string`/`boolean`/`select`/`multiselect`/`passwd`/`password`/
//! `note`/`title`/`text`/`error`/`seen`。`seen` 行の値は `true`/`false`、
//! `boolean` も `true`/`false`。コメントは `#`、`# Only for` 注釈付き。
//! 所有者は `d-i` 主体の他 `anna`/`tasksel`/`pkgsel`/`partman-auto` 等。
//!
//! ```
//! let b = b"d-i debian-installer/locale string en_US\nd-i keyboard-configuration/xkb-keymap select jp\nd-i netcfg/hostname string unassigned-hostname\nd-i mirror/http/proxy string\nd-i passwd/make-user boolean false\nd-i passwd/root-password-crypted password $6$salt$hash\nd-i clock-setup/utc boolean true\nd-i time/zone string Asia/Tokyo\nd-i partman-auto/method string lvm\nd-i pkgsel/include string openssh-server vim\nd-i grub-installer/bootdev string /dev/sda\nd-i finish-install/reboot_in_progress note\n";
//! assert!(izanagi_kit::preseed::detect(b));
//! let c = izanagi_kit::preseed::parse(b).unwrap();
//! assert_eq!(c.entries, 12);
//! assert_eq!(c.strings, 9); // string 7 + password + note
//! assert_eq!(c.booleans, 2);
//! assert_eq!(c.selects, 1);
//! assert_eq!(c.owners, 1); // 全て d-i
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<owner> <question> <type> <value>` エントリ行の個数。
    pub entries: usize,
    /// `string`/`note`/`title`/`text`/`error`/`passwd`/`password` 型の個数。
    pub strings: usize,
    /// `boolean` 型の個数。
    pub booleans: usize,
    /// `select` 型の個数。
    pub selects: usize,
    /// `multiselect` 型の個数。
    pub multiselects: usize,
    /// `seen` 型の個数。
    pub seens: usize,
    /// 先頭トークン(`d-i`/`anna`/`tasksel`/`pkgsel`/`partman-*` 等)の種類数。
    pub owners: usize,
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

/// preseed らしさを返す。`d-i <q> <type> …` 行が複数あること。
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
        let mut it = s.split_whitespace();
        let Some(owner) = it.next() else {
            continue;
        };
        let Some(_q) = it.next() else {
            continue;
        };
        let Some(ty) = it.next() else {
            continue;
        };
        if (owner == "d-i" || owner == "anna" || owner == "tasksel") && TYPES.contains(&ty) {
            hits += 1;
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
        strings: 0,
        booleans: 0,
        selects: 0,
        multiselects: 0,
        seens: 0,
        owners: 0,
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
        let mut it = s.split_whitespace();
        let Some(owner) = it.next() else {
            continue;
        };
        let Some(_q) = it.next() else {
            continue;
        };
        let Some(ty) = it.next() else {
            continue;
        };
        if !TYPES.contains(&ty) {
            continue;
        }
        c.entries += 1;
        match ty {
            "boolean" => c.booleans += 1,
            "select" => c.selects += 1,
            "multiselect" => c.multiselects += 1,
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
            b"d-i a/locale string en_US\nd-i b/hostname string h\nd-i c/utc boolean true\n"
        ));
        assert!(detect(b"d-i x string 1\nd-i y select 2\nd-i z note\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo string bar\nbaz string qux\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"d-i a string x\nd-i b boolean true\nanna c select y\nd-i d seen false\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 4);
        assert_eq!(c.strings, 1);
        assert_eq!(c.booleans, 1);
        assert_eq!(c.selects, 1);
        assert_eq!(c.seens, 1);
        assert_eq!(c.owners, 2);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"d-i a string x\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}
