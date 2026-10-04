//! KDE Kate highlighting definition (`.xml` `syntax/*.xml`) parser.
//!
//! Detects Kate syntax files by their `<language name="..." section="...">`
//! root with `<highlighting><contexts><context>` plus matcher tags
//! (`<keyword>`/`<RegExpr>`/`<DetectChar>`/`<Detect2Chars>`/`<StringDetect>`/
//! `<AnyChar>`/`<Int>`/`<Float>`/`<HlCOct>`/`<HlCHex>`/`<HlCStringChar>`/
//! `<HlCChar>`/`<RangeDetect>`/`<LineContinue>`/`<IncludeRules>`/
//! `<DetectSpaces>`/`<DetectIdentifier>`/`<WordDetect>`) and `<itemData>`
//! attributes.
//!
//! ```
//! let b = b"<language name=\"Demo\" section=\"Other\">\n<highlighting><contexts><context name=\"ctx\" attribute=\"a\">\n<RegExpr String=\"[0-9]+\" attribute=\"num\"/>\n<keyword String=\"for\" insensitive=\"true\"/>\n</context></contexts></highlighting>\n<itemDatas><itemData name=\"a\" defStyleNum=\"dsNormal\"/></itemDatas>\n</language>\n";
//! assert!(izanagi_kit::katesyntax::detect(b));
//! let c = izanagi_kit::katesyntax::Katesyn::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed Kate highlighting summary.
#[derive(Debug, Clone)]
pub struct Katesyn {
    /// Recognized tag occurrences.
    pub keys: usize,
    /// Structure tags (`<language`/`<highlighting`/`<contexts`/`<context `/`<itemDatas`/`<itemData`/`<list`/`<item>`/`<comments`/`comment name=`/`abstract=`).
    pub struct_keys: usize,
    /// Matcher tags (`<keyword`/`<RegExpr`/`<DetectChar`/`<Detect2Chars`/`<StringDetect`/`<AnyChar`/`<WordDetect`/`<Int>`/`<Float>`/`<HlCOct>`/`<HlCHex>`/`<HlCStringChar>`/`<HlCChar>`/`<RangeDetect`/`<LineContinue>`/`<IncludeRules`/`<DetectSpaces>`/`<DetectIdentifier>`/`<keyword `).
    pub match_keys: usize,
    /// Attribute keys (`attribute=`/`context=`/`String=`/`char=`/`char1=`/`insensitive=`/`defStyleNum=`/`firstNonSpace=`/`lookAhead=`/`dynamic=`/`fallthrough=`/`lineEndContext=`/`<general>`/`<encoding>`).
    pub attr_keys: usize,
    /// `<!-- -->`/`#` comment lines.
    pub comments: usize,
}

/// Structure tags.
const STRUCT_KEYS: &[&str] = &[
    "<language",
    "<highlighting>",
    "<contexts>",
    "<context ",
    "<itemDatas>",
    "<itemData",
    "<list ",
    "<item>",
    "<comments>",
];

/// Matcher tags.
const MATCH_KEYS: &[&str] = &[
    "<keyword ",
    "<RegExpr",
    "<DetectChar",
    "<Detect2Chars",
    "<StringDetect",
    "<AnyChar",
    "<WordDetect",
    "<Int>",
    "<Float>",
    "<HlCOct",
    "<HlCHex",
    "<HlCStringChar",
    "<HlCChar",
    "<RangeDetect",
    "<LineContinue>",
    "<IncludeRules",
    "<DetectSpaces>",
    "<DetectIdentifier>",
];

/// Attribute keys.
const ATTR_KEYS: &[&str] = &[
    "attribute=",
    "context=",
    "String=",
    "char=",
    "char1=",
    "insensitive=",
    "defStyleNum=",
    "firstNonSpace=",
    "lookAhead=",
    "dynamic=",
    "fallthrough=",
    "lineEndContext=",
    "<general>",
    "<encoding>",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "<language",
    "<highlighting>",
    "<contexts>",
    "<RegExpr",
    "<DetectChar",
    "<keyword ",
    "<itemData",
    "defStyleNum=",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Kate highlighting file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Katesyn {
    /// Count tag categories in a Kate highlighting file. Returns `None`
    /// when the input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            struct_keys: 0,
            match_keys: 0,
            attr_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in STRUCT_KEYS {
            c.struct_keys += t.matches(k).count();
        }
        for k in MATCH_KEYS {
            c.match_keys += t.matches(k).count();
        }
        for k in ATTR_KEYS {
            c.attr_keys += t.matches(k).count();
        }
        c.keys = c.struct_keys + c.match_keys + c.attr_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"<!-- kate -->\n<language name=\"Demo\" section=\"Other\">\n<highlighting><list name=\"kw\"><item>for</item></list><contexts><context name=\"ctx\" attribute=\"a\" lineEndContext=\"#stay\">\n<keyword String=\"kw\" insensitive=\"true\"/>\n<RegExpr String=\"[0-9]+\" attribute=\"num\"/>\n<DetectChar char=\"{\"/>\n</context></contexts></highlighting>\n<itemDatas><itemData name=\"a\" defStyleNum=\"dsNormal\"/></itemDatas>\n</language>\n";
        assert!(detect(b));
        let c = Katesyn::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.struct_keys >= 3);
        assert!(c.match_keys >= 2);
        assert!(c.attr_keys >= 4);
        assert!(c.keys >= 9);
    }

    #[test]
    fn rejects_html() {
        assert!(!detect(b"<html><body>hi</body></html>"));
        assert!(Katesyn::parse(b"a = b\n").is_none());
    }
}
