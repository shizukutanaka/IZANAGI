//! Elasticsearch mapping definition (`mappings` JSON) detection and census.
//!
//! Counts `mappings`/`properties`/`fields`/`type`/`analyzer`/`search_analyzer`/
//! `normalizer`/`format`/`index`/`store`/`doc_values`/`fielddata`/`term_vector`/
//! `boost`/`null_value`/`copy_to`/`ignore_above`/`dynamic`/`enabled`/`_meta`/
//! `runtime`/`properties` keys, ES field types (`text`/`keyword`/`long`/
//! `integer`/`short`/`byte`/`double`/`float`/`half_float`/`scaled_float`/
//! `date`/`boolean`/`ip`/`geo_point`/`geo_shape`/`nested`/`object`/`flattened`/
//! `dense_vector`/`sparse_vector`/`rank_feature`/`rank_features`/`completion`/
//! `search_as_you_type`/`token_count`/`murmur3`/`annotated-text`/`percolator`/
//! `join`/`alias`/`histogram`/`constant_keyword`/`wildcard`/`version`/
//! `aggregate_metric_double`/`binary`/`match_only_text`/`unsigned_long`),
//! `dynamic_templates`, and `#`/`//` comment lines.
//!
//! ```
//! let b = b"{\"mappings\":{\"properties\":{\"title\":{\"type\":\"text\",\"analyzer\":\"standard\",\"fields\":{\"raw\":{\"type\":\"keyword\"}}},\"year\":{\"type\":\"integer\"}}}}\n";
//! assert!(izanagi_kit::esmapping::detect(b));
//! let c = izanagi_kit::esmapping::Esmapping::parse(b).unwrap();
//! assert_eq!(c.mappings, 1);
//! assert_eq!(c.properties, 1);
//! assert_eq!(c.types, 3);
//! ```

/// Parsed ES mapping summary.
#[derive(Debug, Clone)]
pub struct Esmapping {
    /// `"mappings"` root key.
    pub mappings: usize,
    /// `"properties"` blocks.
    pub properties: usize,
    /// `"fields"` multi-field blocks.
    pub fields: usize,
    /// `"dynamic_templates"`/`"date_detection"`/`"numeric_detection"`/`"dynamic_date_formats"` dynamic entries.
    pub dynamic: usize,
    /// `"type"`/`"analyzer"`/`"search_analyzer"`/`"normalizer"`/`"format"`/`"index"`/`"store"`/`"doc_values"`/`"fielddata"`/`"term_vector"`/`"boost"`/`"null_value"`/`"copy_to"`/`"ignore_above"`/`"eager_global_ordinals"`/`"coerce"`/`"similarity"`/`"enabled"`/`"subobjects"`/`"meta"`/`"dimension"`/`"time_series_metric"`/`"index_options"`/`"position_increment_gap"`/`"scaling_factor"`/`"depth_limit"`/`"dims"`/`"element_type"`/`"ignore_malformed"`/`"ignore_z_value"`/`"orientation"`/`"points_only"`/`"precision_step"`/`"search_quote_analyzer"`/`"split_queries_on_whitespace"`/`"preserve_separators"`/`"preserve_position_increments"`/`"max_input_length"`/`"max_shingle_size"`/`"min_shingle_size"`/`"output_unigrams"`/`"path"`/`"prefix_length"`/`"max_determinized_states"`/`"norms"`/`"on_script_error"`/`"script"`/`"emit`/`"runtime"` keys.
    pub mapping_keys: usize,
    /// `"type":"<known ES type>"` field type values.
    pub types: usize,
    /// `"_meta"`/`"_source"`/`"_routing"`/`"_size"`/`"_doc_count"`/`"_field_names"`/`"_ignored"`/`"_index"`/`"_id"`/`"_ingest"`/`"_tier"` meta keys.
    pub meta_keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const MAP_KEYS: &[&str] = &[
    "\"analyzer\"",
    "\"search_analyzer\"",
    "\"normalizer\"",
    "\"format\"",
    "\"index\"",
    "\"store\"",
    "\"doc_values\"",
    "\"fielddata\"",
    "\"term_vector\"",
    "\"boost\"",
    "\"null_value\"",
    "\"copy_to\"",
    "\"ignore_above\"",
    "\"eager_global_ordinals\"",
    "\"coerce\"",
    "\"similarity\"",
    "\"enabled\"",
    "\"subobjects\"",
    "\"meta\"",
    "\"dimension\"",
    "\"time_series_metric\"",
    "\"index_options\"",
    "\"position_increment_gap\"",
    "\"scaling_factor\"",
    "\"depth_limit\"",
    "\"dims\"",
    "\"element_type\"",
    "\"ignore_malformed\"",
    "\"ignore_z_value\"",
    "\"orientation\"",
    "\"points_only\"",
    "\"precision_step\"",
    "\"search_quote_analyzer\"",
    "\"split_queries_on_whitespace\"",
    "\"preserve_separators\"",
    "\"preserve_position_increments\"",
    "\"max_input_length\"",
    "\"max_shingle_size\"",
    "\"min_shingle_size\"",
    "\"output_unigrams\"",
    "\"path\"",
    "\"prefix_length\"",
    "\"max_determinized_states\"",
    "\"norms\"",
    "\"runtime\"",
];

const TYPES: &[&str] = &[
    "\"text\"",
    "\"keyword\"",
    "\"long\"",
    "\"integer\"",
    "\"short\"",
    "\"byte\"",
    "\"double\"",
    "\"float\"",
    "\"half_float\"",
    "\"scaled_float\"",
    "\"date\"",
    "\"date_nanos\"",
    "\"boolean\"",
    "\"ip\"",
    "\"geo_point\"",
    "\"geo_shape\"",
    "\"point\"",
    "\"shape\"",
    "\"nested\"",
    "\"object\"",
    "\"flattened\"",
    "\"dense_vector\"",
    "\"sparse_vector\"",
    "\"rank_feature\"",
    "\"rank_features\"",
    "\"completion\"",
    "\"search_as_you_type\"",
    "\"token_count\"",
    "\"murmur3\"",
    "\"annotated-text\"",
    "\"percolator\"",
    "\"join\"",
    "\"alias\"",
    "\"histogram\"",
    "\"constant_keyword\"",
    "\"wildcard\"",
    "\"version\"",
    "\"aggregate_metric_double\"",
    "\"binary\"",
    "\"match_only_text\"",
    "\"unsigned_long\"",
    "\"range\"",
];

const META: &[&str] = &[
    "\"_meta\"",
    "\"_source\"",
    "\"_routing\"",
    "\"_size\"",
    "\"_doc_count\"",
    "\"_field_names\"",
    "\"_ignored\"",
    "\"_index\"",
    "\"_id\"",
    "\"_ingest\"",
    "\"_tier\"",
];

/// Returns `true` when the bytes look like an ES mapping.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"mappings\"") && (t.contains("\"properties\"") || t.contains("\"type\"")))
        || (t.contains("\"properties\"")
            && (t.contains("\"type\": \"text\"")
                || t.contains("\"type\":\"text\"")
                || t.contains("\"type\": \"keyword\"")
                || t.contains("\"type\":\"keyword\"")))
}

