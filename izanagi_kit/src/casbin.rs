//! Census of a Casbin `model.conf` (and `.csv` policy lines).
//!
//! Casbin model.conf: `[request_definition]`/`[policy_definition]`/
//! `[policy_effect]`/`[matchers]`/`[role_definition]`/`[validators]`/
//! `[function_def]` sections with `r = sub, obj, act` style assignments
//! (`r2`/`p2`/`e`/`m` variants and `[custom*`/`rbac.*` functions like
//! `keyMatch`/`regexMatch`/`globMatch`/`keyGet`). Policy `.csv` lines:
//! `p, sub, obj, act`/`g, user, role` CSV rows. `#`/`;` comments.
//!
//! ```rust
//! let c = izanagi_kit::casbin::Casbin::parse(
//!     b"[request_definition]\nr = sub, obj, act\n[policy_effect]\ne = some(where (p.eft == allow))\n",
//! ).unwrap();
//! assert_eq!(c.sections, 2);
//! ```
#![forbid(unsafe_code)]

/// Casbin model/policy census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Casbin {
    /// `[section]` headers.
    pub sections: usize,
    /// `r`/`p`/`e`/`m`/`g`/`p2`-style `key = value` model lines.
    pub definitions: usize,
    /// `p, …`/`g, …` CSV policy rows.
    pub policies: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// True if `b` looks like Casbin config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[request_definition]")
        || t.contains("[policy_effect]")
        || t.contains("[matchers]")
        || (t.contains("p.eft") && t.contains("p.obj"))
        || t.lines().take(8).any(|l| {
            let l = l.trim_start();
            l.starts_with("p, ") || l.starts_with("g, ")
        })
}

impl Casbin {
    /// Parse model.conf/policy.csv into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            definitions: 0,
            policies: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                c.comments += 1;
            } else if l.starts_with('[') && l.ends_with(']') {
                c.sections += 1;
            } else if l.starts_with("p,")
                || l.starts_with("g,")
                || l.starts_with("p ,")
                || l.starts_with("g ,")
            {
                c.policies += 1;
            } else if let Some(eq) = l.find('=') {
                let key = l[..eq].trim();
                let head = key.trim_end_matches(|ch: char| ch.is_ascii_digit());
                if head.len() == 1 && matches!(head, "r" | "p" | "e" | "m" | "g" | "r2" | "p2")
                    || head.starts_with("function")
                {
                    c.definitions += 1;
                }
            }
        }
        if c.sections + c.definitions + c.policies == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# casbin\n",
            "[request_definition]\n",
            "r = sub, obj, act\n",
            "[policy_definition]\n",
            "p = sub, obj, act\n",
            "[policy_effect]\n",
            "e = some(where (p.eft == allow))\n",
            "[matchers]\n",
            "m = r.sub == p.sub && keyMatch(r.obj, p.obj) && regexMatch(r.act, p.act)\n",
            "[role_definition]\n",
            "g = _, _\n",
            "p, alice, data1, read\n",
            "p, bob, data2, write\n",
            "g, alice, admin\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Casbin::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.definitions, 5);
        assert_eq!(c.policies, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\na=b\n"));
        assert!(Casbin::parse(b"# none\n").is_none());
    }
}
