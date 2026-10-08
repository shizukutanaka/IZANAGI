//! RAML 0.8 / 1.0 REST API definitions — `#%RAML <version>` YAML.

use core::str::from_utf8;

const METHODS: [&str; 7] = ["get", "put", "post", "delete", "patch", "head", "options"];

#[derive(Debug, Clone)]
/// Parsed census of a RAML document.
pub struct Raml {
    /// `#%RAML` header version (`1\x2e0` / `0\x2e8` / Library/Trait fragment name).
    pub version: String,
    /// `title:`.
    pub title: String,
    /// `version:` API version field.
    pub api_version: String,
    /// `baseUri:`.
    pub base_uri: String,
    /// Resource paths (`/x` or `/{id}`-headed lines).
    pub resources: usize,
    /// Method keys (`get:`/`post:`/…) at any indent.
    pub methods: usize,
    /// `is:` trait applications + `traits:` declared trait entries.
    pub traits: usize,
    /// `securitySchemes:` entries.
    pub security_schemes: usize,
    /// `types:`/`schemas:` declared entries.
    pub types: usize,
    /// `uses:` library imports.
    pub uses: usize,
    /// `responses:` status codes (`200:`-style keys).
    pub responses: usize,
    /// `documentation:` entries.
    pub docs: usize,
    /// `annotations`/`(annotationName)` usages.
    pub annotations: usize,
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
        let k = tr.trim_start_matches(['"', '\'']);
        k.strip_prefix(key).and_then(|r| {
            let r = r.trim_start_matches(':').trim();
            (!r.is_empty() && !r.starts_with('{')).then(|| r.trim_matches('"'))
        })
    })
}

fn child_keys(t: &str, key: &str) -> usize {
    let mut out = 0;
    let mut inside = false;
    let mut at = 0usize;
    let mut min: Option<usize> = None;
    for l in t.lines() {
        let tr = l.trim_end();
        let i = tr.len() - tr.trim_start().len();
        if inside {
            if !tr.trim().is_empty() && i <= at {
                inside = false;
            } else if !tr.trim().is_empty() {
                if min.is_none() {
                    min = Some(i);
                }
                let tr = tr.trim();
                if Some(i) == min && (tr.ends_with(':') || tr.contains(": ")) {
                    out += 1;
                }
                continue;
            }
        }
        if !tr.trim().is_empty() && is_key(tr.trim(), key) {
            inside = true;
            at = i;
            min = None;
        }
    }
    out
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `true` when the text has a `#%RAML` marker.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .take(4)
        .any(|l| l.trim_start().starts_with("#%RAML"))
}

impl Raml {
    #[must_use]
    /// Parses `b` into `Raml`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        let t = strip_bom(t);
        if !detect(b) {
            return None;
        }
        let version = t
            .lines()
            .find_map(|l| l.trim_start().strip_prefix("#%RAML"))
            .map(|r| r.trim().to_string())
            .unwrap_or_default();
        let mut resources = 0;
        let mut methods = 0;
        let mut responses = 0;
        let mut comments = 0;
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                comments += 1;
                continue;
            }
            if tr.starts_with('/') && tr.ends_with(':') {
                resources += 1;
                continue;
            }
            if METHODS.iter().any(|m| is_key(tr, m)) {
                methods += 1;
                continue;
            }
            if let Some(pos) = tr.find(':') {
                if pos > 0 && tr[..pos].chars().all(|c| c.is_ascii_digit()) {
                    responses += 1;
                }
            }
        }
        Some(Self {
            version,
            title: val_after(t, "title").unwrap_or("").to_string(),
            api_version: val_after(t, "version").unwrap_or("").to_string(),
            base_uri: val_after(t, "baseUri").unwrap_or("").to_string(),
            resources,
            methods,
            traits: child_keys(t, "traits") + t.matches("is: [").count(),
            security_schemes: child_keys(t, "securitySchemes"),
            types: child_keys(t, "types") + child_keys(t, "schemas"),
            uses: child_keys(t, "uses"),
            responses,
            docs: child_keys(t, "documentation"),
            annotations: t
                .lines()
                .filter(|l| l.trim().starts_with('(') && l.trim().ends_with(':'))
                .count(),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"#%RAML 1\x2e0\ntitle: Shop API\nversion: v1\nbaseUri: https://api.example.com\ntraits:\n  secured:\n    usage: applies to all\ntypes:\n  User:\n    type: object\n/users:\n  get:\n    is: [secured]\n    responses:\n      200:\n      404:\n  /{id}:\n    put:\n      responses:\n        200:\n";

    #[test]
    fn detects_raml() {
        assert!(detect(FIX));
        assert!(!detect(b"title: plain yaml"));
    }

    #[test]
    fn parses_raml() {
        let r = Raml::parse(FIX).unwrap();
        assert_eq!(r.version, "1\x2e0");
        assert_eq!(r.title, "Shop API");
        assert_eq!(r.api_version, "v1");
        assert_eq!(r.resources, 2);
        assert_eq!(r.methods, 2);
        assert_eq!(r.traits, 2);
        assert_eq!(r.types, 1);
        assert_eq!(r.responses, 3);
        assert!(Raml::parse(b"").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
