//! Meilisearch index settings JSON detection and census.
//!
//! Counts `searchableAttributes`/`displayedAttributes`/`filterableAttributes`/
//! `sortableAttributes`/`rankingRules`/`stopWords`/`synonyms`/`distinctAttribute`/
//! `typoTolerance`/`minWordSizeForTypos`/`typo`/`pagination`/`maxTotalHits`/
//! `faceting`/`maxValuesPerFacet`/`sortFacetValuesBy`/`proximityPrecision`/
//! `embedders`/`searchCutoffMs`/`localizedAttributes`/`facetSearch`/`prefixSearch`/
//! `nonSeparatorTokens`/`separatorTokens`/`dictionary`/`proximityPrecision`/
//! `disableOnWords`/`disableOnAttributes`/`disableOnExactness`/`disableOnNumbers`/
//! `oneTypo`/`twoTypos`/`rankingScoreThreshold`/`visible`/`name`/`default`/
//! `apiKey`/`taskUid`/`indexUid`/`status`/`type`/`enqueuedAt`/`startedAt`/
//! `finishedAt`/`duration`/`details`/`error`/`canceledBy`/`uid`/`primaryKey`/
//! `createdAt`/`updatedAt`/`fieldDistribution`/`isIndexing`/`numberOfDocuments`/
//! `rawDocumentDbSize`/`avgDocumentSize`/`numberOfEmbeddings`/`numberOfEmbeddedDocuments`/
//! `fieldSize`/`indexes`/`results`/`limit`/`offset`/`total`/`stats`/`databaseSize`/
//! `lastUpdate`/`entries`/`env`/`commitSha`/`buildDate`/`pkgVersion`/`instanceUid`/
//! `fromSnapshot`/`snapshotProcessing`/`version`/`hits`/`hitsPerPage`/`page`/
//! `totalPages`/`totalHits`/`estimatedTotalHits`/`processingTimeMs`/`query`/
//! `params`/`facetsStats`/`facetDistribution`/`semanticHitCount`/`vector`/
//! `hybrid`/`semanticRatio`/`retrieveVectors`/`media`/`context`/`matchingStrategy`/
//! `attributesToHighlight`/`attributesToCrop`/`cropLength`/`cropMarker`/
//! `highlightPreTag`/`highlightPostTag`/`showMatchesPosition`/`matchingStrategy`/
//! `attributesToRetrieve`/`attributesToSearchOn`/`filter`/`sort`/`q`/`meilisearch`/
//! `MEILI_*` env vars, and `#`/`//` comment lines.
//!
//! ```
//! let b = b"{\"rankingRules\":[\"words\",\"typo\",\"proximity\",\"attribute\",\"sort\",\"exactness\"],\"searchableAttributes\":[\"title\",\"overview\"],\"filterableAttributes\":[\"genre\"],\"sortableAttributes\":[\"release_date\"],\"stopWords\":[\"the\",\"a\"],\"distinctAttribute\":\"movie_id\",\"typoTolerance\":{\"enabled\":true,\"minWordSizeForTypos\":{\"oneTypo\":4,\"twoTypos\":8}},\"pagination\":{\"maxTotalHits\":1000}}\n";
//! assert!(izanagi_kit::meili::detect(b));
//! let c = izanagi_kit::meili::Meili::parse(b).unwrap();
//! assert_eq!(c.attr_keys, 4);
//! assert_eq!(c.ranking_rules, 6);
//! assert!(c.misc_keys >= 3);
//! ```

