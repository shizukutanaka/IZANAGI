//! DVC pipeline file parser (`dvc.yaml` / `dvc.lock`).
//!
//! Detects `stages:` + `cmd:`/`outs:`/`deps:` pipeline definitions and
//! counts stages, command/deps/outs/metrics/params entries, `md5`
//! checksums and frozen flags.
//!
//! ```
//! let b = concat!(
//!     "stages:\n",
//!     "  prepare:\n",
//!     "    cmd: python prep.py\n",
//!     "    deps:\n",
//!     "      - prep.py\n",
//!     "    outs:\n",
//!     "      - data\n",
//!     "  train:\n",
//!     "    cmd: python train.py\n",
//!     "    frozen: true\n"
//! ).as_bytes();
//! assert!(izanagi_kit::dvcfile::detect(b));
//! let c = izanagi_kit::dvcfile::Dvcfile::parse(b).unwrap();
//! assert_eq!(c.stages, 2);
//! assert_eq!(c.cmds, 2);
//! ```

/// Parsed DVC pipeline file summary.
#[derive(Debug, Clone)]
pub struct Dvcfile {
    /// Named stage entries under `stages:`.
    pub stages: usize,
    /// `cmd:` entries.
    pub cmds: usize,
    /// `deps:` list items (`- ` lines inside deps block).
    pub deps: usize,
    /// `outs:` list items.
    pub outs: usize,
    /// `metrics:`/`params:`/`plots:` list items.
    pub metrics_params_plots: usize,
    /// `md5:`/`etag:`/`checksum_jobs:` checksum entries.
    pub checksums: usize,
    /// `frozen: true` / `always_changed: true` flags.
    pub frozen: usize,
    /// `foreach:` entries.
    pub foreach: usize,
}

/// Whether the buffer looks like a DVC pipeline file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("stages:") && (t.contains("cmd:") || t.contains("outs:") || t.contains("deps:"))
}

impl Dvcfile {
    /// Parses a DVC pipeline file summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            stages: 0,
            cmds: 0,
            deps: 0,
            outs: 0,
            metrics_params_plots: 0,
            checksums: 0,
            frozen: 0,
            foreach: 0,
        };
        let mut in_stages = false;
        let mut block: Option<u8> = None; // b'd'eps, b'o'uts, b'm'etrics/params/plots
        for l in t.lines() {
            if l == "stages:" {
                in_stages = true;
                continue;
            }
            if !in_stages {
                continue;
            }
            if l.starts_with("  ")
                && !l.starts_with("    ")
                && !l.trim_start().starts_with('-')
                && l.trim_end().ends_with(':')
            {
                c.stages += 1;
            }
            if !l.starts_with(' ') && !l.is_empty() && !l.starts_with('#') {
                in_stages = false;
            }
            let tr = l.trim();
            match tr {
                "cmd:" | "deps:" | "outs:" | "metrics:" | "params:" | "plots:" | "foreach:"
                | "do:" => {
                    block = match tr {
                        "deps:" => Some(b'd'),
                        "outs:" => Some(b'o'),
                        "metrics:" | "params:" | "plots:" => Some(b'm'),
                        "foreach:" => Some(b'f'),
                        _ => None,
                    };
                    if tr == "cmd:" {
                        c.cmds += 1;
                    }
                    if tr == "foreach:" {
                        c.foreach += 1;
                    }
                }
                s if s.starts_with("cmd: ") => c.cmds += 1,
                s if s.starts_with("md5:")
                    || s.starts_with("etag:")
                    || s.starts_with("checksum") =>
                {
                    c.checksums += 1
                }
                s if s.starts_with("frozen:") || s.starts_with("always_changed:") => c.frozen += 1,
                s if s.starts_with("- ") => match block {
                    Some(b'd') => c.deps += 1,
                    Some(b'o') => c.outs += 1,
                    Some(b'm') => c.metrics_params_plots += 1,
                    _ => {}
                },
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "stages:\n",
            "  prepare:\n",
            "    cmd: python prep.py\n",
            "    deps:\n",
            "      - prep.py\n",
            "      - raw.csv\n",
            "    outs:\n",
            "      - path: data\n",
            "        md5: aabb\n",
            "  train:\n",
            "    cmd: python train.py\n",
            "    metrics:\n",
            "      - metrics.json\n",
            "    params:\n",
            "      - lr\n",
            "    frozen: true\n",
            "  sweep:\n",
            "    foreach:\n",
            "      - a\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Dvcfile::parse(b).unwrap();
        assert_eq!(c.stages, 3);
        assert_eq!(c.cmds, 2);
        assert_eq!(c.deps, 2);
        assert_eq!(c.outs, 1);
        assert_eq!(c.metrics_params_plots, 2);
        assert_eq!(c.checksums, 1);
        assert_eq!(c.frozen, 1);
        assert_eq!(c.foreach, 1);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"stages: []\n"));
        assert!(Dvcfile::parse(b"x").is_none());
    }
}
