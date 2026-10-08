//! Vercel `vercel.json` プロジェクト設定の認識と計数。
//!
//! `vercel.json` は JSON で、`version`(=2)、`name`、`builds`(`src`/`use`/
//! `config` を持つエントリ)、`routes`/`rewrites`/`redirects`/`headers`/
//! `trailingSlash`/`cleanUrls`/`buildCommand`/`outputDirectory`/`installCommand`/
//! `devCommand`/`framework`/`functions`/`regions`/`env`/`build.env`/`public`/
//! `github`/`alias` 等のキーを持つ。JSONC スタイルの `//` コメントも
//! 許容して走査する。
//!
//! ```
//! let b = b"{\n  \"version\": 2,\n  \"builds\": [\n    { \"src\": \"package.json\", \"use\": \"@vercel/next\" },\n    { \"src\": \"api/*.go\", \"use\": \"@vercel/go\" }\n  ],\n  \"routes\": [\n    { \"src\": \"/old\", \"dest\": \"/new\" }\n  ],\n  \"rewrites\": [\n    { \"source\": \"/a\", \"destination\": \"/b\" }\n  ],\n  \"functions\": {\n    \"api/test.js\": { \"maxDuration\": 10 }\n  },\n  \"regions\": [\"hnd1\"],\n  \"env\": {\n    \"MY_KEY\": \"x\"\n  }\n}\n";
//! assert!(izanagi_kit::vercelconf::detect(b));
//! let c = izanagi_kit::vercelconf::parse(b).unwrap();
//! assert_eq!(c.keys, 18); // version/builds/src×2/use×2/routes/src/dest/rewrites/source/destination/functions/api\/*\/maxDuration/regions/env/MY_KEY
//! assert_eq!(c.builds, 2);
//! assert_eq!(c.routes, 2); // routes + rewrites entries
//! assert_eq!(c.sections, 2); // functions, env
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"key":` の個数。
    pub keys: usize,
    /// `builds` 配列内の `{` エントリ個数(`src`/`use` を持つ要素)。
    pub builds: usize,
    /// `routes`/`rewrites`/`redirects`/`headers`/`files`/`trailingSlash` 系
    /// 配列内のエントリ個数。
    pub routes: usize,
    /// `functions`/`env`/`build.env`/`github`/`images`/`projectSettings` 等
    /// オブジェクト値を持つトップレベルキーの個数。
    pub sections: usize,
    /// `true`/`false` 値の個数。
    pub bools: usize,
    /// `//`/`/* */` コメント行の個数。
    pub comments: usize,
}

const OBJECT_KEYS: &[&str] = &[
    "functions",
    "env",
    "build",
    "github",
    "images",
    "projectSettings",
    "crons",
    "wildcards",
];

const ROUTE_ARRAYS: &[&str] = &["routes", "rewrites", "redirects", "headers", "files"];

/// `vercel.json` らしさを返す。`"version": 2` や `"builds"`/`"rewrites"` 等
/// キーの組合せで判定。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        for key in [
            "\"version\": 2",
            "\"builds\"",
            "\"rewrites\"",
            "\"routes\"",
            "\"vercel",
            "\"now\"",
            "\"buildCommand\"",
            "\"outputDirectory\"",
            "\"framework\"",
            "\"functions\"",
        ] {
            if s.contains(key) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// 全テキストから `"key":` を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        keys: 0,
        builds: 0,
        routes: 0,
        sections: 0,
        bools: 0,
        comments: 0,
    };
    let mut in_builds = false;
    let mut in_routes = false;
    let mut depth: i64 = 0;
    let mut route_depth: i64 = -1;
    let mut builds_depth: i64 = -1;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with("//") || s.starts_with("/*") {
            c.comments += 1;
            continue;
        }
        // `"key": value` あるいは `"key":` の走査 — 各行で出現するすべての
        // クォートキーを数える。
        let bytes = s.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] == b'"' {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != b'"' && bytes[j] != b'\n' {
                    j += 1;
                }
                if j < bytes.len() {
                    let after = s[j + 1..].trim_start();
                    if let Some(val0) = after.strip_prefix(':') {
                        let key = &s[i + 1..j];
                        c.keys += 1;
                        if OBJECT_KEYS.contains(&key) {
                            c.sections += 1;
                        }
                        if ROUTE_ARRAYS.contains(&key) {
                            in_routes = true;
                            route_depth = depth;
                        }
                        if key == "builds" {
                            in_builds = true;
                            builds_depth = depth;
                        }
                        let val = val0.trim_start();
                        if val.starts_with("true") || val.starts_with("false") {
                            c.bools += 1;
                        }
                    }
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
        for ch in s.chars() {
            match ch {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
        }
        if in_builds && depth <= builds_depth {
            in_builds = false;
            builds_depth = -1;
        }
        if in_routes && depth <= route_depth {
            in_routes = false;
            route_depth = -1;
        }
        if in_builds && s.contains('{') {
            c.builds += 1;
        }
        if in_routes && s.contains('{') {
            c.routes += 1;
        }
    }
    if !t.trim().is_empty() && c.keys + c.builds + c.routes + c.sections + c.bools + c.comments == 0
    {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_vercel() {
        assert!(detect(
            b"{\"version\": 2, \"builds\": [{\"src\": \"a\", \"use\": \"b\"}]}"
        ));
        assert!(detect(b"{\n  \"version\": 2,\n  \"rewrites\": []\n}\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"{\"name\": \"x\", \"private\": true}"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"{\n  \"version\": 2,\n  \"builds\": [\n    { \"src\": \"package.json\", \"use\": \"@vercel/next\" }\n  ],\n  \"rewrites\": [\n    { \"source\": \"/a\", \"destination\": \"/b\" }\n  ],\n  \"env\": { \"X\": \"1\" }\n}\n";
        let c = parse(b).unwrap();
        assert_eq!(c.keys, 9);
        assert_eq!(c.builds, 1);
        assert_eq!(c.routes, 1);
        assert_eq!(c.sections, 1); // env
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"{\"version\": 2, \"builds\": []}").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a config file at all\n").is_none());
    }
}
