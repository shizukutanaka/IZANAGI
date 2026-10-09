//! Render Blueprint `render.yaml` の認識と計数。
//!
//! `render.yaml` は Render の Blueprint 形式で、`services:` リストの各項目が
//! `type:`(webservice/privateService/worker/cronJob/staticSite/pserv)、
//! `name:`、`env:`、`plan:`、`region:`、`buildCommand:`、`startCommand:`、
//! `runtime:`/`repo:`/`branch:`/`domains:`/`healthCheckPath:`/`routes:`、
//! `envVars:`(`- key:`/`value:`/`sync: false`/`fromDatabase:`/`fromService:`)、
//! `databases:`、トップレベル `previewsEnabled:`/`envVarGroups:` を持つ。
//!
//! ```
//! let b = b"services:\n  - type: web\n    name: my-app\n    env: docker\n    plan: starter\n    region: oregon\n    buildCommand: npm run build\n    startCommand: npm start\n    healthCheckPath: /health\n    envVars:\n      - key: NODE_ENV\n        value: production\n      - key: DATABASE_URL\n        fromDatabase:\n          name: my-db\n          property: connectionString\ndatabases:\n  - name: my-db\n    plan: free\npreviewsEnabled: true\n";
//! assert!(izanagi_kit::renderconf::detect(b));
//! let c = izanagi_kit::renderconf::parse(b).unwrap();
//! assert_eq!(c.services, 1);
//! assert_eq!(c.databases, 1);
//! assert_eq!(c.env_vars, 2);
//! assert_eq!(c.keys, 20);
//! ```

use crate::textutil::strip_bom;
/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key:`/`key: value` 行の個数(`- key:` を含む)。
    pub keys: usize,
    /// `services:` 配下の `- type:` サービス項目の個数。
    pub services: usize,
    /// `databases:` 配下の項目個数。
    pub databases: usize,
    /// `envVars:` 配下の `- key:` 項目の個数。
    pub env_vars: usize,
    /// `type:` 値別個数(web/pserv/worker/cron/static 系を含む全 type 行)。
    pub typed: usize,
    /// `fromDatabase:`/`fromService:`/`fromGroup:` 参照キーの個数。
    pub refs: usize,
    /// `-` リスト項目行の個数。
    pub list_items: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

/// `render.yaml` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut services = false;
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("services:") {
            services = true;
        }
        for key in [
            "type: web",
            "type: pserv",
            "type: worker",
            "type: cron",
            "type: static",
            "type: private",
            "env:",
            "plan:",
            "region:",
            "buildCommand:",
            "startCommand:",
            "healthCheckPath:",
            "envVars:",
            "databases:",
            "previewsEnabled:",
        ] {
            if s.contains(key) {
                hits += 1;
            }
        }
    }
    services && hits >= 2
}

/// 行の `key:` からキー名を取る(`- key:` 行は `-` を除いた部分)。
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
        services: 0,
        databases: 0,
        env_vars: 0,
        typed: 0,
        refs: 0,
        list_items: 0,
        comments: 0,
    };
    let mut scope = 0u8; // 0=top, 1=services, 2=databases, 3=envVars
    for l in t.lines() {
        if l.trim().is_empty() {
            continue;
        }
        let indent = l.len() - l.trim_start().len();
        let s = l.trim();
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if indent == 0 {
            scope = match s {
                "services:" => 1,
                "databases:" => 2,
                "envVars:" => 3,
                _ => 0,
            };
        }
        let is_dash = s.starts_with('-');
        if is_dash {
            c.list_items += 1;
        }
        let Some(key) = line_key(s) else {
            continue;
        };
        c.keys += 1;
        match key {
            "type" => {
                c.typed += 1;
                if is_dash && scope == 1 {
                    c.services += 1;
                } else if is_dash && scope == 2 {
                    c.databases += 1;
                }
            }
            "key" => {
                if is_dash && scope == 3 {
                    c.env_vars += 1;
                }
            }
            "name" => {
                if is_dash && scope == 2 {
                    c.databases += 1;
                }
            }
            "fromDatabase" | "fromService" | "fromGroup" | "fromSecret" => {
                c.refs += 1;
            }
            _ => {}
        }
        // envVars は services 内でも現れる: `- key:` 項目を envVars スコープで
        // 追跡するために、`envVars:` 見出しが indent>0 でも切替える。
        if key == "envVars" {
            scope = 3;
        } else if indent == 0 && key != "services" && key != "databases" && key != "envVars" {
            scope = 0;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_render() {
        let b = b"services:\n  - type: web\n    name: app\n    env: docker\n    buildCommand: b\n    startCommand: s\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"name: x\nkind: y\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"services:\n  - type: web\n    name: app\n    envVars:\n      - key: A\n        value: v\n      - key: B\n        fromDatabase:\n          name: db\n          property: connectionString\ndatabases:\n  - name: db\n    plan: free\n";
        let c = parse(b).unwrap();
        assert_eq!(c.services, 1);
        assert_eq!(c.databases, 1);
        assert_eq!(c.env_vars, 2);
        assert_eq!(c.refs, 1);
        assert_eq!(c.typed, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"services:\n  - type: web\n    name: a\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
