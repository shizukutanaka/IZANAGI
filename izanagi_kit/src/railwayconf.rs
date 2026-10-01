//! Railway `railway.json`/`railway.toml` デプロイ設定の認識と計数。
//!
//! Railway の設定ファイルは JSON または TOML で、`build`(builder/
//! buildCommand/dockerfilePath/dockerContext/watchPatterns)、`deploy`
//! (startCommand/healthcheckPath/healthcheckTimeout/restartPolicyType/
//! restartPolicyMaxRetries/numReplicas/preDeployCommand/sleepApplication/
//! region)、`environments`、`variables`、`cron` 等を持つ。JSON の
//! `"build": {…}` / `"deploy": {…}` と TOML の `[build]`/`[deploy]` の
//! 両方を走査する。
//!
//! ```
//! let b = b"{\n  \"$schema\": \"https://railway.app/railway.schema.json\",\n  \"build\": {\n    \"builder\": \"NIXPACKS\",\n    \"buildCommand\": \"npm run build\"\n  },\n  \"deploy\": {\n    \"startCommand\": \"npm start\",\n    \"healthcheckPath\": \"/health\",\n    \"restartPolicyType\": \"ON_FAILURE\",\n    \"restartPolicyMaxRetries\": 10\n  }\n}\n";
//! assert!(izanagi_kit::railwayconf::detect(b));
//! let c = izanagi_kit::railwayconf::parse(b).unwrap();
//! assert_eq!(c.keys, 9);
//! assert_eq!(c.sections, 2); // build, deploy
//! assert_eq!(c.build_keys, 2); // builder, buildCommand
//! assert_eq!(c.deploy_keys, 4); // startCommand/healthcheckPath/restartPolicyType/restartPolicyMaxRetries
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"key":` または `key =`/`key:` 行の個数。
    pub keys: usize,
    /// `build`/`deploy`/`environments`/`variables`/`cron`/`source`/`icons`
    /// セクションの個数。
    pub sections: usize,
    /// `build` セクション内のキー個数。
    pub build_keys: usize,
    /// `deploy` セクション内のキー個数。
    pub deploy_keys: usize,
    /// `healthcheck*`/`restartPolicy*`/`preDeployCommand`/`sleepApplication`/
    /// `numReplicas`/`region` 系デプロイ制御キーの個数。
    pub deploy_controls: usize,
    /// `#`/`//` コメント行の個数。
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "build",
    "deploy",
    "environments",
    "variables",
    "cron",
    "source",
    "icons",
];

const DEPLOY_CONTROLS: &[&str] = &[
    "healthcheckPath",
    "healthcheckTimeout",
    "restartPolicyType",
    "restartPolicyMaxRetries",
    "preDeployCommand",
    "sleepApplication",
    "numReplicas",
    "region",
    "multiRegionConfig",
    "overlapSeconds",
];

/// `railway.json`/`railway.toml` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        for key in [
            "railway",
            "NIXPACKS",
            "RAILPACK",
            "restartPolicyType",
            "healthcheckPath",
            "startCommand",
            "buildCommand",
            "\"build\"",
            "\"deploy\"",
            "[build]",
            "[deploy]",
            "sleepApplication",
        ] {
            if s.contains(key) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// TOML 行 `key = value` / `key: value` からキー名を取る。
fn toml_key(s: &str) -> Option<&str> {
    for sep in ['=', ':'] {
        if let Some(pos) = s.find(sep) {
            let k = s[..pos]
                .trim_end()
                .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
                .next()
                .unwrap_or("");
            if !k.is_empty() {
                return Some(k);
            }
        }
    }
    None
}

/// 行内の `"key":` をすべて列挙して `out` に積む。
fn json_keys<'a>(s: &'a str, out: &mut std::vec::Vec<&'a str>) {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j < bytes.len() && s[j + 1..].trim_start().starts_with(':') {
                out.push(&s[i + 1..j]);
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        keys: 0,
        sections: 0,
        build_keys: 0,
        deploy_keys: 0,
        deploy_controls: 0,
        comments: 0,
    };
    // 0=なし, 1=build, 2=deploy — TOML はセクション、JSON は深度で追跡。
    let mut scope = 0u8;
    let mut scope_depth: i64 = 0;
    let mut depth: i64 = 0;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') && !s.starts_with("[[") && !s.contains('"') {
            let inner = s.trim_start_matches('[').trim_end_matches(']');
            scope = match inner {
                "build" => 1,
                "deploy" => 2,
                _ => 0,
            };
            scope_depth = -1; // TOML セクションは次の `[` まで有効
            if SECTIONS.contains(&inner) {
                c.sections += 1;
            }
            continue;
        }
        let mut keys: std::vec::Vec<&str> = std::vec::Vec::new();
        let toml_line = match (s.find('='), s.find('"')) {
            (Some(e), Some(q)) => e < q,
            (Some(_), None) => true,
            _ => false,
        };
        if toml_line {
            if let Some(k) = toml_key(s) {
                keys.push(k);
            }
        } else {
            json_keys(s, &mut keys);
        }
        for &k in &keys {
            c.keys += 1;
            if SECTIONS.contains(&k) {
                c.sections += 1;
                scope = if k == "build" {
                    1
                } else if k == "deploy" {
                    2
                } else {
                    0
                };
                scope_depth = depth;
                continue;
            }
            if scope == 1 {
                c.build_keys += 1;
            } else if scope == 2 {
                c.deploy_keys += 1;
            }
            if DEPLOY_CONTROLS.contains(&k) {
                c.deploy_controls += 1;
            }
        }
        for ch in s.chars() {
            match ch {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
        }
        if scope != 0 && depth <= scope_depth {
            scope = 0;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_railway() {
        assert!(detect(
            b"{\"build\": {\"builder\": \"NIXPACKS\"}, \"deploy\": {\"restartPolicyType\": \"ON_FAILURE\"}}"
        ));
        assert!(detect(
            b"[build]\nbuilder = \"NIXPACKS\"\n[deploy]\nstartCommand = \"x\"\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"{\"name\": \"x\"}"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"{\n  \"build\": { \"builder\": \"NIXPACKS\" },\n  \"deploy\": { \"startCommand\": \"npm start\", \"healthcheckPath\": \"/h\" }\n}\n";
        let c = parse(b).unwrap();
        assert_eq!(c.keys, 5);
        assert_eq!(c.sections, 2);
        assert_eq!(c.build_keys, 1);
        assert_eq!(c.deploy_keys, 2);
        assert_eq!(c.deploy_controls, 1);
    }

    #[test]
    fn toml_counts() {
        let b = b"[build]\nbuilder = \"NIXPACKS\"\nbuildCommand = \"npm run build\"\n\n[deploy]\nstartCommand = \"npm start\"\nhealthcheckPath = \"/h\"\n";
        let c = parse(b).unwrap();
        assert_eq!(c.keys, 4);
        assert_eq!(c.sections, 2);
        assert_eq!(c.build_keys, 2);
        assert_eq!(c.deploy_keys, 2);
        assert_eq!(c.deploy_controls, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"{\"build\": {}}").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}
