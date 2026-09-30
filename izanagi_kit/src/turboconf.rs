//! Turborepo `turbo.json` census.
//!
//! turbo.json is JSON(C): `"pipeline"`/`"tasks"` ブロック内の
//! タスク名キー、`"dependsOn"`/`"outputs"`/`"inputs"`/`"env"`/
//! `"globalEnv"`/`"globalDependencies"`/`"globalPassThroughEnv"`/
//! `"cache"`/`"daemon"`/`"ui"`/`"$schema"`/`"remoteCache"`.
//!
//! ```rust
//! let c = izanagi_kit::turboconf::Turboconf::parse(b"{ \"tasks\": { \"build\": { \"dependsOn\": [\"^build\"] } } }").unwrap();
//! assert_eq!(c.tasks, 1);
//! ```

/// `turbo.json` census.
#[derive(Debug, Clone)]
pub struct Turboconf {
    /// Task-name keys inside `"pipeline"`/`"tasks"`.
    pub tasks: usize,
    /// All `"key":` occurrences.
    pub keys: usize,
    /// `[` array starts.
    pub arrays: usize,
    /// `//`/`/*` comments.
    pub comments: usize,
}

enum Tok {
    Chr(u8),
    Named(usize, usize),
}

fn toks(t: &str) -> Vec<Tok> {
    let b = t.as_bytes();
    let mut v = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(b.len());
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
                v.push(Tok::Named(start, i));
                i += 1;
            }
            b'{' | b'}' | b'[' | b']' | b':' => {
                v.push(Tok::Chr(b[i]));
                i += 1;
            }
            _ => i += 1,
        }
    }
    v
}

/// Whether the buffer looks like a turbo.json file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"pipeline\"") || t.contains("\"tasks\""))
        && (t.contains("\"dependsOn\"")
            || t.contains("\"outputs\"")
            || t.contains("\"env\"")
            || t.contains("\"globalEnv\"")
            || t.contains("\"cache\"")
            || t.contains("turbo.build")
            || t.contains("turbo\"")
            || t.contains("\"remoteCache\"")
            || t.contains("\"ui\""))
}

impl Turboconf {
    /// Parse a turbo.json file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            tasks: 0,
            keys: 0,
            arrays: 0,
            comments: 0,
        };
        c.comments = t
            .lines()
            .filter(|l| {
                let s = l.trim();
                s.starts_with("//") || s.starts_with("/*")
            })
            .count();
        c.arrays = t.matches('[').count();
        let tk = toks(t);
        let mut depth = 0usize;
        let mut task_ctx: Option<usize> = None;
        let mut i = 0usize;
        while i < tk.len() {
            match &tk[i] {
                Tok::Named(s, e) => {
                    let key = &t[*s..*e];
                    if matches!(tk.get(i + 1), Some(Tok::Chr(b':'))) {
                        c.keys += 1;
                        if matches!(tk.get(i + 2), Some(Tok::Chr(b'{'))) {
                            if key == "pipeline" || key == "tasks" {
                                task_ctx = Some(depth + 1);
                            } else if task_ctx == Some(depth) {
                                c.tasks += 1;
                            }
                        }
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
                Tok::Chr(b'{') | Tok::Chr(b'[') => {
                    depth += 1;
                    i += 1;
                }
                Tok::Chr(b'}') | Tok::Chr(b']') => {
                    depth = depth.saturating_sub(1);
                    if task_ctx.is_some_and(|d| depth < d) {
                        task_ctx = None;
                    }
                    i += 1;
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
    fn parses_turbo_json() {
        let b = concat!(
            "{\n",
            "  \"$schema\": \"https://turbo.build/schema.json\",\n",
            "  \"ui\": \"tui\",\n",
            "  \"tasks\": {\n",
            "    \"build\": {\n",
            "      \"dependsOn\": [\"^build\"],\n",
            "      \"outputs\": [\"dist/**\"],\n",
            "      \"inputs\": [\"src/**\"]\n",
            "    },\n",
            "    \"test\": {\n",
            "      \"dependsOn\": [\"build\"],\n",
            "      \"cache\": false\n",
            "    },\n",
            "    \"lint\": {}\n",
            "  },\n",
            "  \"globalEnv\": [\"CI\"],\n",
            "  \"globalDependencies\": [\".env\"]\n",
            "}\n",
        );
        let c = Turboconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.tasks, 3);
        assert!(c.keys >= 12);
        assert_eq!(c.arrays, 6);
    }

    #[test]
    fn rejects_other() {
        assert!(Turboconf::parse(b"{ \"foo\": {} }").is_none());
    }
}
