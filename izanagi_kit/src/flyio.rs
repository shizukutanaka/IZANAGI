//! Fly.io `fly.toml` アプリケーション設定の認識と計数。
//!
//! `fly.toml` は TOML で、トップレベルに `app` / `primary_region` /
//! `kill_signal` / `kill_timeout` / `console_command` を置き、`[build]` /
//! `[deploy]` / `[env]` / `[http_service]` / `[services]` / `[checks]` /
//! `[mounts]` / `[processes]` / `[experimental]` / `[[vm]]` /
//! `[[services.ports]]` / `[[services.tcp_checks]]` 等のセクションを持つ。
//!
//! ```
//! let b = b"app = \"my-app\"\nprimary_region = \"nrt\"\n\n[build]\n  image = \"nginx:latest\"\n\n[http_service]\n  internal_port = 8080\n  force_https = true\n  [http_service.concurrency]\n    type = \"requests\"\n    soft_limit = 200\n\n[[services]]\n  internal_port = 443\n  [[services.ports]]\n    port = 443\n    handlers = [\"tls\", \"http\"]\n\n[env]\n  LOG_LEVEL = \"info\"\n";
//! assert!(izanagi_kit::flyio::detect(b));
//! let c = izanagi_kit::flyio::parse(b).unwrap();
//! assert_eq!(c.assigns, 11); // app/primary_region/image/internal_port×2/force_https/type/soft_limit/port/handlers/LOG_LEVEL
//! assert_eq!(c.tables, 4); // [build] [http_service] [http_service.concurrency] [env]
//! assert_eq!(c.array_tables, 2); // [[services]] [[services.ports]]
//! assert_eq!(c.service_keys, 4); // http_service + concurrency + services + services.ports
//! ```

use crate::textutil::strip_bom;
/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` 行の個数。
    pub assigns: usize,
    /// `[table]` 行の個数(`[[array]]` を除く)。
    pub tables: usize,
    /// `[[array-table]]` 行の個数。
    pub array_tables: usize,
    /// `[http_service]`/`[services]`/`[checks]`/`[deploy]`/`[processes]` 等
    /// サービス系セクションおよびそのネストの個数。
    pub service_keys: usize,
    /// `true`/`false` を値に持つ代入の個数。
    pub bools: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const SERVICE_TABLES: &[&str] = &[
    "http_service",
    "services",
    "checks",
    "deploy",
    "processes",
    "mounts",
    "vm",
];

/// `fly.toml` らしさを返す。`app =`/`primary_region` と `[http_service]`/
/// `[services]`/`[[services.ports]]` 等の組合せで判定。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    let mut app = false;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if s.starts_with("app =")
            || s.starts_with("app=")
            || s.starts_with("primary_region")
            || s.starts_with("kill_signal")
            || s.starts_with("console_command")
        {
            app = true;
        }
        for key in [
            "[http_service",
            "[services",
            "[[services",
            "[deploy]",
            "[checks]",
            "internal_port",
            "force_https",
            "fly.io",
        ] {
            if s.starts_with(key) {
                hits += 1;
            }
        }
    }
    app && hits >= 1 || hits >= 3
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        assigns: 0,
        tables: 0,
        array_tables: 0,
        service_keys: 0,
        bools: 0,
        comments: 0,
    };
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if s.starts_with("[[") {
            c.array_tables += 1;
            let inner = s.trim_start_matches('[').trim_end_matches(']');
            if SERVICE_TABLES
                .iter()
                .any(|p| inner == *p || inner.starts_with(&format!("{p}.")))
            {
                c.service_keys += 1;
            }
            continue;
        }
        if s.starts_with('[') {
            c.tables += 1;
            let inner = s.trim_start_matches('[').trim_end_matches(']');
            if SERVICE_TABLES
                .iter()
                .any(|p| inner == *p || inner.starts_with(&format!("{p}.")))
            {
                c.service_keys += 1;
            }
            continue;
        }
        let Some(eq) = s.find('=') else {
            continue;
        };
        let key = s[..eq]
            .trim_end()
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'))
            .next()
            .unwrap_or("");
        if key.is_empty() {
            continue;
        }
        c.assigns += 1;
        let v = s[eq + 1..].trim();
        if v == "true" || v == "false" {
            c.bools += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_fly() {
        let b = b"app = \"a\"\n[http_service]\n  internal_port = 8080\n";
        assert!(detect(b));
        assert!(detect(b"[services]\n  internal_port = 80\n  [[services.ports]]\n    port = 80\n    handlers = [\"http\"]\n"));
    }

    #[test]
    fn rejects_other_toml() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(&[0xff, 0x00]));
    }

    #[test]
    fn counts() {
        let b = b"app = \"my-app\"\nprimary_region = \"nrt\"\n\n[http_service]\n  internal_port = 8080\n  force_https = true\n\n[[services]]\n  internal_port = 443\n  [[services.ports]]\n    port = 443\n\n[env]\n  X = \"1\"\n";
        let c = parse(b).unwrap();
        assert_eq!(c.assigns, 7);
        assert_eq!(c.tables, 2); // http_service + env
        assert_eq!(c.array_tables, 2);
        assert_eq!(c.service_keys, 3); // http_service + services + services.ports
        assert_eq!(c.bools, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"app = \"a\"\n[http_service]\n  internal_port = 1\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