/// Parsed Meilisearch settings summary.
#[derive(Debug, Clone)]
pub struct Meili {
    /// `"searchableAttributes"`/`"displayedAttributes"`/`"filterableAttributes"`/`"sortableAttributes"`/`"distinctAttribute"`/`"localizedAttributes"`/`"attributesToRetrieve"`/`"attributesToHighlight"`/`"attributesToCrop"`/`"attributesToSearchOn"`/`"nonSeparatorTokens"`/`"separatorTokens"`/`"dictionary"`/`"proximityAttribute"` attribute list keys.
    pub attr_keys: usize,
    /// values inside `rankingRules` (`words`/`typo`/`proximity`/`attribute`/`sort`/`exactness`/`desc(field)`/`asc(field)`/`score`/`random`).
    pub ranking_rules: usize,
    /// `"typoTolerance"`/`"minWordSizeForTypos"`/`"disableOnWords"`/`"disableOnAttributes"`/`"disableOnExactness"`/`"disableOnNumbers"`/`"oneTypo"`/`"twoTypos"`/`"enabled"`/`"typo"`/`"pagination"`/`"maxTotalHits"`/`"faceting"`/`"maxValuesPerFacet"`/`"sortFacetValuesBy"`/`"proximityPrecision"`/`"embedders"`/`"searchCutoffMs"`/`"facetSearch"`/`"prefixSearch"`/`"rankingScoreThreshold"`/`"stopWords"`/`"synonyms"`/`"primaryKey"`/`"uid"`/`"indexUid"`/`"taskUid"`/`"status"`/`"type"`/`"enqueuedAt"`/`"startedAt"`/`"finishedAt"`/`"duration"`/`"details"`/`"error"`/`"canceledBy"`/`"createdAt"`/`"updatedAt"`/`"fieldDistribution"`/`"isIndexing"`/`"numberOfDocuments"`/`"rawDocumentDbSize"`/`"avgDocumentSize"`/`"numberOfEmbeddings"`/`"numberOfEmbeddedDocuments"`/`"indexes"`/`"results"`/`"limit"`/`"offset"`/`"total"`/`"stats"`/`"databaseSize"`/`"lastUpdate"`/`"entries"`/`"env"`/`"commitSha"`/`"buildDate"`/`"pkgVersion"`/`"instanceUid"`/`"fromSnapshot"`/`"snapshotProcessing"`/`"version"`/`"hits"`/`"hitsPerPage"`/`"page"`/`"totalPages"`/`"totalHits"`/`"estimatedTotalHits"`/`"processingTimeMs"`/`"query"`/`"params"`/`"facetsStats"`/`"facetDistribution"`/`"semanticHitCount"`/`"vector"`/`"hybrid"`/`"semanticRatio"`/`"retrieveVectors"`/`"media"`/`"context"`/`"matchingStrategy"`/`"cropLength"`/`"cropMarker"`/`"highlightPreTag"`/`"highlightPostTag"`/`"showMatchesPosition"`/`"filter"`/`"sort"`/`"q"`/`"visible"`/`"name"`/`"default"`/`"apiKey"`/`"source"`/`"url"`/`"revision"`/`"documentTemplate"`/`"request"`/`"dimensions"`/`"documentTemplateMaxBytes"`/`"binaryQuantized"`/`"model"`/`"apiKey"`/`"headers"` misc settings keys.
    pub misc_keys: usize,
    /// `MEILI_*` environment variable names.
    pub env_vars: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const ATTRS: &[&str] = &[
    "\"searchableAttributes\"",
    "\"displayedAttributes\"",
    "\"filterableAttributes\"",
    "\"sortableAttributes\"",
    "\"distinctAttribute\"",
    "\"localizedAttributes\"",
    "\"attributesToRetrieve\"",
    "\"attributesToHighlight\"",
    "\"attributesToCrop\"",
    "\"attributesToSearchOn\"",
    "\"nonSeparatorTokens\"",
    "\"separatorTokens\"",
    "\"dictionary\"",
];

const MISC: &[&str] = &[
    "\"typoTolerance\"",
    "\"minWordSizeForTypos\"",
    "\"disableOnWords\"",
    "\"disableOnAttributes\"",
    "\"disableOnExactness\"",
    "\"disableOnNumbers\"",
    "\"oneTypo\"",
    "\"twoTypos\"",
    "\"enabled\"",
    "\"typo\"",
    "\"pagination\"",
    "\"maxTotalHits\"",
    "\"faceting\"",
    "\"maxValuesPerFacet\"",
    "\"sortFacetValuesBy\"",
    "\"proximityPrecision\"",
    "\"embedders\"",
    "\"searchCutoffMs\"",
    "\"facetSearch\"",
    "\"prefixSearch\"",
    "\"rankingScoreThreshold\"",
    "\"stopWords\"",
    "\"synonyms\"",
    "\"primaryKey\"",
    "\"uid\"",
    "\"indexUid\"",
    "\"taskUid\"",
    "\"status\"",
    "\"type\"",
    "\"enqueuedAt\"",
    "\"startedAt\"",
    "\"finishedAt\"",
    "\"duration\"",
    "\"details\"",
    "\"error\"",
    "\"canceledBy\"",
    "\"createdAt\"",
    "\"updatedAt\"",
    "\"fieldDistribution\"",
    "\"isIndexing\"",
    "\"numberOfDocuments\"",
    "\"rawDocumentDbSize\"",
    "\"avgDocumentSize\"",
    "\"numberOfEmbeddings\"",
    "\"numberOfEmbeddedDocuments\"",
    "\"indexes\"",
    "\"results\"",
    "\"limit\"",
    "\"offset\"",
    "\"total\"",
    "\"stats\"",
    "\"databaseSize\"",
    "\"lastUpdate\"",
    "\"entries\"",
    "\"env\"",
    "\"commitSha\"",
    "\"buildDate\"",
    "\"pkgVersion\"",
    "\"instanceUid\"",
    "\"fromSnapshot\"",
    "\"snapshotProcessing\"",
    "\"version\"",
    "\"hits\"",
    "\"hitsPerPage\"",
    "\"page\"",
    "\"totalPages\"",
    "\"totalHits\"",
    "\"estimatedTotalHits\"",
    "\"processingTimeMs\"",
    "\"query\"",
    "\"params\"",
    "\"facetsStats\"",
    "\"facetDistribution\"",
    "\"semanticHitCount\"",
    "\"vector\"",
    "\"hybrid\"",
    "\"semanticRatio\"",
    "\"retrieveVectors\"",
    "\"media\"",
    "\"context\"",
    "\"matchingStrategy\"",
    "\"cropLength\"",
    "\"cropMarker\"",
    "\"highlightPreTag\"",
    "\"highlightPostTag\"",
    "\"showMatchesPosition\"",
    "\"filter\"",
    "\"sort\"",
    "\"q\"",
    "\"visible\"",
    "\"name\"",
    "\"default\"",
    "\"apiKey\"",
    "\"source\"",
    "\"url\"",
    "\"revision\"",
    "\"documentTemplate\"",
    "\"request\"",
    "\"dimensions\"",
    "\"documentTemplateMaxBytes\"",
    "\"binaryQuantized\"",
    "\"model\"",
    "\"headers\"",
];

/// Returns `true` when the bytes look like Meilisearch settings.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"rankingRules\"")
        || t.contains("\"searchableAttributes\"")
        || t.contains("\"filterableAttributes\""))
        && t.contains("{")
        && t.contains("}")
        || t.contains("MEILI_")
}

