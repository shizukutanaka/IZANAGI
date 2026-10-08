//! Solr `solrconfig.xml` detection and census.
//!
//! Counts `<requestHandler>`/`<searchComponent>`/`<queryConverter>`/
//! `<updateHandler>`/`<requestDispatcher>`/`<queryResponseWriter>`/
//! `<admin>`/`<luceneMatchVersion>`/`<dataDir>`/`<directoryFactory>`/
//! `<codecFactory>`/`<schemaFactory>`/`<updateLog>`/`<indexConfig>`/
//! `<mergePolicy>`/`<termIndex>`/`<slowQueryThresholdMillis>`/`<listener>`/
//! `<highlighting>`/`<spellcheck>`/`<termsComponent>`/`<suggester>`/
//! `<initParams>`/`<cache>`/`<max*`/`<arr>`/`<lst>`/`<str>`/`<int>`/`<bool>`/
//! `<float>`/`<long>`/`<double>` elements, `class=`/`name=`/`default=`/`qt=`/
//! `startup=` attributes, `${}` property refs, and `<!-- -->` comments.
//!
//! ```
//! let b = b"<config>\n<luceneMatchVersion>9.0</luceneMatchVersion>\n<dataDir>${solr.data.dir:}</dataDir>\n<updateHandler>\n<updateLog><str name=\"dir\">x</str></updateLog>\n</updateHandler>\n<query>\n<maxBooleanClauses>1024</maxBooleanClauses>\n</query>\n<requestHandler name=\"/select\" class=\"solr.SearchHandler\">\n<lst name=\"defaults\"><str name=\"df\">text</str></lst>\n</requestHandler>\n</config>\n";
//! assert!(izanagi_kit::solrconfig::detect(b));
//! let c = izanagi_kit::solrconfig::Solrconfig::parse(b).unwrap();
//! assert_eq!(c.handlers, 1);
//! assert_eq!(c.update_handler, 2);
//! assert!(c.typed_elems >= 2);
//! ```

use crate::textutil::strip_xml_comments;
/// Parsed solrconfig.xml summary.
#[derive(Debug, Clone)]
pub struct Solrconfig {
    /// `<config>`/`<luceneMatchVersion>`/`<dataDir>` top-level elements.
    pub core_elems: usize,
    /// `<requestHandler>` entries.
    pub handlers: usize,
    /// `<searchComponent>`/`<queryConverter>`/`<termsComponent>`/`<highlighting>`/`<spellcheck>`/`<suggester>`/`firstSearcher`/`newSearcher` component entries.
    pub components: usize,
    /// `<updateHandler>`/`<updateLog>`/`<indexConfig>`/`<mergePolicy>`/`<mergeScheduler>`/`<mergedSegmentWarmer>`/`<ramBufferSizeMB>`/`<maxIndexingThreads>`/`<commit>`/`<autoCommit>`/`<autoSoftCommit>` update/index entries.
    pub update_handler: usize,
    /// `<requestDispatcher>`/`<httpCaching>`/`<requestParsers>`/`<queryResponseWriter>`/`<admin>`/`<listener>`/`<shardHandlerFactory>` misc entries.
    pub misc_elems: usize,
    /// `<cache>`/`<filterCache>`/`<queryResultCache>`/`<documentCache>`/`<fieldValueCache>`/`<slowQueryThresholdMillis>`/`<maxBooleanClauses>`/`<maxWarmingSearchers>`/`<useColdSearcher>` query/cache entries.
    pub caches: usize,
    /// `<initParams>`/`<lst>`/`<arr>`/`<str>`/`<int>`/`<bool>`/`<float>`/`<long>`/`<double>` typed elements.
    pub typed_elems: usize,
    /// `class="*"` class refs.
    pub classes: usize,
    /// `${...}` property references.
    pub prop_refs: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

const CORE: &[&str] = &[
    "<config",
    "<luceneMatchVersion",
    "<dataDir",
    "<directoryFactory",
    "<codecFactory",
    "<schemaFactory",
];
const COMP: &[&str] = &[
    "<searchComponent",
    "<queryConverter",
    "<termsComponent",
    "<highlighting",
    "<spellcheck",
    "<suggester",
    "<firstSearcher",
    "<newSearcher",
];
const UPD: &[&str] = &[
    "<updateHandler",
    "<updateLog",
    "<indexConfig",
    "<mergePolicy",
    "<mergeScheduler",
    "<mergedSegmentWarmer",
    "<ramBufferSizeMB",
    "<maxIndexingThreads",
    "<commit",
    "<autoCommit",
    "<autoSoftCommit",
    "<openSearcher",
];
const MISC: &[&str] = &[
    "<requestDispatcher",
    "<httpCaching",
    "<requestParsers",
    "<queryResponseWriter",
    "<admin",
    "<listener",
    "<shardHandlerFactory",
    "<startupInfo",
];
const CACHE: &[&str] = &[
    "<cache",
    "<filterCache",
    "<queryResultCache",
    "<documentCache",
    "<fieldValueCache",
    "<slowQueryThresholdMillis",
    "<maxBooleanClauses",
    "<maxWarmingSearchers",
    "<useColdSearcher",
    "<enableLazyFieldLoading",
];
const TYPED: &[&str] = &[
    "<initParams",
    "<lst",
    "<arr",
    "<str",
    "<int",
    "<bool",
    "<float",
    "<long",
    "<double",
];

/// Returns `true` when the bytes look like a solrconfig.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_xml_comments(t);
    (t.contains("<config")
        && (t.contains("requestHandler")
            || t.contains("solr.")
            || t.contains("luceneMatchVersion")))
        || (t.contains("<requestHandler") && t.contains("solr."))
}

