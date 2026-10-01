//! Elasticsearch index/cluster settings JSON detection and census.
//!
//! Counts `settings`/`analysis`/`analyzer`/`tokenizer`/`filter`/`char_filter`/
//! `normalizer`/`index`/`number_of_shards`/`number_of_replicas`/`refresh_interval`/
//! `max_result_window`/`max_inner_result_window`/`max_docvalue_fields_search`/
//! `max_script_fields`/`max_terms_count`/`max_regex_length`/`routing`/`codec`/
//! `blocks.*`/`mapping.*`/`highlight.max_analyzed_offset`/`merge.scheduler`/
//! `store.type`/`index.priority`/`routing_partition_size`/`hidden`/`sort.*`/
//! `check_on_startup`/`gc_deletes`/`soft_deletes`/`translog.*`/`flush.*`/
//! `recovery`/`search`/`request`/`get`/`bulk`/`write`/`queries`/`allocation`/
//! `cluster`/`node`/`path`/`network`/`discovery`/`gateway`/`action`/`xpack` keys,
//! built-in analyzer/filter names (`standard`/`simple`/`whitespace`/`stop`/
//! `keyword`/`pattern`/`english`/`custom`/`lowercase`/`uppercase`/`ngram`/
//! `edge_ngram`/`stop`/`synonym`/`stemmer`/`shingle`/`asciifolding`/`icu_*`/
//! `kuromoji*`/`smartcn*`/`cjk`/`elision`/`hunspell`/`keep`/`length`/`limit`/
//! `minhash`/`pattern_capture`/`pattern_replace`/`phonetic`/`porter_stem`/
//! `remove_duplicates`/`reverse`/`snowball`/`trim`/`truncate`/`unique`/
//! `word_delimiter*`/`fingerprint`/`flatten_graph`/`multiplexer`/`predicate_token_filter`),
//! and `#`/`//` comment lines.
//!
//! ```
//! let b = b"{\"settings\":{\"index\":{\"number_of_shards\":2,\"number_of_replicas\":1,\"refresh_interval\":\"5s\"},\"analysis\":{\"analyzer\":{\"mine\":{\"type\":\"custom\",\"tokenizer\":\"standard\",\"filter\":[\"lowercase\",\"stop\"]}}}}}\n";
//! assert!(izanagi_kit::essettings::detect(b));
//! let c = izanagi_kit::essettings::Essettings::parse(b).unwrap();
//! assert_eq!(c.settings, 1);
//! assert_eq!(c.index_keys, 4);
//! assert_eq!(c.analyzers, 1);
//! ```