impl Meili {
    /// Parses Meilisearch settings, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            attr_keys: 0,
            ranking_rules: 0,
            misc_keys: 0,
            env_vars: 0,
            comments: 0,
        };
        for k in ATTRS {
            c.attr_keys += t.matches(k).count();
        }
        for r in [
            "\"words\"",
            "\"typo\"",
            "\"proximity\"",
            "\"attribute\"",
            "\"sort\"",
            "\"exactness\"",
            "\"score\"",
            "\"random\"",
            "\"asc(",
            "\"desc(",
            "asc(",
            "desc(",
        ] {
            c.ranking_rules += t.matches(r).count();
        }
        for k in MISC {
            c.misc_keys += t.matches(k).count();
        }
        let mut rest = t;
        while let Some(i) = rest.find("MEILI_") {
            c.env_vars += 1;
            rest = &rest[i + 6..];
        }
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"{\"rankingRules\":[\"words\",\"typo\",\"proximity\",\"attribute\",\"sort\",\"exactness\"],\"searchableAttributes\":[\"title\",\"overview\"],\"filterableAttributes\":[\"genre\"],\"sortableAttributes\":[\"release_date\"],\"stopWords\":[\"the\",\"a\"],\"distinctAttribute\":\"movie_id\",\"typoTolerance\":{\"enabled\":true,\"minWordSizeForTypos\":{\"oneTypo\":4,\"twoTypos\":8}},\"pagination\":{\"maxTotalHits\":1000}}\n";

    #[test]
    fn parses_meili() {
        let c = Meili::parse(CONF).unwrap();
        assert_eq!(c.attr_keys, 4);
        assert_eq!(c.ranking_rules, 6);
        assert!(c.misc_keys >= 8);
    }

    #[test]
    fn rejects_non_meili() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(Meili::parse(b"x").is_none());
    }
}
