//! Census of an NGINX Unit `conf.json` (or cert bundle upload).
//!
//! Unit JSON config: top-level `listeners`/`routes`/`applications`/
//! `upstreams`/`certificates`/`settings`/`access_log`/`http`/`websocket`
//! sections, `applications` entries `{ "type": "python", "processes": N,
//! "working_directory": … }`, `routes` array of `match`/`action`
//! (`pass`/`share`/`proxy`/`return`), `listeners` `"*:8080"` socket keys,
//! upstream `servers` list, TLS `tls/certificate` references.
//!
//! ```rust
//! let c = izanagi_kit::unitconf::Unitconf::parse(
//!     b"{\"listeners\":{\"*:80\":{\"pass\":\"applications/app\"}},\"applications\":{\"app\":{\"type\":\"python\"}}}",
//! ).unwrap();
//! assert_eq!(c.sections, 2);
//! ```
#![forbid(unsafe_code)]

use crate::textutil::strip_bom;
/// Unit config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unitconf {
    /// Top-level sections seen (`listeners`/`routes`/`applications`/`upstreams`/`certificates`/`settings`/`access_log`).
    pub sections: usize,
    /// `"key":` named entries inside sections.
    pub keys: usize,
    /// `"match"`/`"action"`/`"pass"`/`"share"`/`"proxy"`/`"return"`/`"type"`/`"processes"` control keys.
    pub controls: usize,
}

/// Top-level section names.
const SECTIONS: &[&str] = &[
    "\"listeners\"",
    "\"routes\"",
    "\"applications\"",
    "\"upstreams\"",
    "\"certificates\"",
    "\"settings\"",
    "\"access_log\"",
];

/// Control-ish keys.
const CONTROLS: &[&str] = &[
    "\"match\"",
    "\"action\"",
    "\"pass\"",
    "\"share\"",
    "\"proxy\"",
    "\"return\"",
    "\"type\"",
    "\"processes\"",
];

/// True if `b` looks like a Unit JSON config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let t = t.trim();
    t.starts_with('{')
        && (t.contains("\"listeners\"") || t.contains("\"applications\""))
        && (t.contains("\"pass\"") || t.contains("\"type\""))
}

impl Unitconf {
    /// Parse a Unit JSON config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            keys: 0,
            controls: 0,
        };
        for s in SECTIONS {
            if t.contains(s) {
                c.sections += 1;
            }
        }
        for s in CONTROLS {
            c.controls += t.matches(s).count();
        }
        c.keys = t.matches("\":").count();
        if c.sections == 0 {
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
            "{\n",
            "  \"listeners\": {\n",
            "    \"*:8080\": { \"pass\": \"routes\" }\n",
            "  },\n",
            "  \"routes\": [\n",
            "    { \"match\": { \"uri\": \"/api/*\" }, \"action\": { \"proxy\": \"http://127.0.0.1:9000\" } },\n",
            "    { \"action\": { \"share\": \"/srv$uri\" } }\n",
            "  ],\n",
            "  \"applications\": {\n",
            "    \"app\": { \"type\": \"python 3\", \"processes\": 4, \"working_directory\": \"/srv\" }\n",
            "  }\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Unitconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert!(c.keys >= 8);
        assert!(c.controls >= 5);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(Unitconf::parse(b"not json\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
