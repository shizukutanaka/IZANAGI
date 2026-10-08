//! Zephyr `west.yml` west マニフェストの認識と計数。
//!
//! `west.yml` は west(Zephyr マルチリポジトリツール)のマニフェストで、
//! `manifest:` 直下に `remotes:`(`- name:`/`url-base:`)、`projects:`(`- name:`/
//! `remote:`/`repo-path:`/`url:`/`revision:`/`path:`/`west-commands:`/
//! `import:`/`groups:`/`submodules:`)、`self:`(`path:`/`west-commands:`/
//! `import:`)、`group-filter:`、`defaults:`(`remote:`/`revision:`) を持つ。
//!
//! ```
//! let b = b"manifest:\n    defaults:\n        remote: upstream\n        revision: v4.0.0\n    remotes:\n        - name: upstream\n          url-base: https://github.com/zephyrproject-rtos\n        - name: nordic\n          url-base: https://github.com/nrfconnect\n    projects:\n        - name: zephyr\n          remote: upstream\n          repo-path: zephyr\n          import: true\n        - name: hal_nordic\n          remote: nordic\n          path: modules/hal/nordic\n        - name: cmsis\n          remote: upstream\n          groups:\n            - external\n    self:\n        path: application\n";
//! assert!(izanagi_kit::westconf::detect(b));
//! let c = izanagi_kit::westconf::parse(b).unwrap();
//! assert_eq!(c.remotes, 2);
//! assert_eq!(c.projects, 3);
//! assert_eq!(c.sections, 5); // manifest/defaults/remotes/projects/self
//! assert_eq!(c.keys, 22);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key:`/`key: value` 行の個数(`- key:` を含む)。
    pub keys: usize,
    /// `manifest:`/`defaults:`/`remotes:`/`projects:`/`self:`/`group-filter:`/
    /// `west-commands:`/`import:`/`applications:` 構造キーの個数。
    pub sections: usize,
    /// `remotes:` 配下の `- name:` 項目の個数。
    pub remotes: usize,
    /// `projects:` 配下の `- name:` 項目の個数。
    pub projects: usize,
    /// `url:`/`url-base:`/`repo-path:`/`remote:`/`revision:`/`path:`/
    /// `west-commands:`/`import:`/`groups:`/`submodules:`/`limit:`/
    /// `clone-depth:`/`line-endings:`/`userdata:`/`qualifiers:` 属性キーの個数。
    pub attrs: usize,
    /// `-` リスト項目行の個数。
    pub list_items: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const SECTION_KEYS: &[&str] = &[
    "manifest",
    "defaults",
    "remotes",
    "projects",
    "self",
    "group-filter",
    "west-commands",
    "applications",
];

const ATTR_KEYS: &[&str] = &[
    "url",
    "url-base",
    "repo-path",
    "remote",
    "revision",
    "path",
    "west-commands",
    "import",
    "groups",
    "submodules",
    "limit",
    "clone-depth",
    "line-endings",
    "userdata",
    "qualifiers",
    "whitelist",
    "blacklist",
    "name",
    "version",
    "description",
    "classification",
    "uuid",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `west.yml` らしさを返す。`manifest:` と `remotes:`/`projects:` の組合せ。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        for key in [
            "manifest:",
            "remotes:",
            "projects:",
            "url-base:",
            "repo-path:",
            "west-commands:",
            "self:",
            "group-filter:",
        ] {
            if s.starts_with(key)
                || s.starts_with(&format!("- {key}"))
                || s.contains(&format!(" {key}"))
            {
                hits += 1;
            }
        }
    }
    hits >= 3
}

fn line_key(s: &str) -> Option<&str> {
    let t = s.strip_prefix('-').map_or(s, |r| r.trim_start());
    let colon = t.find(':')?;
    let k = t[..colon]
        .trim_end()
        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
        .next()
        .unwrap_or("");
    if k.is_empty() {
        None
    } else {
        Some(k)
    }
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        keys: 0,
        sections: 0,
        remotes: 0,
        projects: 0,
        attrs: 0,
        list_items: 0,
        comments: 0,
    };
    // 0=なし, 1=remotes, 2=projects
    let mut scope = 0u8;
    let mut scope_indent: i64 = -1;
    for l in t.lines() {
        if l.trim().is_empty() {
            continue;
        }
        let ind = (l.len() - l.trim_start().len()) as i64;
        let s = l.trim();
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if scope_indent >= 0 && ind <= scope_indent && !s.starts_with('-') {
            scope = 0;
            scope_indent = -1;
        }
        let is_dash = s.starts_with('-');
        if is_dash {
            c.list_items += 1;
        }
        let Some(key) = line_key(s) else {
            continue;
        };
        c.keys += 1;
        if SECTION_KEYS.contains(&key) {
            c.sections += 1;
        }
        match key {
            "remotes" => {
                scope = 1;
                scope_indent = ind;
            }
            "projects" => {
                scope = 2;
                scope_indent = ind;
            }
            "name" => {
                if is_dash && scope == 1 {
                    c.remotes += 1;
                } else if is_dash && scope == 2 {
                    c.projects += 1;
                }
            }
            _ => {}
        }
        if ATTR_KEYS.contains(&key) {
            c.attrs += 1;
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
            b"manifest:\n    remotes:\n        - name: a\n          url-base: x\n    projects:\n        - name: z\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"name: x\nkind: y\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"manifest:\n    remotes:\n        - name: a\n          url-base: x\n        - name: b\n          url-base: y\n    projects:\n        - name: z\n          remote: a\n        - name: w\n          repo-path: w\n    self:\n        path: app\n";
        let c = parse(b).unwrap();
        assert_eq!(c.remotes, 2);
        assert_eq!(c.projects, 2);
        assert_eq!(c.sections, 4); // manifest/remotes/projects/self
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"manifest:\n    remotes:\n        - name: a\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
