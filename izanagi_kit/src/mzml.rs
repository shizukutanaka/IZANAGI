//! mzML — mass-spectrometry XML (HUPO-PSI).
//!
//! `<mzML ...version="1.1.0">` wraps `<cvList>` and
//! `<spectrumList count="n">` of `<spectrum index id>` elements; each
//! spectrum's `<binaryDataArrayList count>` gives its array count.
//!
//! ```
//! use izanagi_kit::mzml::parse;
//!
//! let x = parse(br#"<mzML xmlns="x" version="1.1.0"><run><spectrumList count="1" defaultDataProcessingRef="d"><spectrum index="0" id="scan=1" defaultArrayLength="4"><binaryDataArrayList count="2"/></spectrum></spectrumList></run></mzML>"#).unwrap();
//! assert_eq!(x.version, "1.1.0");
//! assert_eq!(x.spectra[0].id, "scan=1");
//! assert_eq!(x.spectra[0].arrays, 2);
//! ```

/// One `<spectrum>` element.
#[derive(Clone, Debug)]
pub struct Spectrum {
    /// `index` attribute.
    pub index: u64,
    /// `id` attribute (e.g. `scan=123`).
    pub id: String,
    /// `defaultArrayLength`.
    pub default_array_length: Option<u64>,
    /// `binaryDataArrayList count`.
    pub arrays: u64,
}

/// A parsed mzML document.
#[derive(Clone, Debug)]
pub struct Mzml {
    /// `version` attribute of `<mzML>`.
    pub version: String,
    /// `spectrumList count` when present.
    pub declared: Option<u64>,
    /// Parsed `<spectrum>` elements in document order.
    pub spectra: Vec<Spectrum>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let mut pos = 0;
    while let Some(rel) = tag[pos..].find(&pat) {
        let i = pos + rel;
        let prev = tag.as_bytes().get(i.wrapping_sub(1));
        if i > 0 && prev.map(|b| b.is_ascii_alphanumeric() || *b == b'-') == Some(true) {
            pos = i + pat.len();
            continue;
        }
        let start = i + pat.len();
        let end = tag[start..].find('"')? + start;
        return Some(tag[start..end].to_string());
    }
    None
}

fn u64attr(tag: &str, key: &str) -> Option<u64> {
    attr(tag, key)?.parse().ok()
}

/// Parse an mzML document. `None` without `<mzML`/version or on a
/// malformed `<spectrum` tag.
pub fn parse(d: &[u8]) -> Option<Mzml> {
    let text = std::str::from_utf8(d).ok()?;
    let open = text.find("<mzML")?;
    let gt = text[open..].find('>')? + open;
    let version = attr(&text[open..gt], "version")?;
    let mut declared = None;
    if let Some(sl) = text[gt..].find("<spectrumList") {
        let sgt = text[gt + sl..].find('>')? + gt + sl;
        declared = u64attr(&text[gt + sl..sgt], "count");
    }
    let mut spectra = Vec::new();
    let mut i = gt;
    while let Some(rel) = text[i..].find("<spectrum") {
        let s = i + rel;
        if text[s..].starts_with("<spectrumList") {
            i = s + 13;
            continue;
        }
        let e = text[s..].find('>')? + s;
        let head = &text[s..e];
        let index = u64attr(head, "index")?;
        let id = attr(head, "id")?;
        let default_array_length = u64attr(head, "defaultArrayLength");
        // find the spectrum's array list count before </spectrum>
        let send = text[s..].find("</spectrum>").map(|p| s + p);
        let mut arrays = 0;
        if let Some(send) = send {
            if let Some(bd) = text[s..send].find("<binaryDataArrayList") {
                let bgt = text[s + bd..send].find('>')? + s + bd;
                arrays = u64attr(&text[s + bd..bgt], "count").unwrap_or(0);
            }
            i = send;
        } else {
            i = e + 1;
        }
        spectra.push(Spectrum {
            index,
            id,
            default_array_length,
            arrays,
        });
    }
    Some(Mzml {
        version,
        declared,
        spectra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let x = parse(br#"<mzML version="1.1.0"><spectrumList count="2"><spectrum index="0" id="a" defaultArrayLength="9"><binaryDataArrayList count="2"/></spectrum><spectrum index="1" id="b"/></spectrumList></mzML>"#).unwrap();
        assert_eq!(x.declared, Some(2));
        assert_eq!(x.spectra.len(), 2);
        assert_eq!(x.spectra[0].arrays, 2);
        assert_eq!(x.spectra[1].id, "b");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<mzML></mzML>").is_none());
        assert!(parse(b"<mzML version=\"1\"><spectrum /></mzML>").is_none());
    }
}
