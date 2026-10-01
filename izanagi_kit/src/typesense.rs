//! Typesense collection schema / search settings JSON detection and census.
//!
//! Counts `fields`/`name`/`type`/`facet`/`optional`/`index`/`sort`/`store`/
//! `locale`/`infix`/`stem`/`nested`/`nested_array`/`reference`/`range_index`/
//! `num_dim`/`vec_dist`/`hnsw_params`/`M`/`ef_construction`/`ef`/`flat_search_cutoff`/
//! `drop`/`embed`/`from`/`model_name`/`api_key`/`symbols_to_index`/`token_separators`/
//! `default_sorting_field`/`enable_nested_fields`/`voice_query_model`/`metadata`/
//! `collection`/`q`/`query_by`/`query_by_weights`/`prefix`/`infix`/`filter_by`/
//! `sort_by`/`facet_by`/`max_facet_values`/`facet_query`/`facet_strategy`/`num_typos`/
//! `page`/`per_page`/`group_by`/`group_limit`/`group_missing_values`/`include_fields`/
//! `exclude_fields`/`highlight_fields`/`highlight_full_fields`/`highlight_affix_num_tokens`/
//! `highlight_start_tag`/`highlight_end_tag`/`snippet_threshold`/`drop_tokens_threshold`/
//! `typo_tokens_threshold`/`pinned_hits`/`hidden_hits`/`limit_hits`/`pre_segmented_query`/
//! `preset`/`max_candidates`/`split_join_tokens`/`text_match_type`/`enable_lazy_filter`/
//! `max_filtering_candidates`/`rerank_hybrid_matches`/`validate_field_names`/
//! `enable_typos_for_numerical_tokens`/`enable_typos_for_alpha_numerical_tokens`/
//! `synonym_precedence`/`search_cutoff_ms`/`use_cache`/`cache_size`/`facet_query_num_typos`/
//! `remote_embedding_timeout_ms`/`remote_embedding_num_tries`/`prioritize_exact_match`/
//! `prioritize_token_position`/`prioritize_num_matching_fields`/`exhaustive_search`/
//! `conversation`/`conversation_model_id`/`conversation_id`/`system_prompt`/`history`/
//! `min_len_1typo`/`min_len_2typo`/`drop_tokens_mode`/`prioritize_conversation_history`/
//! `vector_query`/`k`/`distance_threshold`/`alpha`/`exclude_fields`/`personalization_*`/
//! `override`/`synonym`/`rule`/`tags`/`stopwords`/`curation`/`analytics`/`destination`/
//! `src`/`target`/`popular`/`counter`/`type` fields, `TYPESENSE_*`/`TYPESENSE` env,
//! and `#`/`//` comment lines.
//!
//! ```
//! let b = b"{\"name\":\"books\",\"fields\":[{\"name\":\"title\",\"type\":\"string\"},{\"name\":\"year\",\"type\":\"int32\",\"facet\":true},{\"name\":\"rating\",\"type\":\"float\",\"optional\":true}],\"default_sorting_field\":\"year\",\"symbols_to_index\":[\"+\"],\"token_separators\":[\"-\"]}\n";
//! assert!(izanagi_kit::typesense::detect(b));
//! let c = izanagi_kit::typesense::Typesense::parse(b).unwrap();
//! assert_eq!(c.fields, 3);
//! assert_eq!(c.types, 3);
//! assert!(c.field_opts >= 2);
//! ```

