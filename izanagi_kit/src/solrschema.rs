//! Solr `schema.xml` / `managed-schema` detection and census.
//!
//! Counts `<field>`/`<dynamicField>`/`<copyField>`/`<fieldType>`/`<analyzer>`/
//! `<tokenizer>`/`<filter>`/`<uniqueKey>`/`<solrQueryParser>`/`<similarity>`/
//! `<types>` elements, `name`/`type`/`indexed`/`stored`/`multiValued`/`required`
//! `default`/`sortMissing*`/`omit*`/`term*`/`positionIncrementGap` attributes,
//! `class="solr.*"` type classes, and `<!-- -->` comments.
//!
//! ```
//! let b = b"<schema name=\"x\" version=\"1.6\">\n<fields>\n<field name=\"id\" type=\"string\" indexed=\"true\" stored=\"true\"/>\n<fieldType name=\"text\" class=\"solr.TextField\"><analyzer><tokenizer class=\"solr.StandardTokenizerFactory\"/></analyzer></fieldType>\n<copyField source=\"a\" dest=\"b\"/>\n</fields>\n<uniqueKey>id</uniqueKey>\n</schema>\n";
//! assert!(izanagi_kit::solrschema::detect(b));
//! let c = izanagi_kit::solrschema::Solrschema::parse(b).unwrap();
//! assert_eq!(c.fields, 1);
//! assert_eq!(c.field_types, 1);
//! assert_eq!(c.copy_fields, 1);
//! assert_eq!(c.analyzers, 1);
//! ```

use crate::textutil::strip_xml_comments;
/// Parsed Solr schema summary.
#[derive(Debug, Clone)]
pub struct Solrschema {
    /// `<schema ...>` root element.
    pub schema: usize,
    /// `<field .../>` entries.
    pub fields: usize,
    /// `<dynamicField .../>` entries.
    pub dynamic_fields: usize,
    /// `<fieldType ...>` entries.
    pub field_types: usize,
    /// `<copyField .../>` entries.
    pub copy_fields: usize,
    /// `<uniqueKey>`/`<solrQueryParser>`/`<similarity>`/`<types>` elements.
    pub schema_keys: usize,
    /// `<analyzer>` elements.
    pub analyzers: usize,
    /// `<tokenizer>`/`<filter>`/`<charFilter>` elements.
    pub chain_parts: usize,
    /// `name`/`type`/`indexed`/`stored`/`multiValued`/`required`/`default`/`sortMissing*`/`omit*`/`term*`/`positionIncrementGap`/`docValues`/`useDocValuesAsStored`/`large`/`uninvertible` attributes.
    pub field_attrs: usize,
    /// `class="solr.*"` class references.
    pub solr_classes: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

const ATTRS: &[&str] = &[
    "name",
    "type",
    "indexed",
    "stored",
    "multiValued",
    "required",
    "default",
    "sortMissingFirst",
    "sortMissingLast",
    "omitNorms",
    "omitTermFreqAndPositions",
    "omitPositions",
    "termVectors",
    "termPositions",
    "termOffsets",
    "termPayloads",
    "positionIncrementGap",
    "docValues",
    "useDocValuesAsStored",
    "large",
    "uninvertible",
    "autoGeneratePhraseQueries",
    "enableGraphQueries",
    "maxChars",
    "catenateAll",
    "catenateNumbers",
    "catenateWords",
    "splitOnCaseChange",
    "splitOnNumerics",
    "stemEnglishPossessive",
    "preserveOriginal",
];

/// Returns `true` when the bytes look like a Solr schema.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_xml_comments(t);
    (t.contains("<schema") && (t.contains("<field") || t.contains("solr.")))
        || (t.contains("<fieldType") && t.contains("class=\"solr."))
}

fn count_tag(t: &str, tag: &str) -> usize {
    t.matches(tag).count()
}

impl Solrschema {
    /// Parses a Solr schema, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            schema: 0,
            fields: 0,
            dynamic_fields: 0,
            field_types: 0,
            copy_fields: 0,
            schema_keys: 0,
            analyzers: 0,
            chain_parts: 0,
            field_attrs: 0,
            solr_classes: 0,
            comments: 0,
        };
        c.schema += count_tag(t, "<schema");
        c.dynamic_fields += count_tag(t, "<dynamicField");
        c.fields += count_tag(t, "<field ") + count_tag(t, "<field\t") + count_tag(t, "<field\n");
        c.field_types += count_tag(t, "<fieldType");
        c.copy_fields += count_tag(t, "<copyField");
        c.schema_keys += count_tag(t, "<uniqueKey")
            + count_tag(t, "<solrQueryParser")
            + count_tag(t, "<similarity")
            + count_tag(t, "<types>");
        c.analyzers += count_tag(t, "<analyzer");
        c.chain_parts +=
            count_tag(t, "<tokenizer") + count_tag(t, "<filter") + count_tag(t, "<charFilter");
        c.solr_classes += count_tag(t, "class=\"solr.") + count_tag(t, "class='solr.");
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            for a in ATTRS {
                if tr.contains(&format!("{a}=")) {
                    c.field_attrs += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"<!-- solr -->\n<schema name=\"x\" version=\"1.6\">\n<fields>\n<field name=\"id\" type=\"string\" indexed=\"true\" stored=\"true\" required=\"true\"/>\n<field name=\"title\" type=\"text\" indexed=\"true\" stored=\"true\" multiValued=\"false\"/>\n<dynamicField name=\"*_s\" type=\"string\" indexed=\"true\" stored=\"true\"/>\n<fieldType name=\"text\" class=\"solr.TextField\" positionIncrementGap=\"100\">\n<analyzer type=\"index\">\n<tokenizer class=\"solr.StandardTokenizerFactory\"/>\n<filter class=\"solr.LowerCaseFilterFactory\"/>\n</analyzer>\n</fieldType>\n<copyField source=\"title\" dest=\"_text_\"/>\n</fields>\n<uniqueKey>id</uniqueKey>\n</schema>\n";

    #[test]
    fn parses_solrschema() {
        let c = Solrschema::parse(CONF).unwrap();
        assert_eq!(c.schema, 1);
        assert_eq!(c.fields, 2);
        assert_eq!(c.dynamic_fields, 1);
        assert_eq!(c.field_types, 1);
        assert_eq!(c.copy_fields, 1);
        assert_eq!(c.analyzers, 1);
        assert_eq!(c.chain_parts, 2);
        assert!(c.field_attrs >= 8);
        assert_eq!(c.solr_classes, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_solrschema() {
        assert!(!detect(b"<html><body/></html>"));
        assert!(Solrschema::parse(b"x").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_xml_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_xml_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
