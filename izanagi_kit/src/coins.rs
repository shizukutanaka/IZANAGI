//! Minimal reader for COinS (ContextObjects in Spans): the OpenURL
//! key/value set hidden in `<span class="Z3988" title="ctx_ver=...&rft...">`.
//!
//! `parse` accepts either a full HTML fragment (finds the `Z3988` span and
//! reads its `title` attribute) or the raw `title` value. Values are
//! percent-decoded; keys keep their `rft.`/`ctx_` prefixes.
//!
//! ```
//! use izanagi_kit::coins::parse;
//!
//! let c = parse(b"ctx_ver=Z39.88-2004&rft_val_fmt=info%3Aofi%2Ffmt%3Akev%3Amtx%3Abook\
//!     &rft.btitle=TAOCP&rft.au=Knuth").unwrap();
//! assert_eq!(c.get("ctx_ver").unwrap(), "Z39.88-2004");
//! assert_eq!(c.get("rft.btitle").unwrap(), "TAOCP");
//! ```

/// A decoded COinS context object.
#[derive(Debug)]
pub struct Coins {
    /// `ctx_ver` (usually `Z39.88-2004`).
    pub ctx_ver: String,
    /// `rft_val_fmt` — the metadata format URI.
    pub format: Option<String>,
    /// All `(key, value)` pairs in order (decoded).
    pub fields: Vec<(String, String)>,
}

impl Coins {
    /// First value for `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Percent-decode (`%XX`, `+` = space). `None` on a bad escape.
fn unquote(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let hi = hex(*b.get(i + 1)?)?;
                let lo = hex(*b.get(i + 2)?)?;
                out.push(hi * 16 + lo);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn query(src: &str) -> Option<Vec<(String, String)>> {
    let mut fields = Vec::new();
    for pair in src.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        fields.push((unquote(k)?, unquote(v)?));
    }
    Some(fields)
}

/// Parse a COinS object from a `title` value or an HTML fragment.
/// `None` when no `ctx_ver` field is present.
pub fn parse(data: &[u8]) -> Option<Coins> {
    let src = std::str::from_utf8(data).ok()?;
    // If this looks like markup, pull the title attribute of a Z3988 span.
    let q = if src.contains("Z3988") {
        let at = src.find("Z3988")?;
        let after = &src[at + 5..];
        // title="..." or title='...' after the class marker
        let t = after.find("title=\"")?;
        let from = t + "title=\"".len();
        let end = after[from..].find('"')? + from;
        &after[from..end]
    } else {
        src.trim()
    };
    let fields = query(q)?;
    let ctx_ver = fields
        .iter()
        .find(|(k, _)| k == "ctx_ver")
        .map(|(_, v)| v.clone())?;
    let format = fields
        .iter()
        .find(|(k, _)| k == "rft_val_fmt")
        .map(|(_, v)| v.clone());
    Some(Coins {
        ctx_ver,
        format,
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_raw() {
        let c = parse(b"ctx_ver=Z39.88-2004&rft_val_fmt=info%3Aofi%2Ffmt%3Akev%3Amtx%3Ajournal&rft.atitle=T&rft.au=Doe").unwrap();
        assert_eq!(c.ctx_ver, "Z39.88-2004");
        assert_eq!(c.format.as_deref(), Some("info:ofi/fmt:kev:mtx:journal"));
        assert_eq!(c.get("rft.atitle"), Some("T"));
        assert_eq!(c.get("rft.au"), Some("Doe"));
        assert_eq!(c.get("nope"), None);
    }

    #[test]
    fn parses_span() {
        let html = b"<span class=\"Z3988\" title=\"ctx_ver=Z39.88-2004&rft.btitle=X+Y\"></span>";
        let c = parse(html).unwrap();
        assert_eq!(c.get("rft.btitle"), Some("X Y")); // '+' decodes to space
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none()); // no ctx_ver
        assert!(parse(b"rft.title=X").is_none());
        assert!(parse(b"ctx_ver=%GG").is_none()); // bad escape
        assert!(parse(b"ctx_ver=%ZZ-2004").is_none());
    }
}