/// Parsed Typesense schema summary.
#[derive(Debug, Clone)]
pub struct Typesense {
    /// `"name"` collection/field names.
    pub names: usize,
    /// `"fields"` array entries (estimated by `{"name":` inside fields array).
    pub fields: usize,
    /// `"type":"<type>"` values (`string`/`string[]`/`int32`/`int64`/`float`/`bool`/`object`/`object[]`/`string*`/`auto`/`geopoint`/`geopoint[]`/`geopolygon`/`image`/`float[]`/`int32[]`/`int64[]`/`bool[]`/`auto_embedding`/`vec_dist`/`union`).
    pub types: usize,
    /// `"facet"`/`"optional"`/`"index"`/`"sort"`/`"store"`/`"locale"`/`"infix"`/`"stem"`/`"nested"`/`"reference"`/`"range_index"`/`"num_dim"`/`"vec_dist"`/`"hnsw_params"`/`"M"`/`"ef_construction"`/`"ef"`/`"flat_search_cutoff"`/`"drop"`/`"embed"`/`"from"`/`"model_name"`/`"api_key"` field options.
    pub field_opts: usize,
    /// `"default_sorting_field"`/`"enable_nested_fields"`/`"symbols_to_index"`/`"token_separators"`/`"voice_query_model"`/`"metadata"` collection-level keys.
    pub collection_keys: usize,
    /// search/curation keys (`"q"`/`"query_by"`/`"filter_by"`/`"sort_by"`/`"facet_by"`/`"num_typos"`/`"group_by"`/`"include_fields"`/`"exclude_fields"`/`"highlight_*"`/`"pinned_hits"`/`"hidden_hits"`/`"limit_hits"`/`"preset"`/`"max_candidates"`/`"vector_query"`/`"conversation*"`/`"min_len_*"`/`"override"`/`"synonym"`/`"stopwords"`/`"curation"`/`"analytics"`/`"destination"`/`"src"`/`"popular"`/`"counter"`).
    pub query_keys: usize,
    /// `TYPESENSE_*` env vars / `--typesense-*` flags.
    pub env_vars: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const TYPES: &[&str] = &[
    "\"string\"",
    "\"string[]\"",
    "\"int32\"",
    "\"int64\"",
    "\"float\"",
    "\"bool\"",
    "\"object\"",
    "\"object[]\"",
    "\"string*\"",
    "\"auto\"",
    "\"geopoint\"",
    "\"geopoint[]\"",
    "\"geopolygon\"",
    "\"image\"",
    "\"float[]\"",
    "\"int32[]\"",
    "\"int64[]\"",
    "\"bool[]\"",
    "\"auto_embedding\"",
    "\"union\"",
];

const FIELD_OPTS: &[&str] = &[
    "\"facet\"",
    "\"optional\"",
    "\"index\"",
    "\"sort\"",
    "\"store\"",
    "\"locale\"",
    "\"infix\"",
    "\"stem\"",
    "\"nested\"",
    "\"nested_array\"",
    "\"reference\"",
    "\"range_index\"",
    "\"num_dim\"",
    "\"vec_dist\"",
    "\"hnsw_params\"",
    "\"M\"",
    "\"ef_construction\"",
    "\"ef\"",
    "\"flat_search_cutoff\"",
    "\"drop\"",
    "\"embed\"",
    "\"from\"",
    "\"model_name\"",
    "\"api_key\"",
];

const COLL_KEYS: &[&str] = &[
    "\"default_sorting_field\"",
    "\"enable_nested_fields\"",
    "\"symbols_to_index\"",
    "\"token_separators\"",
    "\"voice_query_model\"",
    "\"metadata\"",
    "\"created_at\"",
];

const QUERY_KEYS: &[&str] = &[
    "\"q\"",
    "\"query_by\"",
    "\"query_by_weights\"",
    "\"prefix\"",
    "\"filter_by\"",
    "\"sort_by\"",
    "\"facet_by\"",
    "\"max_facet_values\"",
    "\"facet_query\"",
    "\"facet_strategy\"",
    "\"num_typos\"",
    "\"page\"",
    "\"per_page\"",
    "\"group_by\"",
    "\"group_limit\"",
    "\"group_missing_values\"",
    "\"include_fields\"",
    "\"exclude_fields\"",
    "\"highlight_fields\"",
    "\"highlight_full_fields\"",
    "\"highlight_affix_num_tokens\"",
    "\"highlight_start_tag\"",
    "\"highlight_end_tag\"",
    "\"snippet_threshold\"",
    "\"drop_tokens_threshold\"",
    "\"typo_tokens_threshold\"",
    "\"pinned_hits\"",
    "\"hidden_hits\"",
    "\"limit_hits\"",
    "\"pre_segmented_query\"",
    "\"preset\"",
    "\"max_candidates\"",
    "\"split_join_tokens\"",
    "\"text_match_type\"",
    "\"enable_lazy_filter\"",
    "\"max_filtering_candidates\"",
    "\"rerank_hybrid_matches\"",
    "\"validate_field_names\"",
    "\"enable_typos_for_numerical_tokens\"",
    "\"enable_typos_for_alpha_numerical_tokens\"",
    "\"synonym_precedence\"",
    "\"search_cutoff_ms\"",
    "\"use_cache\"",
    "\"cache_size\"",
    "\"facet_query_num_typos\"",
    "\"remote_embedding_timeout_ms\"",
    "\"remote_embedding_num_tries\"",
    "\"prioritize_exact_match\"",
    "\"prioritize_token_position\"",
    "\"prioritize_num_matching_fields\"",
    "\"exhaustive_search\"",
    "\"conversation\"",
    "\"conversation_model_id\"",
    "\"conversation_id\"",
    "\"system_prompt\"",
    "\"history\"",
    "\"min_len_1typo\"",
    "\"min_len_2typo\"",
    "\"drop_tokens_mode\"",
    "\"prioritize_conversation_history\"",
    "\"vector_query\"",
    "\"k\"",
    "\"distance_threshold\"",
    "\"alpha\"",
    "\"personalization_type\"",
    "\"personalization_user_id\"",
    "\"personalization_model_id\"",
    "\"personalization_event_id\"",
    "\"personalization_n_events\"",
    "\"override\"",
    "\"synonym\"",
    "\"rule\"",
    "\"tags\"",
    "\"stopwords\"",
    "\"curation\"",
    "\"analytics\"",
    "\"destination\"",
    "\"src\"",
    "\"target\"",
    "\"popular\"",
    "\"counter\"",
    "\"apiKey\"",
    "\"node\"",
    "\"host\"",
    "\"port\"",
    "\"protocol\"",
    "\"path\"",
    "\"numRetries\"",
    "\"retryIntervalSeconds\"",
    "\"connectionTimeoutSeconds\"",
    "\"nearestNode\"",
    "\"logLevel\"",
];

/// Returns `true` when the bytes look like a Typesense schema.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"fields\"")
        && (t.contains("\"facet\"")
            || t.contains("\"default_sorting_field\"")
            || t.contains("\"type\":\"string\"")
            || t.contains("\"type\": \"string\"")))
        || t.contains("TYPESENSE_")
        || t.contains("\"token_separators\"")
}

