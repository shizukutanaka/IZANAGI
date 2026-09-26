//! Minimal reader for CSL-JSON bibliographies (the input format of
//! citeproc / citeproc-js): a JSON array of items, each with `type`,
//! optional `id`, `title`, `author` (array of `{"family","given"}`), and
//! `issued` (a `{"date-parts": [[y,m,d]]}` object). Built on `crate::json`.
//!
//! ```
//! use izanagi_kit::csljson::parse;
//!
//! let c = parse(
//!     br#"[{"id":"k84","type":"book","title":"TAOCP",
//!          "author":[{"family":"Knuth","given":"Donald"}],
//!          "issued":{"date-parts":[[1968]]}}]"#,
//! )
//! .unwrap();
//! assert_eq!(c.items.len(), 1);
//! let it = &c.items[0];
//! assert_eq!(it.id.as_deref(), Some("k84"));
//! assert_eq!(it.authors[0].family, "Knuth");
//! assert_eq!(it.issued_year, Some(1968));
//! ```

use crate::json::{parse as parse_json, Json};

/// A CSL name (`family` + optional `given`/`literal`).
#[derive(Debug)]
pub struct Name {
    /// `family` name (or `literal` when present).
    pub family: String,
    /// `given` may be absent; `literal` names land in `family`.
    pub given: Option<String>,
}

/// One bibliography item.
#[derive(Debug)]
pub struct Item {
    /// `id` / `citation-key` as a string.
    pub id: Option<String>,
    /// CSL `type` (`book`, `article-journal`, ...).
    pub kind: String,
    /// `title` verbatim.
    pub title: Option<String>,
    /// `author` array (each becomes a [`Name`]).
    pub authors: Vec<Name>,
    /// `issued.date-parts[0][0]` — the publication year.
    pub issued_year: Option<i64>,
    /// `container-title` (journal / book the item appears in).
    pub container: Option<String>,
}

/// A parsed CSL-JSON document.
#[derive(Debug)]
pub struct CslJson {
    /// Items in array order.
    pub items: Vec<Item>,
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

fn s(j: &Json) -> Option<&str> {
    match j {
        Json::Str(s) => Some(s),
        _ => None,
    }
}

fn i(j: &Json) -> Option<i64> {
    match j {
        Json::Int(n) => Some(*n),
        _ => None,
    }
}

fn name(j: &Json) -> Option<Name> {
    let m = obj(j)?;
    let family = m
        .get("family")
        .and_then(s)
        .or_else(|| m.get("literal").and_then(s))?
        .to_string();
    let given = m.get("given").and_then(s).map(str::to_string);
    Some(Name { family, given })
}

fn item(j: &Json) -> Option<Item> {
    let m = obj(j)?;
    let kind = m.get("type").and_then(s)?.to_string();
    let id = m.get("id").or_else(|| m.get("citation-key")).and_then(|v| {
        s(v).map(str::to_string)
            .or_else(|| i(v).map(|n| n.to_string()))
    });
    let title = m.get("title").and_then(s).map(str::to_string);
    let container = m.get("container-title").and_then(s).map(str::to_string);
    let mut authors = Vec::new();
    if let Some(Json::Arr(a)) = m.get("author") {
        for a in a {
            authors.push(name(a)?);
        }
    }
    let issued_year = get(j, "issued")
        .and_then(|v| get(v, "date-parts"))
        .and_then(|v| match v {
            Json::Arr(years) => years.first(),
            _ => None,
        })
        .and_then(|v| match v {
            Json::Arr(parts) => parts.first(),
            _ => None,
        })
        .and_then(i);
    Some(Item {
        id,
        kind,
        title,
        authors,
        issued_year,
        container,
    })
}

/// Parse a CSL-JSON document. `None` on invalid JSON or a non-array root.
pub fn parse(data: &[u8]) -> Option<CslJson> {
    let j = parse_json(data).ok()?;
    let arr = match &j {
        Json::Arr(a) => a,
        // a single bare object is accepted as a one-item list
        Json::Obj(_) => {
            return item(&j).map(|it| CslJson { items: vec![it] });
        }
        _ => return None,
    };
    let mut items = Vec::with_capacity(arr.len());
    for it in arr {
        items.push(item(it)?);
    }
    Some(CslJson { items })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let c = parse(
            br#"[{"type":"article-journal","id":"x","title":"T",
                 "author":[{"family":"Doe","given":"J"},{"literal":"Team"}],
                 "container-title":"CACM","issued":{"date-parts":[[2020,5,1]]}},
                {"type":"book","citation-key":"k"}]"#,
        )
        .unwrap();
        assert_eq!(c.items.len(), 2);
        let a = &c.items[0];
        assert_eq!(a.kind, "article-journal");
        assert_eq!(a.title.as_deref(), Some("T"));
        assert_eq!(a.container.as_deref(), Some("CACM"));
        assert_eq!(a.issued_year, Some(2020));
        assert_eq!(a.authors.len(), 2);
        assert_eq!(a.authors[0].given.as_deref(), Some("J"));
        assert_eq!(a.authors[1].family, "Team");
        let b = &c.items[1];
        assert_eq!(b.id.as_deref(), Some("k"));
        assert!(b.authors.is_empty());
        assert_eq!(b.issued_year, None);
    }

    #[test]
    fn single_object_ok() {
        let c = parse(br#"{"type":"book","title":"Solo"}"#).unwrap();
        assert_eq!(c.items.len(), 1);
        assert_eq!(c.items[0].title.as_deref(), Some("Solo"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"not json").is_none());
        assert!(parse(b"42").is_none()); // scalar root
        assert!(parse(br#"[{"title":"no type"}]"#).is_none());
        assert!(parse(br#"[{"type":"book","author":[{"given":"no family"}]}]"#).is_none());
    }
}
