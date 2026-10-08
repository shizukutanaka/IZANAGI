//! Platform.sh `.platform.app.yaml` / `.platform/routes.yaml` / `services.yaml`
//! の認識と計数。
//!
//! `.platform.app.yaml` はアプリ定義で、`name:`/`type:`(ランタイム識別)、
//! `runtime:`、`relationships:`(`service: "svc:db"` 形式)、
//! `web:`/`workers:`/`crons:`(commands.start/spec/croncmd)、`hooks:`(build/
//! deploy/post_deploy)、`mounts:`(`"/dir": { source: local, source_path: … }`)、
//! `disk:`/`size:`/`build:`/`dependencies:`/`access:`/`firewall:`/
//! `variables:`/`operations:` を持つ。`routes.yaml` は
//! `"https://{default}/": { type: upstream, upstream: "app:http" }` 形式、
//! `services.yaml` は `svc: { type: mariadb:11.4, disk: … }` 形式。
//!
//! ```
//! let b = b"name: app\ntype: \"python:3.11\"\n\nrelationships:\n    db: \"mariadb:mysql\"\n\nweb:\n    commands:\n        start: \"gunicorn app:app\"\n    locations:\n        \"/\":\n            root: \"public\"\n            passthru: true\n\ndisk: 512\n\nmounts:\n    \"/var\":\n        source: local\n        source_path: var\n\nhooks:\n    build: |\n        pip install -r requirements.txt\n    deploy: |\n        python manage.py migrate\n";
//! assert!(izanagi_kit::platformsh::detect(b));
//! let c = izanagi_kit::platformsh::parse(b).unwrap();
//! assert_eq!(c.keys, 19);
//! assert_eq!(c.sections, 6); // relationships/web/locations/mounts/hooks/build
//! assert_eq!(c.hooks, 2); // build, deploy
//! assert_eq!(c.mounts, 1);
//! assert_eq!(c.relationships, 1);
//! assert_eq!(c.scalars, 3); // name, type, disk
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key:`/`key: value` 行の個数(クォートキー含む)。
    pub keys: usize,
    /// `relationships:`/`web:`/`workers:`/`crons:`/`hooks:`/`mounts:`/`build:`/
    /// `dependencies:`/`access:`/`firewall:`/`variables:`/`operations:`/
    /// `locations:`/`rules:` 等構造セクションの個数。
    pub sections: usize,
    /// `hooks:` 配下の `build`/`deploy`/`post_deploy` エントリ数。
    pub hooks: usize,
    /// `mounts:` 直下のマウントパス数(`"/path":` エントリ)。
    pub mounts: usize,
    /// `relationships:` 直下のサービス参照数。
    pub relationships: usize,
    /// `type:`/`size:`/`disk:`/`name:`/`runtime:` スカラプロパティの個数。
    pub scalars: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const SECTION_KEYS: &[&str] = &[
    "relationships",
    "web",
    "workers",
    "crons",
    "hooks",
    "mounts",
    "build",
    "dependencies",
    "access",
    "firewall",
    "variables",
    "operations",
    "locations",
    "rules",
    "routes",
    "upstream",
    "cache",
    "ssi",
    "redirects",
    "primary",
];

const SCALAR_KEYS: &[&str] = &["type", "size", "disk", "name", "runtime", "timezone"];

const HOOK_KEYS: &[&str] = &["build", "deploy", "post_deploy"];

/// クォート `"/path":` を含む行から `key:` のキー名を取る。
fn line_key(s: &str) -> Option<&str> {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix('"') {
        let end = rest.find('"')?;
        if rest[end + 1..].trim_start().starts_with(':') {
            return Some(&rest[..end]);
        }
        return None;
    }
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
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `.platform.app.yaml` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        for key in [
            "relationships:",
            "mounts:",
            "hooks:",
            "web:",
            "workers:",
            "crons:",
            "source_path",
            "{default}",
            "upstream:",
            "passthru",
            "type: \"",
        ] {
            if s.starts_with(key) || s.contains(key) {
                hits += 1;
            }
        }
    }
    hits >= 3
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        keys: 0,
        sections: 0,
        hooks: 0,
        mounts: 0,
        relationships: 0,
        scalars: 0,
        comments: 0,
    };
    // -1=セクション外, それ以外=セクション見出しのインデント幅。
    let mut in_hooks: i64 = -1;
    let mut in_mounts: i64 = -1;
    let mut in_rel: i64 = -1;
    for l in t.lines() {
        if l.trim().is_empty() {
            continue;
        }
        let ind = (l.len() - l.trim_start().len()) as i64;
        let s = l.trim();
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        // セクション終了判定: インデントが見出し以下に戻ったら脱出。
        if in_hooks >= 0 && ind <= in_hooks {
            in_hooks = -1;
        }
        if in_mounts >= 0 && ind <= in_mounts {
            in_mounts = -1;
        }
        if in_rel >= 0 && ind <= in_rel {
            in_rel = -1;
        }
        let Some(key) = line_key(s) else {
            continue;
        };
        c.keys += 1;
        if SECTION_KEYS.contains(&key) {
            c.sections += 1;
        }
        if SCALAR_KEYS.contains(&key) {
            c.scalars += 1;
        }
        if in_hooks >= 0 && HOOK_KEYS.contains(&key) && ind > in_hooks {
            c.hooks += 1;
        } else if in_mounts >= 0 && ind > in_mounts && s.starts_with('"') {
            c.mounts += 1;
        } else if in_rel >= 0 && ind > in_rel {
            c.relationships += 1;
        }
        match key {
            "hooks" => in_hooks = ind,
            "mounts" => in_mounts = ind,
            "relationships" => in_rel = ind,
            _ => {}
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_platform() {
        let b = b"name: app\ntype: \"python:3.11\"\nrelationships:\n    db: \"m:mysql\"\nweb:\n    locations:\n        \"/\": {root: public}\nmounts:\n    \"/var\": {source: local}\nhooks:\n    build: x\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"name: x\nfoo: bar\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"name: app\ntype: \"python:3.11\"\nrelationships:\n    db: \"m:mysql\"\nmounts:\n    \"/var\": {source: local}\n    \"/static\": {source: local}\nhooks:\n    build: |\n        cmd1\n    deploy: |\n        cmd2\n";
        let c = parse(b).unwrap();
        assert_eq!(c.relationships, 1);
        assert_eq!(c.mounts, 2);
        assert_eq!(c.hooks, 2);
        assert_eq!(c.scalars, 2); // name + type
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"name: a\ntype: \"php:8\"\nrelationships:\n    db: x\nmounts:\n    \"/v\": {}\nhooks:\n    build: x\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
