//! Nx `nx.json` workspace census.
//!
//! nx.json is JSON(C): `"targetDefaults"`(ターゲット名キー),
//! `"namedInputs"`(入力名キー), `"generators"`, `"plugins"`,
//! `"tasksRunnerOptions"`, `"affected"`, `"workspaceLayout"`,
//! `"nxCloud"`/`"nxCloudAccessToken"`, `"defaultBase"`,
//! `"tui"`, `"parallel"`, `"captureStderr"`, `"cli"`.
//!
//! ```rust
//! let c = izanagi_kit::nxconf::Nxconf::parse(b"{ \"targetDefaults\": { \"build\": {} }, \"affected\": {} }").unwrap();
//! assert_eq!(c.targets, 1);
//! ```

/// `nx.json` census.
#[derive(Debug, Clone)]
pub struct Nxconf {
    /// Target-name keys inside `"targetDefaults"`.
    pub targets: usize,
    /// Input-name keys inside `"namedInputs"`.
    pub named_inputs: usize,
    /// All `"key":` occurrences.
    pub keys: usize,
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

/// Whether the buffer looks like an nx.json file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for k in [
        "\"targetDefaults\"",
        "\"namedInputs\"",
        "\"generators\"",
        "\"tasksRunnerOptions\"",
        "\"workspaceLayout\"",
        "\"nxCloudAccessToken\"",
        "\"nxCloud\"",
        "\"affected\"",
        "\"plugins\"",
        "nx/schemas",
    ] {
        if t.contains(k) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Nxconf {
    /// Parse an nx.json file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            targets: 0,
            named_inputs: 0,
            keys: 0,
            comments: 0,
        };
        c.comments = t
            .lines()
            .filter(|l| {
                let s = l.trim();
                s.starts_with("//") || s.starts_with("/*")
            })
            .count();
        let tk = toks(t);
        let mut depth = 0usize;
        let mut tgt_ctx: Option<usize> = None;
        let mut inp_ctx: Option<usize> = None;
        let mut i = 0usize;
        while i < tk.len() {
            match &tk[i] {
                Tok::Named(s, e) => {
                    let key = &t[*s..*e];
                    if matches!(tk.get(i + 1), Some(Tok::Chr(b':'))) {
                        c.keys += 1;
                        if tgt_ctx == Some(depth) {
                            c.targets += 1;
                        }
                        if inp_ctx == Some(depth) {
                            c.named_inputs += 1;
                        }
                        if matches!(tk.get(i + 2), Some(Tok::Chr(b'{'))) {
                            if key == "targetDefaults" {
                                tgt_ctx = Some(depth + 1);
                            } else if key == "namedInputs" {
                                inp_ctx = Some(depth + 1);
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
                    if tgt_ctx.is_some_and(|d| depth < d) {
                        tgt_ctx = None;
                    }
                    if inp_ctx.is_some_and(|d| depth < d) {
                        inp_ctx = None;
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
    fn parses_nx_json() {
        let b = concat!(
            "{\n",
            "  \"$schema\": \"./node_modules/nx/schemas/nx-schema.json\",\n",
            "  \"namedInputs\": {\n",
            "    \"default\": [\"{projectRoot}/**/*\"],\n",
            "    \"production\": [\"!{projectRoot}/**/*.spec.ts\"]\n",
            "  },\n",
            "  \"targetDefaults\": {\n",
            "    \"build\": { \"dependsOn\": [\"^build\"], \"cache\": true },\n",
            "    \"test\": { \"inputs\": [\"default\", \"{projectRoot}/jest.config.ts\"] },\n",
            "    \"e2e\": { \"cache\": false }\n",
            "  },\n",
            "  \"affected\": { \"defaultBase\": \"main\" },\n",
            "  \"workspaceLayout\": { \"appsDir\": \"apps\", \"libsDir\": \"libs\" }\n",
            "}\n",
        );
        let c = Nxconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.targets, 3);
        assert_eq!(c.named_inputs, 2);
        assert!(c.keys >= 14);
    }

    #[test]
    fn rejects_other() {
        assert!(Nxconf::parse(b"{ \"foo\": {} }").is_none());
    }
}
