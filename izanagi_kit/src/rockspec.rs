//! LuaRocks `.rockspec` — `package = "…"`, `version`, `source`,
//! `description`, `dependencies` and `build` table census.
//!
//! ```
//! let d = b"rockspec_format = \"3\x2e0\"\npackage = \"demo\"\nversion = \"1\x2e0-1\"\nsource = {\n   url = \"git://example\x2ecom/demo\",\n}\ndescription = {\n   summary = \"demo rock\",\n   license = \"MIT\",\n}\ndependencies = {\n   \"lua >= 5\x2e1\",\n   \"lpeg\",\n}\nbuild = {\n   type = \"builtin\",\n   modules = { demo = \"src/demo\x2elua\" },\n}\n";
//! let r = izanagi_kit::rockspec::parse(d).unwrap();
//! assert_eq!(r.package, "demo");
//! assert_eq!(r.dependencies, 2);
//! assert_eq!(r.build_type, "builtin");
//! assert!(izanagi_kit::rockspec::detect(d));
//! ```

/// A parsed `.rockspec` summary.
#[derive(Debug, Clone)]
pub struct Rockspec {
    /// `package = "…"`.
    pub package: String,
    /// `version = "…"` (`<semver>-<revision>`).
    pub version: String,
    /// `rockspec_format` value (`3.0`/`1.1`/`1.0`).
    pub rockspec_format: String,
    /// `source.url` / `source.file` value.
    pub source_url: String,
    /// `source.tag`/`branch`/`commit`/`tag_or_branch` keys present.
    pub source_vcs_keys: usize,
    /// `description.summary`/`homepage`/`license`/`maintainer` keys present.
    pub description_keys: usize,
    /// Entries in `dependencies = { … }` (quoted strings).
    pub dependencies: usize,
    /// Entries in `test_dependencies = { … }`.
    pub test_dependencies: usize,
    /// `build.type` (`builtin`/`make`/`cmake`/`command`/`none`).
    pub build_type: String,
    /// Entries in `build.modules`/`build.install` tables.
    pub build_entries: usize,
    /// `supported_platforms = { … }` entries.
    pub supported_platforms: usize,
    /// `test = { … }`/`test_dependencies`/`checks` markers present.
    pub test_keys: usize,
}

/// First `key = "value"` / `key = 'value'` scalar at top level.
fn scalar<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for line in t.lines() {
        let l = line.trim_start();
        if let Some(rest) = l.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                let v = v.trim();
                let v = v.trim_end_matches(',').trim_matches('"').trim_matches('\'');
                return Some(v);
            }
        }
    }
    None
}

/// Contents of a `key = { … }` block (may span lines until matching `}`).
/// The key must sit on a word boundary and be followed directly by `=`.
fn table_block<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let mut off = 0usize;
    let rest;
    loop {
        let p = off + t[off..].find(key)?;
        let left_ok =
            p == 0 || !(t.as_bytes()[p - 1].is_ascii_alphanumeric() || t.as_bytes()[p - 1] == b'_');
        let after = t[p + key.len()..].trim_start();
        if left_ok && after.starts_with('=') {
            rest = &after[1..];
            break;
        }
        off = p + key.len();
    }
    let brace = rest.find('{')?;
    let inner = &rest[brace + 1..];
    let mut depth = 1usize;
    for (i, c) in inner.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&inner[..i]);
                }
            }
            _ => {}
        }
    }
    Some(inner)
}

