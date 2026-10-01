//! Lerna `lerna.json` census.
//!
//! lerna.json is JSON(C): `"version"`(`fixed`/`independent`),
//! `"packages"`/`"npmClient"`/`"command.X"` サブコマンド設定
//! (`"command.init"`/`"command.bootstrap"`/`"command.publish"`…),
//! `"useWorkspaces"`/`"useNx"`/`"registry"`/`"ignoreChanges"`/
//! `"loglevel"`/`"concurrency"`/`"$schema"`.
//!
//! ```rust
//! let c = izanagi_kit::lerna::Lerna::parse(b"{ \"version\": \"independent\", \"packages\": [\"pkgs/*\"], \"command.init\": {} }").unwrap();
//! assert_eq!(c.commands, 1);
//! ```

/// `lerna.json` census.
#[derive(Debug, Clone)]
pub struct Lerna {
    /// `"key":` option occurrences.
    pub settings: usize,
    /// `"command.*"` keys.
    pub commands: usize,
    /// `[` array starts.
    pub arrays: usize,
    /// `//`/`/*` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a lerna.json file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for k in [
        "\"packages\"",
        "\"command.",
        "\"npmClient\"",
        "\"useWorkspaces\"",
        "\"useNx\"",
        "\"ignoreChanges\"",
        "\"loglevel\"",
        "\"concurrency\"",
        "\"version\"",
        "lerna.json",
    ] {
        if t.contains(k) {
            hits += 1;
        }
    }
    hits >= 2 && (t.contains("\"command.") || t.contains("\"packages\"") || t.contains("lerna"))
}

impl Lerna {
    /// Parse a lerna.json file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            commands: 0,
            arrays: 0,
            comments: 0,
        };
        let b = t.as_bytes();
        let mut i = 0usize;
        while i < b.len() {
            match b[i] {
                b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                    c.comments += 1;
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                }
                b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                    c.comments += 1;
                    i += 2;
                    while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                        i += 1;
                    }
                    i = (i + 2).min(b.len());
                }
                b'[' => {
                    c.arrays += 1;
                    i += 1;
                }
                b'"' => {
                    let start = i + 1;
                    i += 1;
                    while i < b.len() && b[i] != b'"' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    let key = &t[start..i];
                    i += 1;
                    let mut j = i;
                    while j < b.len() && b[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < b.len() && b[j] == b':' {
                        c.settings += 1;
                        if key.starts_with("command.") {
                            c.commands += 1;
                        }
                    }
                }
                _ => i += 1,
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lerna() {
        let b = concat!(
            "{\n",
            "  // lerna\n",
            "  \"$schema\": \"node_modules/lerna/schemas/lerna-schema.json\",\n",
            "  \"version\": \"independent\",\n",
            "  \"npmClient\": \"pnpm\",\n",
            "  \"packages\": [\"pkgs/*\"],\n",
            "  \"command.publish\": { \"ignoreChanges\": [\"*.md\"] },\n",
            "  \"command.bootstrap\": { \"concurrency\": 4 },\n",
            "  \"loglevel\": \"verbose\"\n",
            "}\n",
        );
        let c = Lerna::parse(b.as_bytes()).unwrap();
        assert_eq!(c.commands, 2);
        assert_eq!(c.arrays, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Lerna::parse(b"{ \"foo\": {} }").is_none());
    }
}
