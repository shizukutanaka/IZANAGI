//! Algolia index settings / query params JSON detection and census.
//!
//! Counts `searchableAttributes`/`attributesForFaceting`/`unretrievableAttributes`/
//! `attributesToRetrieve`/`attributesToSnippet`/`attributesToHighlight`/
//! `restrictSearchableAttributes`/`ranking`/`customRanking`/`replicas`/
//! `virtualReplicas`/`paginationLimitedTo`/`hitsPerPage`/`maxValuesPerFacet`/
//! `sortFacetsBy`/`attributesToTransliterate`/`camelCaseAttributes`/`decompoundedAttributes`/
//! `keepDiacriticsOnCharacters`/`customNormalization`/`customSynonyms`/`indexLanguages`/
//! `queryLanguages`/`ignorePlurals`/`removeStopWords`/`queryType`/`removeWordsIfNoResults`/
//! `disablePrefixOnAttributes`/`disableExactOnAttributes`/`exactOnSingleWordQuery`/
//! `alternativesAsExact`/`advancedSyntax`/`advancedSyntaxFeatures`/`optionalWords`/
//! `minProximity`/`separatorsToIndex`/`numericAttributesForFiltering`/
//! `allowCompressionOfIntegerArray`/`attributeForDistinct`/`distinct`/
//! `proximity`/`attributeCriteriaComputedByMinProximity`/`enableReRanking`/
//! `relevancyStrictness`/`renderingContent`/`synonyms`/`placeholders`/
//! `optionalFilter`/`responseFields`/`maxFacetHits`/`version`/`minWordSizefor1Typo`/
//! `minWordSizefor2Typos`/`typoTolerance`/`allowTyposOnNumericTokens`/
//! `disableTypoToleranceOnAttributes`/`disableTypoToleranceOnWords`/`separators`/
//! `synonym`/`oneWaySynonym`/`altCorrection1`/`altCorrection2`/`placeholders`/
//! `rules`/`consequence`/`objectID`/`condition`/`anchoring`/`pattern`/`context`/
//! `promote`/`hide`/`userData`/`filter`/`filters`/`facetFilters`/`numericFilters`/
//! `tagFilters`/`optionalFilters`/`facets`/`facets_stats`/`exhaustiveFacetsCount`/
//! `aroundLatLng`/`aroundLatLngViaIP`/`aroundRadius`/`aroundPrecision`/`minimumAroundRadius`/
//! `insideBoundingBox`/`insidePolygon`/`offset`/`length`/`snippetEllipsisText`/
//! `highlightPreTag`/`highlightPostTag`/`restrictHighlightAndSnippetArrays`/
//! `getRankingInfo`/`clickAnalytics`/`analytics`/`analyticsTags`/`enableABTest`/
//! `queryID`/`facetingAfterDistinct`/`explain`/`naturalLanguages`/`ruleContexts`/
//! `userToken`/`personalization`/`enablePersonalization`/`personalizationImpact`/
//! `abTestID`/`abTestVariantID`/`percentileComputation`/`enableRules`/`apiKey`/
//! `appID`/`X-Algolia-*` headers, `ALGOLIA_*` env vars, and `#`/`//` comments.
//!
//! ```
//! let b = b"{\"searchableAttributes\":[\"title\",\"unordered(overview)\"],\"attributesForFaceting\":[\"filterOnly(genre)\",\"searchable(author)\"],\"customRanking\":[\"desc(popularity)\",\"asc(price)\"],\"ranking\":[\"typo\",\"geo\",\"words\",\"filters\",\"proximity\",\"attribute\",\"exact\",\"custom\"],\"hitsPerPage\":20,\"maxValuesPerFacet\":100,\"typoTolerance\":true,\"minWordSizefor1Typo\":4,\"minWordSizefor2Typos\":8,\"enableReRanking\":true,\"paginationLimitedTo\":5000,\"attributeForDistinct\":\"imdb\",\"distinct\":true}\n";
//! assert!(izanagi_kit::algolia::detect(b));
//! let c = izanagi_kit::algolia::Algolia::parse(b).unwrap();
//! assert_eq!(c.attr_keys, 2);
//! assert_eq!(c.ranking_keys, 2);
//! assert!(c.misc_keys >= 7);
//! ```