/// Count `"…"`/`'…'` entries inside a table body.
fn quoted_entries(s: &str) -> usize {
    let mut n = 0;
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'"' || b[i] == b'\'' {
            let q = b[i];
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != q {
                j += 1;
            }
            if j < b.len() && j > start {
                n += 1;
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    n
}

/// Count `word = …` assignments inside a table body; multiple assignments
/// may share one line and comparison operators are not assignments.
fn assign_entries(s: &str) -> usize {
    let b = s.as_bytes();
    let mut n = 0;
    for (i, &byte) in b.iter().enumerate() {
        if byte != b'=' {
            continue;
        }
        // `==`, `>=`, `<=`, `~=` and the like are not assignments.
        if b.get(i + 1) == Some(&b'=') {
            continue;
        }
        let mut j = i;
        while j > 0 && b[j - 1].is_ascii_whitespace() {
            j -= 1;
        }
        let prev_ok = j > 0
            && (b[j - 1].is_ascii_alphanumeric() || matches!(b[j - 1], b'_' | b'"' | b'\'' | b'}'));
        if prev_ok {
            n += 1;
        }
    }
    n
}

fn has_word(t: &str, w: &str) -> bool {
    t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|p| p == w)
}

/// Detects a rockspec: `package =` plus `source`/`dependencies`/`rockspec_format`/`build` tables.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    scalar(t, "package").is_some()
        && (has_word(t, "rockspec_format")
            || table_block(t, "source").is_some()
            || table_block(t, "build").is_some())
}

/// Parses a `.rockspec`; `None` without `package =` + rockspec tables.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Rockspec> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let src = table_block(t, "source").unwrap_or("");
    let desc = table_block(t, "description").unwrap_or("");
    let deps = table_block(t, "dependencies").unwrap_or("");
    let tdeps = table_block(t, "test_dependencies").unwrap_or("");
    let build = table_block(t, "build").unwrap_or("");
    let mut vcs = 0;
    for k in [
        "tag",
        "branch",
        "commit",
        "tag_or_branch",
        "dir_name",
        "dir",
    ] {
        if scalar(src, k).is_some() {
            vcs += 1;
        }
    }
    let mut dkeys = 0;
    for k in [
        "summary",
        "detailed",
        "homepage",
        "license",
        "maintainer",
        "labels",
    ] {
        if scalar(desc, k).is_some() {
            dkeys += 1;
        }
    }
    Some(Rockspec {
        package: scalar(t, "package").unwrap_or("").to_string(),
        version: scalar(t, "version").unwrap_or("").to_string(),
        rockspec_format: scalar(t, "rockspec_format").unwrap_or("").to_string(),
        source_url: scalar(src, "url")
            .or_else(|| scalar(src, "file"))
            .unwrap_or("")
            .to_string(),
        source_vcs_keys: vcs,
        description_keys: dkeys,
        dependencies: quoted_entries(deps),
        test_dependencies: quoted_entries(tdeps),
        build_type: scalar(build, "type").unwrap_or("").to_string(),
        build_entries: assign_entries(build)
            .saturating_sub(usize::from(scalar(build, "type").is_some())),
        supported_platforms: table_block(t, "supported_platforms")
            .map(quoted_entries)
            .unwrap_or(0),
        test_keys: usize::from(table_block(t, "test").is_some())
            + usize::from(table_block(t, "checks").is_some()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"rockspec_format = \"3\x2e0\"\npackage = \"demo\"\nversion = \"1\x2e0-1\"\nsource = {\n   url = \"git://example\x2ecom/demo\",\n   tag = \"v1\x2e0\",\n}\ndescription = {\n   summary = \"demo rock\",\n   homepage = \"https://example\x2ecom\",\n   license = \"MIT\",\n}\ndependencies = {\n   \"lua >= 5\x2e1\",\n   \"lpeg\",\n   \"luasocket >= 2\x2e0\",\n}\nbuild = {\n   type = \"builtin\",\n   modules = { demo = \"src/demo\x2elua\", demo_sub = \"src/demo/sub\x2elua\" },\n}\n";

    #[test]
    fn parses() {
        let r = parse(D).unwrap();
        assert_eq!(r.package, "demo");
        assert_eq!(r.version, "1\x2e0-1");
        assert_eq!(r.rockspec_format, "3\x2e0");
        assert!(r.source_url.starts_with("git://"));
        assert_eq!(r.source_vcs_keys, 1);
        assert_eq!(r.description_keys, 3);
        assert_eq!(r.dependencies, 3);
        assert_eq!(r.build_type, "builtin");
        assert!(r.build_entries >= 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"package = \"x\"\nsource = { url = \"u\" }\n"));
        assert!(!detect(b"package x\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key = \"value\"\n").is_none());
    }
}
