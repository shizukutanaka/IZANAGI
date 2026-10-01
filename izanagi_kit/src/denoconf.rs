//! Deno `deno.json`/`deno.jsonc` census.
//!
//! deno.json is JSONC: `"tasks"`, `"imports"`/`"scopes"`,
//! `"compilerOptions"`, `"lint"`/`"fmt"`/`"test"`/`"bench"`/
//! `"publish"`/`"deploy"`, `"lock"`, `"nodeModulesDir"`,
//! `"vendor"`, `"workspace"`, `"name"`/`"version"`/`"exports"`,
//! `"importMap"`, `"unstable"`, `"exclude"`.
//!
//! ```rust
//! let c = izanagi_kit::denoconf::Denoconf::parse(b"{ \"tasks\": { \"dev\": \"deno run main.ts\" }, \"imports\": {} }").unwrap();
//! assert!(c.options >= 3);
//! ```

/// `deno.json`/`deno.jsonc` census.
#[derive(Debug, Clone)]
pub struct Denoconf {
    /// `"key":` option occurrences.
    pub options: usize,
    /// `true`/`false` option values.
    pub booleans: usize,
    /// `[` array starts.
    pub arrays: usize,
    /// `//`/`/*` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "\"tasks\"",
    "\"imports\"",
    "\"scopes\"",
    "\"lint\"",
    "\"fmt\"",
    "\"test\"",
    "\"bench\"",
    "\"publish\"",
    "\"deploy\"",
    "\"lock\"",
    "\"nodeModulesDir\"",
    "\"vendor\"",
    "\"workspace\"",
    "\"exports\"",
    "\"importMap\"",
    "\"unstable\"",
    "\"exclude\"",
    "\"compilerOptions\"",
];

/// Whether the buffer looks like a deno.json/deno.jsonc file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = KEYS.iter().filter(|k| t.contains(**k)).count();
    hits >= 2 || (hits >= 1 && t.contains("\"tasks\"") && t.contains("\"imports\""))
}

impl Denoconf {
    /// Parse a deno config file into census counts.
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
    fn parses_deno_jsonc() {
        let b = concat!(
            "{\n",
            "  // deno\n",
            "  \"name\": \"@scope/pkg\",\n",
            "  \"version\": \"1.0\",\n",
            "  \"tasks\": {\n",
            "    \"dev\": \"deno run --watch main.ts\",\n",
            "    \"test\": \"deno test -A\"\n",
            "  },\n",
            "  \"imports\": { \"@std/\": \"https://deno.land/std/\" },\n",
            "  \"lint\": { \"rules\": { \"exclude\": [\"no-unused-vars\"] } },\n",
            "  \"fmt\": { \"lineWidth\": 100 },\n",
            "  \"lock\": true,\n",
            "  \"nodeModulesDir\": false,\n",
            "  \"unstable\": [\"kv\"]\n",
            "}\n",
        );
        let c = Denoconf::parse(b.as_bytes()).unwrap();
        assert!(c.options >= 12);
        assert_eq!(c.booleans, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Denoconf::parse(b"{ \"foo\": 1 }").is_none());
    }
}