/// Parsed Algolia settings summary.
#[derive(Debug, Clone)]
pub struct Algolia {
    /// `"searchableAttributes"`/`"attributesForFaceting"`/`"unretrievableAttributes"`/`"attributesToRetrieve"`/`"attributesToSnippet"`/`"attributesToHighlight"`/`"restrictSearchableAttributes"`/`"attributesToTransliterate"`/`"camelCaseAttributes"`/`"decompoundedAttributes"`/`"attributesForFaceting"` attribute list keys.
    pub attr_keys: usize,
    /// `"ranking"`/`"customRanking"`/`"replicas"`/`"virtualReplicas"`/`"slaves"` ranking/replica keys.
    pub ranking_keys: usize,
    /// `"hitsPerPage"`/`"paginationLimitedTo"`/`"maxValuesPerFacet"`/`"sortFacetsBy"`/`"keepDiacriticsOnCharacters"`/`"customNormalization"`/`"customSynonyms"`/`"indexLanguages"`/`"queryLanguages"`/`"ignorePlurals"`/`"removeStopWords"`/`"queryType"`/`"removeWordsIfNoResults"`/`"disablePrefixOnAttributes"`/`"disableExactOnAttributes"`/`"exactOnSingleWordQuery"`/`"alternativesAsExact"`/`"advancedSyntax"`/`"advancedSyntaxFeatures"`/`"optionalWords"`/`"minProximity"`/`"separatorsToIndex"`/`"numericAttributesForFiltering"`/`"allowCompressionOfIntegerArray"`/`"attributeForDistinct"`/`"distinct"`/`"proximity"`/`"attributeCriteriaComputedByMinProximity"`/`"enableReRanking"`/`"relevancyStrictness"`/`"renderingContent"`/`"synonyms"`/`"placeholders"`/`"responseFields"`/`"maxFacetHits"`/`"version"`/`"minWordSizefor1Typo"`/`"minWordSizefor2Typos"`/`"typoTolerance"`/`"allowTyposOnNumericTokens"`/`"disableTypoToleranceOnAttributes"`/`"disableTypoToleranceOnWords"`/`"separators"` misc keys.
    pub misc_keys: usize,
    /// rules/synonym keys (`"rules"`/`"consequence"`/`"objectID"`/`"condition"`/`"anchoring"`/`"pattern"`/`"context"`/`"promote"`/`"hide"`/`"userData"`/`"synonym"`/`"oneWaySynonym"`/`"altCorrection1"`/`"altCorrection2"`).
    pub rule_keys: usize,
    /// query-time keys (`"query"`/`"filters"`/`"facetFilters"`/`"numericFilters"`/`"tagFilters"`/`"optionalFilters"`/`"facets"`/`"aroundLatLng"`/`"aroundRadius"`/`"insideBoundingBox"`/`"offset"`/`"length"`/`"highlightPreTag"`/`"highlightPostTag"`/`"getRankingInfo"`/`"clickAnalytics"`/`"analyticsTags"`/`"enableABTest"`/`"facetingAfterDistinct"`/`"userToken"`/`"personalization"`/`"enablePersonalization"`/`"percentileComputation"`/`"enableRules"`).
    pub query_keys: usize,
    /// `ALGOLIA_*` env vars + `X-Algolia-*` headers.
    pub env_vars: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const ATTRS: &[&str] = &[
    "\"searchableAttributes\"",
    "\"attributesForFaceting\"",
    "\"unretrievableAttributes\"",
    "\"attributesToRetrieve\"",
    "\"attributesToSnippet\"",
    "\"attributesToHighlight\"",
    "\"restrictSearchableAttributes\"",
    "\"attributesToTransliterate\"",
    "\"camelCaseAttributes\"",
    "\"decompoundedAttributes\"",
];

const RANK: &[&str] = &[
    "\"ranking\"",
    "\"customRanking\"",
    "\"replicas\"",
    "\"virtualReplicas\"",
    "\"slaves\"",
];

const MISC: &[&str] = &[
    "\"hitsPerPage\"",
    "\"paginationLimitedTo\"",
    "\"maxValuesPerFacet\"",
    "\"sortFacetsBy\"",
    "\"keepDiacriticsOnCharacters\"",
    "\"customNormalization\"",
    "\"customSynonyms\"",
    "\"indexLanguages\"",
    "\"queryLanguages\"",
    "\"ignorePlurals\"",
    "\"removeStopWords\"",
    "\"queryType\"",
    "\"removeWordsIfNoResults\"",
    "\"disablePrefixOnAttributes\"",
    "\"disableExactOnAttributes\"",
    "\"exactOnSingleWordQuery\"",
    "\"alternativesAsExact\"",
    "\"advancedSyntax\"",
    "\"advancedSyntaxFeatures\"",
    "\"optionalWords\"",
    "\"minProximity\"",
    "\"separatorsToIndex\"",
    "\"numericAttributesForFiltering\"",
    "\"allowCompressionOfIntegerArray\"",
    "\"attributeForDistinct\"",
    "\"distinct\"",
    "\"proximity\"",
    "\"attributeCriteriaComputedByMinProximity\"",
    "\"enableReRanking\"",
    "\"relevancyStrictness\"",
    "\"renderingContent\"",
    "\"synonyms\"",
    "\"placeholders\"",
    "\"responseFields\"",
    "\"maxFacetHits\"",
    "\"version\"",
    "\"minWordSizefor1Typo\"",
    "\"minWordSizefor2Typos\"",
    "\"typoTolerance\"",
    "\"allowTyposOnNumericTokens\"",
    "\"disableTypoToleranceOnAttributes\"",
    "\"disableTypoToleranceOnWords\"",
    "\"separators\"",
];

const RULES: &[&str] = &[
    "\"rules\"",
    "\"consequence\"",
    "\"objectID\"",
    "\"condition\"",
    "\"anchoring\"",
    "\"pattern\"",
    "\"context\"",
    "\"promote\"",
    "\"hide\"",
    "\"userData\"",
    "\"synonym\"",
    "\"oneWaySynonym\"",
    "\"altCorrection1\"",
    "\"altCorrection2\"",
];

const QUERY: &[&str] = &[
    "\"query\"",
    "\"filters\"",
    "\"facetFilters\"",
    "\"numericFilters\"",
    "\"tagFilters\"",
    "\"optionalFilters\"",
    "\"facets\"",
    "\"aroundLatLng\"",
    "\"aroundLatLngViaIP\"",
    "\"aroundRadius\"",
    "\"aroundPrecision\"",
    "\"minimumAroundRadius\"",
    "\"insideBoundingBox\"",
    "\"insidePolygon\"",
    "\"offset\"",
    "\"length\"",
    "\"snippetEllipsisText\"",
    "\"highlightPreTag\"",
    "\"highlightPostTag\"",
    "\"restrictHighlightAndSnippetArrays\"",
    "\"getRankingInfo\"",
    "\"clickAnalytics\"",
    "\"analytics\"",
    "\"analyticsTags\"",
    "\"enableABTest\"",
    "\"queryID\"",
    "\"facetingAfterDistinct\"",
    "\"explain\"",
    "\"naturalLanguages\"",
    "\"ruleContexts\"",
    "\"userToken\"",
    "\"personalization\"",
    "\"enablePersonalization\"",
    "\"personalizationImpact\"",
    "\"abTestID\"",
    "\"abTestVariantID\"",
    "\"percentileComputation\"",
    "\"enableRules\"",
    "\"apiKey\"",
    "\"appID\"",
    "\"mode\"",
];

/// Returns `true` when the bytes look like Algolia settings.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"searchableAttributes\"")
        || t.contains("\"attributesForFaceting\"")
        || t.contains("\"customRanking\""))
        && t.contains('{')
        || t.contains("ALGOLIA_")
        || t.contains("X-Algolia-")
}

