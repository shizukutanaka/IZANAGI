//! pnpm `pnpm-lock.yaml` — `lockfileVersion:` + `importers:` + `packages:`
//! `/name@version:` keys + `resolution:`/`integrity`/`snapshots:`.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of `pnpm-lock.yaml`.
pub struct Pnpmlock {
    /// `lockfileVersion` string.
    pub lockfile_version: String,
    /// `settings:` keys.
    pub settings: usize,
    /// `importers:` entries (`.:` / `apps/x:` workspace roots).
    pub importers: usize,
    /// `dependencies:`/`devDependencies:` blocks across importers.
    pub dependency_blocks: usize,
    /// `devDependencies:` blocks.
    pub dev_dependencies: usize,
    /// `optionalDependencies:` blocks.
    pub optional_dependencies: usize,
    /// `packages:` `/name@ver:` keys.
    pub packages: usize,
    /// `resolution:` entries.
    pub resolutions: usize,
    /// `integrity:` hashes.
    pub integrity: usize,
    /// `dependencies:` items inside packages.
    pub dependencies: usize,
    /// `snapshots:` entries.
    pub snapshots: usize,
    /// `dev: true` flags.
    pub dev: usize,
    /// `engines:` blocks.
    pub engines: usize,
    /// `peerDependencies` mentions.
    pub peers: usize,
    /// `#` comments.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn val_after<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    t.lines().find_map(|l| {
        let tr = l.trim();
        tr.strip_prefix(key).and_then(|r| {
            let r = r.trim_start_matches(':').trim();
            (!r.is_empty() && !r.starts_with('{')).then(|| r.trim_matches(['\'', '"']))
        })
    })
}

fn blocks<'a>(t: &'a str, key: &str) -> Vec<Vec<&'a str>> {
    let mut out = Vec::new();
    let mut cur: Option<(usize, Vec<&'a str>)> = None;
    for l in t.lines() {
        let tr = l.trim_end();
        let i = tr.len() - tr.trim_start().len();
        if let Some((h, v)) = &mut cur {
            if !tr.trim().is_empty() && i <= *h {
                out.push(core::mem::take(v));
                cur = None;
            } else {
                v.push(l);
                continue;
            }
        }
        if cur.is_none() && !tr.trim().is_empty() && is_key(tr.trim(), key) {
            cur = Some((i, Vec::new()));
        }
    }
    if let Some((_, v)) = cur {
        out.push(v);
    }
    out
}

fn child_items(t: &str, key: &str) -> usize {
    blocks(t, key)
        .iter()
        .map(|b| {
            let at = b
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            b.iter()
                .filter(|l| !l.trim().is_empty() && l.len() - l.trim_start().len() == at)
                .count()
        })
        .sum()
}

fn dash_items(t: &str, key: &str) -> usize {
    blocks(t, key)
        .iter()
        .map(|b| {
            let at = b
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            b.iter()
                .filter(|l| {
                    let tr = l.trim_start();
                    l.len() - tr.len() == at && tr.starts_with('-')
                })
                .count()
        })
        .sum()
}

/// `true` when the text looks like pnpm-lock.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    t.contains("lockfileVersion") && (t.contains("importers:") || t.contains("/@"))
}

impl Pnpmlock {
    #[must_use]
    /// Parses `b` into `Pnpmlock`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let packages = t
            .lines()
            .filter(|l| {
                let tr = l.trim();
                tr.starts_with('/') && tr.contains('@') && tr.ends_with(':')
            })
            .count();
        Some(Self {
            lockfile_version: val_after(t, "lockfileVersion").unwrap_or("").to_string(),
            settings: child_items(t, "settings"),
            importers: child_items(t, "importers"),
            dependency_blocks: dash_items(t, "dependencies").max(child_items(t, "dependencies")),
            dev_dependencies: t.matches("devDependencies:").count(),
            optional_dependencies: t.matches("optionalDependencies:").count(),
            packages,
            resolutions: t.matches("resolution:").count(),
            integrity: t.matches("integrity:").count() + t.matches("integrity ").count(),
            dependencies: dash_items(t, "dependencies"),
            snapshots: child_items(t, "snapshots"),
            dev: t.matches("dev: true").count(),
            engines: t.matches("engines:").count(),
            peers: t.matches("peerDependencies").count(),
            comments: t
                .lines()
                .filter(|l| l.trim_start().starts_with('#'))
                .count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"lockfileVersion: '9\x2e0'\n\nsettings:\n  autoInstallPeers: true\n\nimporters:\n  .:\n    dependencies:\n      left-pad:\n        specifier: ^1\n        version: 1\x2e3\n\npackages:\n  /left-pad@1\x2e3:\n    resolution: {integrity: sha512-x}\n    engines: {node: '>=0\x2e10'}\n    dev: true\n\nsnapshots:\n  /left-pad@1\x2e3: {}\n";

    #[test]
    fn detects_pnpmlock() {
        assert!(detect(FIX));
        assert!(!detect(b"version: 1"));
    }

    #[test]
    fn parses_pnpmlock() {
        let p = Pnpmlock::parse(FIX).unwrap();
        assert_eq!(p.lockfile_version, "9\x2e0");
        assert_eq!(p.settings, 1);
        assert_eq!(p.importers, 1);
        assert_eq!(p.packages, 1);
        assert_eq!(p.resolutions, 1);
        assert_eq!(p.dev, 1);
        assert_eq!(p.snapshots, 1);
        assert!(Pnpmlock::parse(b"").is_none());
    }
}