impl Esmapping {
    /// Parses an ES mapping, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            mappings: 0,
            properties: 0,
            fields: 0,
            dynamic: 0,
            mapping_keys: 0,
            types: 0,
            meta_keys: 0,
            comments: 0,
        };
        c.mappings += t.matches("\"mappings\"").count();
        c.properties += t.matches("\"properties\"").count();
        c.fields += t.matches("\"fields\"").count();
        c.dynamic += t.matches("\"dynamic_templates\"").count()
            + t.matches("\"date_detection\"").count()
            + t.matches("\"numeric_detection\"").count()
            + t.matches("\"dynamic_date_formats\"").count()
            + t.matches("\"dynamic\":").count();
        for k in MAP_KEYS {
            c.mapping_keys += t.matches(k).count();
        }
        c.mapping_keys += t.matches("\"type\"").count();
        for ty in TYPES {
            c.types += t.matches(&format!("\"type\":{ty}")).count()
                + t.matches(&format!("\"type\": {ty}")).count();
        }
        for m in META {
            c.meta_keys += t.matches(m).count();
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

    const CONF: &[u8] = b"{\"mappings\":{\"dynamic\":\"strict\",\"properties\":{\"title\":{\"type\":\"text\",\"analyzer\":\"standard\",\"fields\":{\"raw\":{\"type\":\"keyword\",\"ignore_above\":256}}},\"year\":{\"type\":\"integer\"},\"geo\":{\"type\":\"geo_point\"}}}}\n";

    #[test]
    fn parses_esmapping() {
        let c = Esmapping::parse(CONF).unwrap();
        assert_eq!(c.mappings, 1);
        assert_eq!(c.properties, 1);
        assert_eq!(c.fields, 1);
        assert_eq!(c.types, 4);
        assert_eq!(c.dynamic, 1);
        assert!(c.mapping_keys >= 5);
    }

    #[test]
    fn rejects_non_esmapping() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(Esmapping::parse(b"x").is_none());
    }
}
