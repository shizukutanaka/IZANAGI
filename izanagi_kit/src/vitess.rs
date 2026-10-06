//! Vitess Operator manifest census (`planetscale.com`).
//!
//! `apiVersion` in the `planetscale.com` API group(s) together with a
//! `kind` in `VitessCluster, VitessShard, VitessCell, VitessKeyspace, VitessBackupStorage, VitessBackupSchedule, VitessBackup, VitessClusterInit, EtcdLockserver`.
//!
//! ```rust
//! let k = b"apiVersion: planetscale.com/v1\nkind: VitessCluster\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
//! assert!(izanagi_kit::vitess::detect(k));
//! ```

/// Vitess Operator manifest census.
#[derive(Debug, Clone)]
pub struct Vitess {
    /// `kind` value, when present.
    pub kind: Option<String>,
    /// `apiVersion`/`kind`/`metadata`/`spec` envelope lines.
    pub envelope: usize,
    /// `- ` list items.
    pub items: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const GROUPS: &[&str] = &["planetscale.com"];

const KINDS: &[&str] = &[
    "VitessCluster",
    "VitessShard",
    "VitessCell",
    "VitessKeyspace",
    "VitessBackupStorage",
    "VitessBackupSchedule",
    "VitessBackup",
    "VitessClusterInit",
    "EtcdLockserver",
];

fn value_of(s: &str) -> &str {
    match s.find(':') {
        Some(i) => s[i + 1..].trim().trim_matches('"').trim_matches('\''),
        None => "",
    }
}

fn api_ok(t: &str) -> bool {
    t.lines().any(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("apiVersion") {
            return false;
        }
        let v = value_of(s);
        let g0 = v.split('/').next().unwrap_or("");
        // `x.group`-style subdomain groups count too.
        GROUPS.iter().any(|g| {
            v.starts_with(g)
                || g0
                    .strip_suffix(*g)
                    .is_some_and(|p| p.is_empty() || p.ends_with('.'))
        })
    })
}

fn kind_val(t: &str) -> Option<String> {
    t.lines().find_map(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("kind") {
            return None;
        }
        let v = value_of(s);
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    })
}

/// Detect a Vitess Operator manifest.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if !api_ok(t) {
        return false;
    }
    match kind_val(t) {
        Some(k) => KINDS.contains(&k.as_str()),
        None => false,
    }
}

impl Vitess {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kind: kind_val(t),
            envelope: 0,
            items: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                let k = s[..s.find(':').unwrap_or(s.len())].trim();
                if matches!(k, "apiVersion" | "kind" | "metadata" | "spec") {
                    c.envelope += 1;
                }
                if !k.is_empty() {
                    c.settings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"apiVersion: planetscale.com/v1\nkind: VitessCluster\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
        assert!(detect(b));
        let c = Vitess::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("VitessCluster"));
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"apiVersion: v1\nkind: VitessCluster\n"));
        assert!(!detect(
            b"apiVersion: planetscale.com/v1\nkind: Deployment\n"
        ));
        assert!(!detect(
            b"# apiVersion: planetscale.com/v1\nkind: VitessCluster\n"
        ));
    }
}
