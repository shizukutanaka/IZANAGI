//! OpenAPI / Swagger API description — detects the `openapi:` YAML key or
//! `"openapi":` JSON field (v3.x) and `swagger:`/`"2.0"` (v2), then censuses
//! path items (`/…` keys under `paths`), `operationId` operations, `components`/
//! `definitions` schema blocks, and `info.title`.
//!
//! ```
//! let d = b"openapi: 3\x2e0\x2e1\ninfo:\n  title: Demo API\n  version: 1\x2e0\npaths:\n  /pets:\n    get:\n      operationId: listPets\n  /pets/{id}:\n    post:\n      operationId: createPet\n";
//! let a = izanagi_kit::openapi::parse(d).unwrap();
//! assert_eq!(a.version, "3\x2e0\x2e1");
//! assert_eq!(a.paths, 2);
//! assert_eq!(a.operations, 2);
//! assert_eq!(a.title.as_deref(), Some("Demo API"));
//! assert!(izanagi_kit::openapi::detect(d));
//! ```

/// A censused OpenAPI/Swagger document.
#[derive(Debug)]
pub struct OpenApi {
    /// Declared spec version (`3.x.y` or `2.0`).
    pub version: String,
    /// `2.0` Swagger style.
    pub is_swagger: bool,
    /// Path-item keys under `paths:` (`"/…"` JSON keys or `  /…:` YAML keys).
    pub paths: u32,
    /// `operationId:` / `"operationId"` occurrences.
    pub operations: u32,
    /// `components:`/`definitions:` schema object keys (approximate census via
    /// two-indent `name:` keys under the schemas block is impossible without a
    /// full parser; count the block's presence per object).
    pub schema_blocks: u32,
    /// `$ref` occurrences.
    pub refs: u32,
    /// `info.title` text, when present.
    pub title: Option<String>,
    /// `servers:`/`host:` presence.
    pub has_servers: bool,
}

fn count(s: &str, pat: &str) -> u32 {
    u32::try_from(s.matches(pat).count()).unwrap_or(u32::MAX)
}

fn after_key<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let i = s.find(key)? + key.len();
    let rest = s[i..].trim_start_matches([' ', ':', '"']);
    let end = rest.find([',', '\n', '}']).unwrap_or(rest.len());
    let v = rest[..end].trim().trim_matches('"');
    (!v.is_empty()).then_some(v)
}

fn yaml_title(s: &str) -> Option<String> {
    for line in s.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("title:") {
            let v = v.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn yaml_paths(s: &str) -> u32 {
    let mut n = 0u32;
    let mut in_paths = false;
    for line in s.lines() {
        if line.trim_start().starts_with("paths:") {
            in_paths = true;
            continue;
        }
        if in_paths {
            let indent = line.len() - line.trim_start().len();
            if indent == 0 && !line.trim().is_empty() {
                break;
            }
            if line.trim_start().starts_with('/') {
                n += 1;
            }
        }
    }
    n
}

/// `openapi:`/`"openapi"` or `swagger:`/`"swagger"` key.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("openapi:")
        || s.contains("\"openapi\"")
        || s.contains("swagger:")
        || s.contains("\"swagger\"")
}

/// Parses the document; `None` without an OpenAPI/Swagger marker.
#[must_use]
pub fn parse(b: &[u8]) -> Option<OpenApi> {
    let s = core::str::from_utf8(b).ok()?;
    let is_swagger = s.contains("swagger:") || s.contains("\"swagger\"");
    let version = after_key(s, "openapi:")
        .or_else(|| after_key(s, "\"openapi\":"))
        .or_else(|| after_key(s, "swagger:"))
        .or_else(|| after_key(s, "\"swagger\":"))
        .unwrap_or("")
        .to_string();
    if version.is_empty() {
        return None;
    }
    let is_json = s.contains("\"openapi\"");
    let paths = if is_json {
        count(s, "\"/")
    } else {
        yaml_paths(s)
    };
    Some(OpenApi {
        version,
        is_swagger,
        paths,
        operations: count(s, "operationId"),
        schema_blocks: count(s, "components:")
            + count(s, "\"components\"")
            + count(s, "definitions:")
            + count(s, "\"definitions\""),
        refs: count(s, "$ref"),
        title: yaml_title(s).or_else(|| after_key(s, "\"title\"").map(str::to_string)),
        has_servers: s.contains("servers:") || s.contains("\"servers\"") || s.contains("host:"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &[u8] = b"openapi: 3\x2e0\x2e1\ninfo:\n  title: Demo API\npaths:\n  /pets:\n    get:\n      operationId: listPets\n  /pets/{id}:\n    post:\n      operationId: createPet\n";
    const JSON: &[u8] = b"{\"openapi\":\"3\x2e1\x2e0\",\"info\":{\"title\":\"J\"},\"paths\":{\"/a\":{\"get\":{\"operationId\":\"g\"}},\"/b\":{}},\"components\":{\"schemas\":{\"Pet\":{}}}}";

    #[test]
    fn detect_works() {
        assert!(detect(YAML));
        assert!(detect(JSON));
        assert!(detect(b"swagger: '2\x2e0'\npaths: {}\n"));
        assert!(!detect(b"api: 1"));
    }

    #[test]
    fn parses_yaml() {
        let a = parse(YAML).unwrap();
        assert_eq!(a.version, "3\x2e0\x2e1");
        assert!(!a.is_swagger);
        assert_eq!(a.paths, 2);
        assert_eq!(a.operations, 2);
        assert_eq!(a.title.as_deref(), Some("Demo API"));
    }

    #[test]
    fn parses_json() {
        let a = parse(JSON).unwrap();
        assert_eq!(a.version, "3\x2e1\x2e0");
        assert_eq!(a.paths, 2);
        assert_eq!(a.operations, 1);
        assert_eq!(a.schema_blocks, 1);
        assert_eq!(a.title.as_deref(), Some("J"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"random: doc").is_none());
    }
}
