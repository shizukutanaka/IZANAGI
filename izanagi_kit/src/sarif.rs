//! SARIF — Static Analysis Results Interchange Format (JSON).
//!
//! Reads `version`, `runs[].tool.driver.name`, and each run's
//! `results[]` (`ruleId`, `level`, `message.text`, first
//! `locations[].physicalLocation.artifactLocation.uri`).
//!
//! ```
//! use izanagi_kit::sarif::parse;
//!
//! let s = parse(br#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"tool"}},"results":[{"ruleId":"R1","level":"warning","message":{"text":"oops"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.c"}}}]}]}]}"#).unwrap();
//! assert_eq!(s.version, "2.1.0");
//! assert_eq!(s.runs[0].results[0].rule_id.as_deref(), Some("R1"));
//! ```

use crate::json::{parse as parse_json, Json};

/// A SARIF `results[]` entry.
#[derive(Clone, Debug)]
pub struct Result_ {
    /// `ruleId`.
    pub rule_id: Option<String>,
    /// `level` (`none`/`note`/`warning`/`error`).
    pub level: Option<String>,
    /// `message.text`.
    pub message: Option<String>,
    /// First artifact URI.
    pub uri: Option<String>,
}

/// A `runs[]` entry.
#[derive(Clone, Debug)]
pub struct Run {
    /// `tool.driver.name`.
    pub tool: Option<String>,
    /// Parsed results.
    pub results: Vec<Result_>,
}

/// A parsed SARIF document.
#[derive(Clone, Debug)]
pub struct Sarif {
    /// `version` (`"2.1.0"`).
    pub version: String,
    /// Runs in document order.
    pub runs: Vec<Run>,
}

fn obj(j: &Json) -> Option<&std::collections::BTreeMap<String, Json>> {
    match j {
        Json::Obj(m) => Some(m),
        _ => None,
    }
}

fn get<'a>(j: &'a Json, k: &str) -> Option<&'a Json> {
    obj(j)?.get(k)
}

fn s(j: &Json) -> Option<String> {
    match j {
        Json::Str(x) => Some(x.clone()),
        _ => None,
    }
}

fn arr(j: &Json) -> Option<&Vec<Json>> {
    match j {
        Json::Arr(v) => Some(v),
        _ => None,
    }
}

/// Parse a SARIF document. `None` when it is not a JSON object or
/// `version` is absent.
pub fn parse(d: &[u8]) -> Option<Sarif> {
    let j = parse_json(d).ok()?;
    let version = s(get(&j, "version")?)?;
    let mut runs = Vec::new();
    if let Some(rs) = get(&j, "runs").and_then(arr) {
        for r in rs {
            let tool = get(r, "tool")
                .and_then(|t| get(t, "driver"))
                .and_then(|d| get(d, "name"))
                .and_then(s);
            let mut results = Vec::new();
            if let Some(res) = get(r, "results").and_then(arr) {
                for x in res {
                    let uri = get(x, "locations")
                        .and_then(arr)
                        .and_then(|l| l.first())
                        .and_then(|l| get(l, "physicalLocation"))
                        .and_then(|p| get(p, "artifactLocation"))
                        .and_then(|a| get(a, "uri"))
                        .and_then(s);
                    results.push(Result_ {
                        rule_id: get(x, "ruleId").and_then(s),
                        level: get(x, "level").and_then(s),
                        message: get(x, "message").and_then(|m| get(m, "text")).and_then(s),
                        uri,
                    });
                }
            }
            runs.push(Run { tool, results });
        }
    }
    Some(Sarif { version, runs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let s = parse(br#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t"}},"results":[{"ruleId":"R","level":"error","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"f.c"}}}]}]}]}"#).unwrap();
        assert_eq!(s.runs[0].tool.as_deref(), Some("t"));
        assert_eq!(s.runs[0].results[0].level.as_deref(), Some("error"));
        assert_eq!(s.runs[0].results[0].uri.as_deref(), Some("f.c"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
        assert!(parse(b"{\"version\":1}").is_none());
        let s = parse(br#"{"version":"2.1.0"}"#).unwrap();
        assert!(s.runs.is_empty());
    }
}
