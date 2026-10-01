//! TypeScript `tsconfig.json`/`jsconfig.json` census.
//!
//! tsconfig is JSONC: `"compilerOptions"`(~150 キー), `"extends"`,
//! `"include"`/`"exclude"`/`"files"`, `"references"`,
//! `"watchOptions"`, `"typeAcquisition"`, `"ts-node"`,
//! `"display"`/`"compileOnSave"`. `//`・`/* */` コメントを許容。
//!
//! ```rust
//! let c = izanagi_kit::tsconfig::Tsconfig::parse(b"{ \"compilerOptions\": { \"strict\": true, \"target\": \"es2022\" } }").unwrap();
//! assert_eq!(c.options, 3);
//! ```

/// `tsconfig.json` census.
#[derive(Debug, Clone)]
pub struct Tsconfig {
    /// `"key":` option occurrences.
    pub options: usize,
    /// `true`/`false` option values.
    pub booleans: usize,
    /// `[` array starts.
    pub arrays: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "\"compilerOptions\"",
    "\"watchOptions\"",
    "\"typeAcquisition\"",
    "\"ts-node\"",
    "\"display\"",
    "\"extends\"",
    "\"include\"",
    "\"exclude\"",
    "\"files\"",
    "\"references\"",
    "\"compileOnSave\"",
];

/// Whether the buffer looks like a tsconfig/jsconfig file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if t.contains("\"compilerOptions\"") {
        return true;
    }
    KEYS[5..].iter().filter(|k| t.contains(**k)).count() >= 2
}

impl Tsconfig {
    /// Parse a tsconfig file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            options: 0,
            booleans: 0,
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
                    i += 1;
                    while i < b.len() && b[i] != b'"' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    i += 1;
                    let mut j = i;
                    while j < b.len() && b[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < b.len() && b[j] == b':' {
                        c.options += 1;
                        j += 1;
                        while j < b.len() && b[j].is_ascii_whitespace() {
                            j += 1;
                        }
                        if t[j..].starts_with("true") || t[j..].starts_with("false") {
                            c.booleans += 1;
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
    fn parses_tsconfig() {
        let b = concat!(
            "{\n",
            "  // base\n",
            "  \"extends\": \"./base.json\",\n",
            "  \"compilerOptions\": {\n",
            "    \"target\": \"es2022\",\n",
            "    \"module\": \"esnext\",\n",
            "    \"strict\": true,\n",
            "    \"noEmit\": false,\n",
            "    \"lib\": [\"es2022\", \"dom\"],\n",
            "    \"paths\": { \"@app/*\": [\"src/*\"] }\n",
            "  },\n",
            "  \"include\": [\"src/**/*\"],\n",
            "  \"exclude\": [\"node_modules\"],\n",
            "  \"references\": [{ \"path\": \"../shared\" }]\n",
            "}\n",
        );
        let c = Tsconfig::parse(b.as_bytes()).unwrap();
        assert!(c.options >= 10);
        assert_eq!(c.booleans, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Tsconfig::parse(b"{ \"foo\": 1 }").is_none());
    }
}