/// Parsed ES settings summary.
#[derive(Debug, Clone)]
pub struct Essettings {
    /// `"settings"` root key.
    pub settings: usize,
    /// `"index"`/`"number_of_shards"`/`"number_of_replicas"`/`"refresh_interval"`/`"max_result_window"`/`"max_inner_result_window"`/`"max_docvalue_fields_search"`/`"max_script_fields"`/`"max_terms_count"`/`"max_regex_length"`/`"max_analyzed_offset"`/`"codec"`/`"routing_partition_size"`/`"hidden"`/`"soft_deletes"`/`"gc_deletes"`/`"check_on_startup"`/`"provided_name"`/`"creation_date"`/`"uuid"`/`"version"`/`"store"`/`"translog"`/`"flush"`/`"merge"`/`"recovery"`/`"search"`/`"slowlog"`/`"sort"`/`"blocks"`/`"queries"`/`"allocation"` index settings keys.
    pub index_keys: usize,
    /// `"analysis"`/`"analyzer"`/`"tokenizer"`/`"filter"`/`"char_filter"`/`"normalizer"` analysis section keys.
    pub analysis: usize,
    /// named analysis components (inner objects with `"type"`).
    pub analyzers: usize,
    /// `"type":"<builtin analyzer/tokenizer/filter name>"` values.
    pub builtin: usize,
    /// `"cluster"`/`"node"`/`"path"`/`"network"`/`"discovery"`/`"gateway"`/`"action"`/`"xpack"`/`"ingest"`/`"script"`/`"reindex"`/`"searchable_snapshots"`/`"slm"`/`"ilm"`/`"ccr"`/`"monitoring"`/`"watcher"`/`"security"` cluster-level keys.
    pub cluster_keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const IDX: &[&str] = &[
    "\"index\"",
    "\"number_of_shards\"",
    "\"number_of_replicas\"",
    "\"refresh_interval\"",
    "\"max_result_window\"",
    "\"max_inner_result_window\"",
    "\"max_docvalue_fields_search\"",
    "\"max_script_fields\"",
    "\"max_terms_count\"",
    "\"max_regex_length\"",
    "\"max_analyzed_offset\"",
    "\"codec\"",
    "\"routing_partition_size\"",
    "\"hidden\"",
    "\"soft_deletes\"",
    "\"gc_deletes\"",
    "\"check_on_startup\"",
    "\"provided_name\"",
    "\"creation_date\"",
    "\"uuid\"",
    "\"version\"",
    "\"store\"",
    "\"translog\"",
    "\"flush\"",
    "\"merge\"",
    "\"recovery\"",
    "\"search\"",
    "\"slowlog\"",
    "\"sort\"",
    "\"blocks\"",
    "\"queries\"",
    "\"allocation\"",
    "\"lifecycle\"",
    "\"rollup\"",
    "\"data_path\"",
    "\"auto_expand_replicas\"",
    "\"search_idle\"",
    "\"write.wait_for_active_shards\"",
    "\"shard.check_on_startup\"",
    "\"final_pipeline\"",
    "\"default_pipeline\"",
];

const ANALYSIS: &[&str] = &[
    "\"analysis\"",
    "\"analyzer\"",
    "\"tokenizer\"",
    "\"filter\"",
    "\"char_filter\"",
    "\"normalizer\"",
];

const BUILTIN: &[&str] = &[
    "\"standard\"",
    "\"simple\"",
    "\"whitespace\"",
    "\"stop\"",
    "\"keyword\"",
    "\"pattern\"",
    "\"english\"",
    "\"custom\"",
    "\"lowercase\"",
    "\"uppercase\"",
    "\"ngram\"",
    "\"edge_ngram\"",
    "\"synonym\"",
    "\"stemmer\"",
    "\"shingle\"",
    "\"asciifolding\"",
    "\"fingerprint\"",
    "\"flatten_graph\"",
    "\"multiplexer\"",
    "\"porter_stem\"",
    "\"reverse\"",
    "\"snowball\"",
    "\"trim\"",
    "\"truncate\"",
    "\"unique\"",
    "\"cjk\"",
    "\"elision\"",
    "\"hunspell\"",
    "\"keep\"",
    "\"length\"",
    "\"limit\"",
    "\"minhash\"",
    "\"pattern_capture\"",
    "\"pattern_replace\"",
    "\"phonetic\"",
    "\"remove_duplicates\"",
    "\"word_delimiter\"",
    "\"word_delimiter_graph\"",
    "\"kstem\"",
    "\"icu_analyzer\"",
    "\"icu_tokenizer\"",
    "\"icu_folding\"",
    "\"icu_normalizer\"",
    "\"icu_collation\"",
    "\"icu_transform\"",
    "\"kuromoji\"",
    "\"kuromoji_tokenizer\"",
    "\"kuromoji_stemmer\"",
    "\"kuromoji_part_of_speech\"",
    "\"kuromoji_readingform\"",
    "\"smartcn_tokenizer\"",
    "\"nori_tokenizer\"",
    "\"nori_part_of_speech\"",
    "\"nori_readingform\"",
    "\"ukrainian\"",
    "\"arabic\"",
    "\"armenian\"",
    "\"basque\"",
    "\"bengali\"",
    "\"brazilian\"",
    "\"bulgarian\"",
    "\"catalan\"",
    "\"chinese\"",
    "\"czech\"",
    "\"danish\"",
    "\"dutch\"",
    "\"estonian\"",
    "\"finnish\"",
    "\"french\"",
    "\"galician\"",
    "\"german\"",
    "\"greek\"",
    "\"hindi\"",
    "\"hungarian\"",
    "\"indonesian\"",
    "\"irish\"",
    "\"italian\"",
    "\"latvian\"",
    "\"lithuanian\"",
    "\"norwegian\"",
    "\"persian\"",
    "\"portuguese\"",
    "\"romanian\"",
    "\"russian\"",
    "\"sorani\"",
    "\"spanish\"",
    "\"swedish\"",
    "\"turkish\"",
    "\"thai\"",
    "\"serbian\"",
];

const CLUSTER: &[&str] = &[
    "\"cluster\"",
    "\"node\"",
    "\"path\"",
    "\"network\"",
    "\"discovery\"",
    "\"gateway\"",
    "\"action\"",
    "\"xpack\"",
    "\"ingest\"",
    "\"script\"",
    "\"reindex\"",
    "\"searchable_snapshots\"",
    "\"slm\"",
    "\"ilm\"",
    "\"ccr\"",
    "\"monitoring\"",
    "\"watcher\"",
    "\"security\"",
    "\"persistent\"",
    "\"transient\"",
];

/// Returns `true` when the bytes look like ES settings.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"settings\"")
        && (t.contains("number_of_shards") || t.contains("analysis") || t.contains("\"index\"")))
        || (t.contains("\"number_of_shards\"") && t.contains("{") && t.contains("}"))
}

