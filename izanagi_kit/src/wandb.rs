//! Weights & Biases metadata/config parser (`wandb-metadata.json`,
//! `config.yaml`).
//!
//! Detects wandb metadata JSON (`"program"` + `"args"` + `"git"`/`"os"`)
//! and wandb config YAML (`wandb_version:` + `{value:…, desc:…}`),
//! counting metadata keys, config entries, git/env fields and sweep
//! agents.
//!
//! ```
//! let b = concat!(
//!     "{\"python\":\"3\",\"program\":\"train.py\",\"args\":[\"--lr\",\"1\"],",
//!     "\"os\":\"linux\",\"host\":\"h\",\"git\":{\"remote\":\"r\",\"commit\":\"c\"}}"
//! ).as_bytes();
//! assert!(izanagi_kit::wandb::detect(b));
//! let c = izanagi_kit::wandb::Wandb::parse(b).unwrap();
//! assert_eq!(c.metadata_keys, 6);
//! ```

/// Parsed wandb document summary.
#[derive(Debug, Clone)]
pub struct Wandb {
    /// Metadata keys present (`python`, `program`, `args`, `os`, `host`,
    /// `git`, `env`, `gpu`, `state`, `codePath`, `executable`,
    /// `cpu_count`, `gpu_count`, `startedAt`).
    pub metadata_keys: usize,
    /// Config entries (`desc:` / `value:` pairs) in config.yaml form.
    pub config_entries: usize,
    /// `wandb_version:` / `wandb_enabled` version keys.
    pub versions: usize,
    /// `"git"`/`"args"`/`"env"`/`"cpu"`/`"gpu"` telemetry objects.
    pub telemetry: usize,
    /// `agent:`/`sweep`/`controller`/`_wandb:` sweep fields.
    pub sweep: usize,
}

const META: &[&str] = &[
    "\"python\"",
    "\"program\"",
    "\"args\"",
    "\"os\"",
    "\"host\"",
    "\"git\"",
    "\"env\"",
    "\"gpu\"",
    "\"state\"",
    "\"codePath\"",
    "\"executable\"",
    "\"cpu_count\"",
    "\"gpu_count\"",
    "\"startedAt\"",
];

fn count_key(t: &str, key: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        n += 1;
        off += i + key.len();
    }
    n
}

/// Whether the buffer looks like a wandb metadata/config document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"program\"")
        && t.contains("\"args\"")
        && (t.contains("\"git\"") || t.contains("\"os\"")))
        || t.contains("wandb_version:")
        || t.contains("_wandb:")
}

impl Wandb {
    /// Parses a wandb document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let metadata_keys = META.iter().map(|k| count_key(t, k)).sum();
        Some(Self {
            metadata_keys,
            config_entries: count_key(t, "desc:"),
            versions: count_key(t, "wandb_version:") + count_key(t, "wandb_enabled"),
            telemetry: count_key(t, "\"git\"") + count_key(t, "\"env\"") + count_key(t, "\"cpu\""),
            sweep: count_key(t, "agent:")
                + count_key(t, "sweep")
                + count_key(t, "_wandb:")
                + count_key(t, "controller:"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_metadata_json() {
        let b = concat!(
            "{\"python\":\"3\",\"program\":\"train.py\",\"args\":[\"--lr\",\"1\"],",
            "\"os\":\"linux\",\"host\":\"h\",\"git\":{\"remote\":\"r\",\"commit\":\"c\"},",
            "\"env\":{\"e\":\"1\"},\"cpu_count\":8,\"gpu_count\":1,",
            "\"executable\":\"x\",\"codePath\":\"train.py\",\"state\":\"running\"}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Wandb::parse(b).unwrap();
        assert_eq!(c.metadata_keys, 12);
        assert_eq!(c.telemetry, 2);
    }

    #[test]
    fn detects_config_yaml() {
        let b = concat!(
            "wandb_version: 1\n",
            "_wandb:\n",
            "  value: {}\n",
            "lr:\n",
            "  value: 1\n",
            "  desc: learning rate\n",
            "epochs:\n",
            "  value: 10\n",
            "  desc: null\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Wandb::parse(b).unwrap();
        assert_eq!(c.versions, 1);
        assert_eq!(c.config_entries, 2);
        assert_eq!(c.sweep, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"program\": \"x\"}"));
        assert!(!detect(b"name: x\n"));
        assert!(Wandb::parse(b"{}").is_none());
    }
}
