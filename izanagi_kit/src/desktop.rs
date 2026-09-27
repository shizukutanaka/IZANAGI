//! freedesktop.org Desktop Entry (`.desktop`) file parsing.
//!
//! INI-style groups (`[Desktop Entry]`), `Key=Value` lines, `#` comments,
//! and locale-suffixed keys `Name[ja_JP]=...`. Values are kept verbatim
//! (percent-escapes like `%f` untouched; `\\s` unescape left to callers).
//!
//! ```
//! use izanagi_kit::desktop;
//! let d = desktop::parse(b"[Desktop Entry]\nName=App\nName[ja]=\xE3\x82\xA2\xE3\x83\x97\xE3\x83\xAA\nExec=a %f\n");
//! assert_eq!(desktop::get(&d, b"Desktop Entry", b"Exec"), Some(b"a %f".as_ref()));
//! ```

use std::vec::Vec;

/// One `Key[locale]=Value` line.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Owning group name (`b"Desktop Entry"`).
    pub group: Vec<u8>,
    /// Key without any locale suffix.
    pub key: Vec<u8>,
    /// Locale tag inside `[...]` (`b"ja"`), if any.
    pub locale: Option<Vec<u8>>,
    /// Raw value bytes.
    pub value: Vec<u8>,
}

/// A parsed `.desktop` file: a flat entry list in file order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Desktop {
    /// All entries across groups, in file order.
    pub entries: Vec<Entry>,
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && (s[a] == b' ' || s[a] == b'\t') {
        a += 1;
    }
    while b > a && (s[b - 1] == b' ' || s[b - 1] == b'\t' || s[b - 1] == b'\r') {
        b -= 1;
    }
    &s[a..b]
}

/// Parses a `.desktop` file. Invalid lines are skipped, never fatal.
pub fn parse(d: &[u8]) -> Desktop {
    let mut entries = Vec::new();
    let mut group: Vec<u8> = Vec::new();
    for raw in d.split(|&b| b == b'\n') {
        let line = trim(raw);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        if line[0] == b'[' && line[line.len() - 1] == b']' {
            group = line[1..line.len() - 1].to_vec();
            continue;
        }
        let eq = match line.iter().position(|&b| b == b'=') {
            Some(e) => e,
            None => continue,
        };
        let key = trim(&line[..eq]);
        if key.is_empty() {
            continue;
        }
        let (key, locale) = match key.iter().position(|&b| b == b'[') {
            Some(p) if key[key.len() - 1] == b']' => {
                (&key[..p], Some(key[p + 1..key.len() - 1].to_vec()))
            }
            _ => (key, None),
        };
        entries.push(Entry {
            group: group.clone(),
            key: key.to_vec(),
            locale,
            value: line[eq + 1..].to_vec(),
        });
    }
    Desktop { entries }
}

/// First unlocalized `group`/`key` value.
pub fn get<'a>(d: &'a Desktop, group: &[u8], key: &[u8]) -> Option<&'a [u8]> {
    d.entries
        .iter()
        .find(|e| e.group == group && e.key == key && e.locale.is_none())
        .map(|e| e.value.as_slice())
}

/// `get`, preferring a `key[locale]` variant when present.
pub fn get_locale<'a>(d: &'a Desktop, group: &[u8], key: &[u8], locale: &[u8]) -> Option<&'a [u8]> {
    d.entries
        .iter()
        .find(|e| e.group == group && e.key == key && e.locale.as_deref() == Some(locale))
        .map(|e| e.value.as_slice())
        .or_else(|| get(d, group, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_and_keys() {
        let d = parse(
            b"[Desktop Entry]\nType=Application\nName=X\n\n[Desktop Action New]\nExec=x -n\n",
        );
        assert_eq!(d.entries.len(), 3);
        assert_eq!(d.entries[2].group, b"Desktop Action New".to_vec());
        assert_eq!(
            get(&d, b"Desktop Entry", b"Type"),
            Some(b"Application".as_ref())
        );
        assert!(get(&d, b"Desktop Entry", b"Nope").is_none());
    }

    #[test]
    fn locale_suffix() {
        let d = parse(b"[Desktop Entry]\nName=App\nName[ja]=x\n");
        assert_eq!(
            get_locale(&d, b"Desktop Entry", b"Name", b"ja"),
            Some(b"x".as_ref())
        );
        assert_eq!(
            get_locale(&d, b"Desktop Entry", b"Name", b"fr"),
            Some(b"App".as_ref())
        );
        assert_eq!(d.entries[1].locale.as_deref(), Some(b"ja".as_ref()));
    }

    #[test]
    fn skips_junk() {
        let d = parse(b"# comment\nno-equals\n=emptykey\n[]\nK=V\n");
        assert_eq!(d.entries.len(), 1);
        assert_eq!(d.entries[0].key, b"K".to_vec());
    }
}