impl Typesense {
    /// Parses a Typesense schema, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            names: 0,
            fields: 0,
            types: 0,
            field_opts: 0,
            collection_keys: 0,
            query_keys: 0,
            env_vars: 0,
            comments: 0,
        };
        c.names += t.matches("\"name\"").count();
        c.fields += t.matches(",{\"name\":").count() + t.matches("[{\"name\":").count();
        for ty in TYPES {
            c.types += t.matches(&format!("\"type\":{ty}")).count()
                + t.matches(&format!("\"type\": {ty}")).count();
        }
        for k in FIELD_OPTS {
            c.field_opts += t.matches(k).count();
        }
        for k in COLL_KEYS {
            c.collection_keys += t.matches(k).count();
        }
        for k in QUERY_KEYS {
            c.query_keys += t.matches(k).count();
        }
        let mut rest = t;
        while let Some(i) = rest.find("TYPESENSE_") {
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

    const CONF: &[u8] = b"{\"name\":\"books\",\"fields\":[{\"name\":\"title\",\"type\":\"string\"},{\"name\":\"year\",\"type\":\"int32\",\"facet\":true},{\"name\":\"rating\",\"type\":\"float\",\"optional\":true}],\"default_sorting_field\":\"year\",\"symbols_to_index\":[\"+\"],\"token_separators\":[\"-\"]}\n";

    #[test]
    fn parses_typesense() {
        let c = Typesense::parse(CONF).unwrap();
        assert_eq!(c.names, 4);
        assert_eq!(c.fields, 3);
        assert_eq!(c.types, 3);
        assert!(c.field_opts >= 2);
        assert_eq!(c.collection_keys, 3);
    }

    #[test]
    fn rejects_non_typesense() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(Typesense::parse(b"x").is_none());
    }
}