impl Solrconfig {
    /// Parses a solrconfig.xml, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            core_elems: 0,
            handlers: 0,
            components: 0,
            update_handler: 0,
            misc_elems: 0,
            caches: 0,
            typed_elems: 0,
            classes: 0,
            prop_refs: 0,
            comments: 0,
        };
        for e in CORE {
            c.core_elems += t.matches(e).count();
        }
        c.handlers += t.matches("<requestHandler").count();
        for e in COMP {
            c.components += t.matches(e).count();
        }
        for e in UPD {
            c.update_handler += t.matches(e).count();
        }
        for e in MISC {
            c.misc_elems += t.matches(e).count();
        }
        for e in CACHE {
            c.caches += t.matches(e).count();
        }
        for e in TYPED {
            c.typed_elems += t.matches(e).count();
        }
        c.classes += t.matches("class=\"").count() + t.matches("class='").count();
        c.prop_refs += t.matches("${").count();
        for l in t.lines() {
            if l.trim_start().starts_with("<!--") {
                c.comments += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"<!-- cfg -->\n<config>\n<luceneMatchVersion>9.0</luceneMatchVersion>\n<dataDir>${solr.data.dir:}</dataDir>\n<updateHandler>\n<updateLog><str name=\"dir\">x</str></updateLog>\n<autoCommit><int name=\"maxTime\">15000</int></autoCommit>\n</updateHandler>\n<query>\n<maxBooleanClauses>1024</maxBooleanClauses>\n<filterCache size=\"512\"/>\n</query>\n<requestHandler name=\"/select\" class=\"solr.SearchHandler\">\n<lst name=\"defaults\"><str name=\"df\">text</str><bool name=\"hl\">true</bool></lst>\n</requestHandler>\n<admin><str name=\"defaultQuery\">*:*</str></admin>\n</config>\n";

    #[test]
    fn parses_solrconfig() {
        let c = Solrconfig::parse(CONF).unwrap();
        assert_eq!(c.core_elems, 3);
        assert_eq!(c.handlers, 1);
        assert_eq!(c.update_handler, 3);
        assert_eq!(c.caches, 2);
        assert_eq!(c.misc_elems, 1);
        assert!(c.typed_elems >= 5);
        assert_eq!(c.classes, 1);
        assert_eq!(c.prop_refs, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_solrconfig() {
        assert!(!detect(b"<xml/>"));
        assert!(Solrconfig::parse(b"x").is_none());
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
