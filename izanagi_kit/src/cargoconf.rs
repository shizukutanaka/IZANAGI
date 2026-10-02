//! Cargo 設定ファイル(`.cargo/config.toml`)の検出と構造カウント。
//!
//! `[build]`/`[target.*]`/`[registry]`/`[registries.*]`/`[source.*]`/`[net]`/
//! `[alias]`/`[cargo-new]`/`[http]`/`[install]`/`[term]`/`[future-incompat-report]`
//! 既知テーブルと `jobs`/`rustflags`/`runner`/`linker`/`token`/`offline`/
//! `credential-provider` 等の既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::cargoconf::parse(
//!     b"[build]\njobs = 4\n\n[target.x86_64-unknown-linux-gnu]\nlinker = \"clang\"\n").unwrap();
//! assert_eq!(c.tables, 2);
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::cargoconf::detect(b"[net]\noffline = true\n[term]\nquiet = false\n"));
//! ```

/// 既知テーブル名(接頭辞一致; `target.`/`source.`/`registries.`/`credential-alias.`)。
const TABLE_PREFIXES: &[&str] = &[
    "alias",
    "build",
    "cache-",
    "cargo-new",
    "credential-alias",
    "env",
    "future-incompat-report",
    "http",
    "install",
    "net",
    "patch",
    "registries",
    "registry",
    "resolver",
    "source",
    "target",
    "term",
    "unstable",
];

/// 既知キー。
const KEYS: &[&str] = &[
    "build-std",
    "build-std-features",
    "cainfo",
    "check-revoke",
    "color",
    "credential-process",
    "credential-provider",
    "debug",
    "debug-fission",
    "default",
    "dep-info-basedir",
    "error-format",
    "features",
    "git-fetch-with-cli",
    "gitoxide",
    "incremental",
    "jobs",
    "lfs",
    "linker",
    "message-format",
    "multiplexing",
    "offline",
    "proxy",
    "quiet",
    "registry",
    "replace",
    "retry",
    "rustdoc",
    "rustdocflags",
    "rustc",
    "rustc-wrapper",
    "rustc-workspace-wrapper",
    "rustflags",
    "runner",
    "ssl-version",
    "strip",
    "target",
    "term-progress",
    "timeout",
    "token",
    "toolchain",
    "vendor",
    "vendored-sources",
    "verbose",
];

/// ユーザ定義キーを持つテーブル(内部の `key = value` は既知問わず options)。
const FREE_TABLES: &[&str] = &[
    "alias",
    "credential-alias",
    "env",
    "patch",
    "registries",
    "source",
];

/// cargoconf 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[table]`/`[[table]]` 行(既知名)。
    pub tables: usize,
    /// `key = value` 既知代入。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 未知キー・テーブル・分類不能行。
    pub misc: usize,
}

/// `[name]`/`[[name]]` テーブル名。
fn table_name(t: &str) -> Option<&str> {
    let t = t.strip_prefix('[')?;
    let t = t.strip_prefix('[').unwrap_or(t);
    let t = t.strip_suffix(']')?;
    let t = t.strip_suffix(']').unwrap_or(t);
    Some(t)
}

/// `key = value` のキー部分。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find('=')?;
    let k = t[..p].trim().trim_matches('"').trim_matches('\'');
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}

/// テーブル名が既知プレフィックスに合うか。
fn known_table(t: &str) -> bool {
    let root = t.split('.').next().unwrap_or("");
    TABLE_PREFIXES.iter().any(|p| {
        t == *p
            || (p.ends_with('-') && t.starts_with(*p))
            || t.starts_with(*p) && t.as_bytes().get(p.len()) == Some(&b'.')
    }) || TABLE_PREFIXES.contains(&root)
}

/// b が .cargo/config.toml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if table_name(t).is_some_and(known_table) {
            hits += 2;
        } else if kv_key(t).is_some_and(|k| KEYS.contains(&k)) {
            hits += 1;
        }
    }
    hits >= 3
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        tables: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut free_keys = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if let Some(name) = table_name(t) {
            free_keys = FREE_TABLES
                .iter()
                .any(|r| name.split('.').next() == Some(*r));
            if known_table(name) {
                c.tables += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if let Some(k) = kv_key(t) {
            if free_keys || KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.tables + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# cargo config\n[build]\njobs = 4\nincremental = false\nrustflags = [\"-L\", \"/lib\"]\n\n[target.x86_64-unknown-linux-gnu]\nlinker = \"clang\"\nrunner = \"qemu-x86_64\"\n\n[target.'cfg(unix)']\nrunner = \"sh -c\"\n\n[net]\nretry = 2\noffline = false\n\n[alias]\nb = \"build\"\n";

    #[test]
    fn cargoconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tables, 5);
        assert_eq!(c.options, 9);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_cargoconf() {
        assert!(!detect(b"[package]\nname = \"x\"\nversion = \"1\"\n"));
        assert!(!detect(b"foo = 1\n"));
    }
}