impl Essettings {
    /// Parses ES settings, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            settings: 0,
            index_keys: 0,
            analysis: 0,
            analyzers: 0,
            builtin: 0,
            cluster_keys: 0,
            comments: 0,
        };
        c.settings += t.matches("\"settings\"").count();
        for k in IDX {
            c.index_keys += t.matches(k).count();
        }
        for k in ANALYSIS {
            c.analysis += t.matches(k).count();
        }
        // Named components: `"<ident>": {` objects nested inside `analysis` at
        // depth > the analysis object (component containers stay at its depth).
        let bytes = t.as_bytes();
        let mut depth: i64 = 0;
        let mut ad: i64 = 0;
        let mut in_analysis = false;
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'"' => {
                    if let Some(e) = t[i + 1..].find('"') {
                        let key = &t[i + 1..i + 1 + e];
                        let mut j = i + 1 + e + 1;
                        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                            j += 1;
                        }
                        if j < bytes.len() && bytes[j] == b':' {
                            let mut k = j + 1;
                            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                                k += 1;
                            }
                            if k < bytes.len() && bytes[k] == b'{' {
                                if key == "analysis" {
                                    in_analysis = true;
                                    ad = depth + 1;
                                } else if in_analysis && depth > ad {
                                    c.analyzers += 1;
                                }
                            }
                            i = j;
                        }
                    }
                }
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if in_analysis && depth < ad {
                        in_analysis = false;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        for k in BUILTIN {
            c.builtin += t.matches(&format!(":{k}")).count()
                + t.matches(&format!(": {k}")).count()
                + t.matches(&format!("[{k}")).count()
                + t.matches(&format!("[ {k}")).count()
                + t.matches(&format!(",{k}")).count()
                + t.matches(&format!(", {k}")).count();
        }
        for k in CLUSTER {
            c.cluster_keys += t.matches(k).count();
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

    const CONF: &[u8] = b"{\"settings\":{\"index\":{\"number_of_shards\":2,\"number_of_replicas\":1,\"refresh_interval\":\"5s\"},\"analysis\":{\"analyzer\":{\"mine\":{\"type\":\"custom\",\"tokenizer\":\"standard\",\"filter\":[\"lowercase\",\"stop\"]}},\"filter\":{\"my_stop\":{\"type\":\"stop\"}}}}}\n";

    #[test]
    fn parses_essettings() {
        let c = Essettings::parse(CONF).unwrap();
        assert_eq!(c.settings, 1);
        assert_eq!(c.index_keys, 4);
        assert!(c.analysis >= 4);
        assert_eq!(c.analyzers, 2);
        assert!(c.builtin >= 4);
    }

    #[test]
    fn rejects_non_essettings() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(Essettings::parse(b"x").is_none());
    }
}
