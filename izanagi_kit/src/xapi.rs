//! xAPI (Tin Can) statements — JSON objects with `actor` / `verb` /
//! `object`, plus array-of-statements input. Reuses `crate::json`.
//!
//! ```
//! use izanagi_kit::xapi::parse;
//!
//! let s = parse(br#"{"actor":{"name":"U","mbox":"mailto:u@x"},
//!   "verb":{"id":"http://adlnet.gov/expapi/verbs/answered"},
//!   "object":{"id":"http://x/act/1"}}"#).unwrap();
//! assert_eq!(s.len(), 1);
//! assert_eq!(s[0].verb_id.as_deref(), Some("http://adlnet.gov/expapi/verbs/answered"));
//! ```

use crate::json::Json;
use std::string::String;
use std::vec::Vec;

/// One xAPI statement (fields kept verbatim).
#[derive(Clone, Debug)]
pub struct Statement {
    /// `actor.name`.
    pub actor_name: Option<String>,
    /// `actor.mbox` / `account` name, whichever present.
    pub actor_id: Option<String>,
    /// `verb.id` (IRI).
    pub verb_id: Option<String>,
    /// `object.id`.
    pub object_id: Option<String>,
    /// `object.objectType` (`Agent`/`Activity`/`StatementRef`/…).
    pub object_type: Option<String>,
    /// `timestamp` string when present.
    pub timestamp: Option<String>,
    /// `result.score.scaled` ×1_000_000 when present (integerized).
    pub score_ppm: Option<i64>,
}

fn get<'a>(o: &'a std::collections::BTreeMap<String, Json>, k: &str) -> Option<&'a Json> {
    o.get(k)
}
fn s(v: &Json) -> Option<String> {
    match v {
        Json::Str(t) => Some(t.clone()),
        _ => None,
    }
}
fn obj(v: &Json) -> Option<&std::collections::BTreeMap<String, Json>> {
    match v {
        Json::Obj(m) => Some(m),
        _ => None,
    }
}

fn statement(m: &std::collections::BTreeMap<String, Json>) -> Statement {
    let actor = get(m, "actor").and_then(obj);
    let actor_name = actor.and_then(|a| get(a, "name")).and_then(s);
    let actor_id = actor.and_then(|a| {
        get(a, "mbox")
            .and_then(s)
            .or_else(|| {
                get(a, "account")
                    .and_then(obj)
                    .and_then(|ac| get(ac, "name"))
                    .and_then(s)
            })
            .or_else(|| get(a, "openid").and_then(s))
    });
    let verb_id = get(m, "verb")
        .and_then(obj)
        .and_then(|v| get(v, "id"))
        .and_then(s);
    let object = get(m, "object").and_then(obj);
    let object_id = object.and_then(|o| get(o, "id")).and_then(s);
    let object_type = object.and_then(|o| get(o, "objectType")).and_then(s);
    let timestamp = get(m, "timestamp").and_then(s);
    // score.scaled ∈ [-1,1] → integerize to ppm via string math
    let score_ppm = get(m, "result")
        .and_then(obj)
        .and_then(|r| get(r, "score"))
        .and_then(obj)
        .and_then(|sc| get(sc, "scaled"))
        .and_then(|v| match v {
            Json::Int(n) => Some(n.saturating_mul(1_000_000)),
            Json::Str(t) => {
                let t = t.trim();
                let neg = t.starts_with('-');
                let t = t.trim_start_matches(['-', '+']);
                let (i, f) = t.split_once('.').unwrap_or((t, ""));
                let int: i64 = i.parse().ok()?;
                let mut ppm = int.saturating_mul(1_000_000);
                let mut scale = 100_000i64;
                for c in f.bytes().take(6) {
                    ppm += ((c - b'0') as i64) * scale;
                    scale /= 10;
                }
                Some(if neg { -ppm } else { ppm })
            }
            _ => None,
        });
    Statement {
        actor_name,
        actor_id,
        verb_id,
        object_id,
        object_type,
        timestamp,
        score_ppm,
    }
}

/// Parse an xAPI statement or JSON array of statements.
pub fn parse(d: &[u8]) -> Option<Vec<Statement>> {
    let v = crate::json::parse(d).ok()?;
    let mut out = Vec::new();
    match &v {
        Json::Arr(a) => {
            for it in a {
                let m = obj(it)?;
                get(m, "actor")?;
                get(m, "verb")?;
                get(m, "object")?;
                out.push(statement(m));
            }
        }
        Json::Obj(m) => {
            get(m, "actor")?;
            get(m, "verb")?;
            get(m, "object")?;
            out.push(statement(m));
        }
        _ => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_and_array() {
        let one = parse(
            br#"{"actor":{"name":"A","mbox":"mailto:a@b"},
            "verb":{"id":"http://x/v/1"},"object":{"id":"act/1","objectType":"Activity"},
            "timestamp":"2024-01-01T00:00:00Z","result":{"score":{"scaled":"0.5"}}}"#,
        )
        .unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].actor_name.as_deref(), Some("A"));
        assert_eq!(one[0].object_type.as_deref(), Some("Activity"));
        assert_eq!(one[0].score_ppm, Some(500_000));
        let arr = parse(
            br#"[{"actor":{},"verb":{},"object":{}},
            {"actor":{"name":"B"},"verb":{"id":"v2"},"object":{"id":"o2"}}]"#,
        )
        .unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[1].verb_id.as_deref(), Some("v2"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
        assert!(parse(b"[1]").is_none());
        assert!(parse(b"not json").is_none());
    }
}