impl Algolia {
    /// Parses Algolia settings, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            attr_keys: 0,
            ranking_keys: 0,
            misc_keys: 0,
            rule_keys: 0,
            query_keys: 0,
            env_vars: 0,
            comments: 0,
        };
        for k in ATTRS {
            c.attr_keys += t.matches(k).count();
        }
        for k in RANK {
            c.ranking_keys += t.matches(k).count();
        }
        for k in MISC {
            c.misc_keys += t.matches(k).count();
        }
        for k in RULES {
            c.rule_keys += t.matches(k).count();
        }
        for k in QUERY {
            c.query_keys += t.matches(k).count();
        }
        let mut rest = t;
        while let Some(i) = rest.find("ALGOLIA_") {
            c.env_vars += 1;
            rest = &rest[i + 8..];
        }
        let mut rest = t;
        while let Some(i) = rest.find("X-Algolia-") {
            c.env_vars += 1;
            rest = &rest[i + 10..];
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

    const CONF: &[u8] = b"{\"searchableAttributes\":[\"title\",\"unordered(overview)\"],\"attributesForFaceting\":[\"filterOnly(genre)\",\"searchable(author)\"],\"customRanking\":[\"desc(popularity)\",\"asc(price)\"],\"ranking\":[\"typo\",\"geo\",\"words\",\"filters\",\"proximity\",\"attribute\",\"exact\",\"custom\"],\"hitsPerPage\":20,\"maxValuesPerFacet\":100,\"typoTolerance\":true,\"minWordSizefor1Typo\":4,\"minWordSizefor2Typos\":8,\"enableReRanking\":true,\"paginationLimitedTo\":5000,\"attributeForDistinct\":\"imdb\",\"distinct\":true}\n";

    #[test]
    fn parses_algolia() {
        let c = Algolia::parse(CONF).unwrap();
        assert_eq!(c.attr_keys, 2);
        assert_eq!(c.ranking_keys, 2);
        assert!(c.misc_keys >= 8);
    }

    #[test]
    fn rejects_non_algolia() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(Algolia::parse(b"x").is_none());
    }
}
