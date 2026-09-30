//! Parser for Docker Compose files (`docker-compose.yml` / `compose.yaml`).
//!
//! Counts `services:` entries, `image:`/`build:` uses, `- ` port and env
//! items, top-level `volumes:`/`networks:`/`secrets:`/`configs:` entries,
//! and comments.
//!
//! ```
//! let b = b"services:\n  web:\n    image: nginx:latest\n    ports:\n      - \"80:80\"\n";
//! assert!(izanagi_kit::compose::detect(b));
//! let c = izanagi_kit::compose::Compose::parse(b).unwrap();
//! assert_eq!(c.services, 1);
//! assert_eq!(c.images, 1);
//! assert_eq!(c.port_maps, 1);
//! ```

/// Parsed Compose file summary.
#[derive(Debug, Clone)]
pub struct Compose {
    /// `version:` value (empty when absent — modern files omit it).
    pub version: String,
    /// Entries under `services:`.
    pub services: usize,
    /// `image:` lines inside services.
    pub images: usize,
    /// `build:` lines inside services.
    pub builds: usize,
    /// `- ` items inside `ports:` blocks.
    pub port_maps: usize,
    /// Items inside `environment:` blocks (list or map form).
    pub env_entries: usize,
    /// `- ` items inside `depends_on:` blocks.
    pub depends_on: usize,
    /// Entries under top-level `volumes:`.
    pub volume_decls: usize,
    /// Entries under top-level `networks:`.
    pub network_decls: usize,
    /// Entries under top-level `secrets:`.
    pub secrets: usize,
    /// Entries under top-level `configs:`.
    pub configs: usize,
    /// `#` comment lines.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn blocks<'a>(t: &'a str, key: &str) -> Vec<Vec<&'a str>> {
    let mut out: Vec<Vec<&'a str>> = Vec::new();
    let mut cur: Option<(usize, Vec<&'a str>)> = None;
    for l in t.lines() {
        let tr = l.trim_start();
        let i = l.len() - tr.len();
        if let Some((d, v)) = cur.as_mut() {
            if !tr.is_empty() && i <= *d {
                out.push(core::mem::take(v));
                cur = None;
            } else {
                v.push(l);
                continue;
            }
        }
        if cur.is_none() && !tr.is_empty() && is_key(tr, key) {
            cur = Some((i, Vec::new()));
        }
    }
    if let Some((_, v)) = cur.take() {
        out.push(v);
    }
    out
}

/// Non-blank lines at the shallowest indent inside each block.
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

/// `- ` items directly inside blocks named `key`.
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

/// Items under top-level (indent-0) `key:` blocks only — `volumes:` etc. may
/// also appear per-service, where they are mounts, not declarations.
fn top_items(t: &str, key: &str) -> usize {
    let mut n = 0usize;
    let mut ind: Option<usize> = None;
    let mut at: Option<usize> = None;
    for l in t.lines() {
        let tr = l.trim_start();
        let i = l.len() - tr.len();
        if let Some(d) = ind {
            if tr.is_empty() {
                continue;
            }
            if i <= d {
                ind = None;
                at = None;
            } else {
                if !tr.starts_with('#') {
                    let a = at.get_or_insert(i);
                    if i == *a {
                        n += 1;
                    }
                }
                continue;
            }
        }
        if ind.is_none() && i == 0 && is_key(tr, key) {
            ind = Some(i);
        }
    }
    n
}

fn val_after<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for l in t.lines() {
        let tr = l.trim();
        if let Some(rest) = tr.trim_start_matches(['"', '\'']).strip_prefix(key) {
            let rest = rest.trim_start_matches(['"', '\'']).trim_start();
            if let Some(v) = rest.strip_prefix(':') {
                let v = v
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .trim_end_matches(',');
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Returns `true` when `b` looks like a Compose file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    if !blocks(t, "services").first().is_some_and(|b| !b.is_empty()) {
        return false;
    }
    t.contains("image:")
        || t.contains("\"image\"")
        || t.contains("build:")
        || t.contains("\"build\"")
        || t.contains("container_name:")
}

impl Compose {
    /// Parses `b` as a Compose file.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let svc = blocks(t, "services");
        let images = svc
            .iter()
            .flatten()
            .filter(|l| is_key(l.trim_start(), "image"))
            .count();
        let builds = svc
            .iter()
            .flatten()
            .filter(|l| is_key(l.trim_start(), "build"))
            .count();
        // `environment:`/`depends_on:` accept list (`- `) or map form — take
        // whichever interpretation yields more items rather than summing.
        let env_entries = dash_items(t, "environment").max(child_items(t, "environment"));
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            version: val_after(t, "version").unwrap_or("").to_string(),
            services: child_items(t, "services"),
            images,
            builds,
            port_maps: dash_items(t, "ports"),
            env_entries,
            depends_on: dash_items(t, "depends_on").max(child_items(t, "depends_on")),
            volume_decls: top_items(t, "volumes"),
            network_decls: top_items(t, "networks"),
            secrets: top_items(t, "secrets"),
            configs: top_items(t, "configs"),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# stack
version: \"3\x2e8\"
services:
  web:
    image: nginx:latest
    ports:
      - \"80:80\"
      - \"443:443\"
    environment:
      - KEY=v
      - OTHER=x
    depends_on:
      - db
  db:
    image: postgres:15
    volumes:
      - pg:/data
volumes:
  pg:
networks:
  front:
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"services: {}"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let c = Compose::parse(SRC).unwrap();
        assert_eq!(c.version, "3\x2e8");
        assert_eq!(c.services, 2);
        assert_eq!(c.images, 2);
        assert_eq!(c.port_maps, 2);
        assert_eq!(c.env_entries, 2);
        assert_eq!(c.depends_on, 1);
        assert_eq!(c.volume_decls, 1);
        assert_eq!(c.network_decls, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Compose::parse(b"\x01\x02").is_none());
    }
}
