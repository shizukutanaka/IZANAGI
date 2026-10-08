//! Cardano node `config.json` の認識と計数。
//!
//! cardano-node の JSON 設定は PascalCase キーで構成される:
//! `"EnableP2P"`/`"PeerSharing"`/`"TargetNumberOfRootPeers"`/`"TargetNumberOfKnownPeers"`、
//! `"Trace*"`(`TraceMux`/`TraceChainSyncClient`/`TraceBlockFetchDecisions`/`TraceForge`…)、
//! `"*GenesisFile"`/`"*GenesisHash"`(`Byron`/`Shelley`/`Alonzo`/`Conway`)、
//! `"defaultScribes"`/`"setupScribes"`/`"hasEKG"`/`"hasPrometheus"`/`"minSeverity"`/
//! `"TracingVerbosity"`/`"TurnOnLogging"`/`"RequiresNetworkMagic"` 等。
//! トップレベルのキーとその値種別(bool/string/number/array/object)を計数する。
//!
//! ```
//! let b = b"{\n  \"EnableP2P\": true,\n  \"PeerSharing\": true,\n  \"TargetNumberOfRootPeers\": 100,\n  \"ByronGenesisFile\": \"mainnet-byron-genesis.json\",\n  \"ShelleyGenesisFile\": \"mainnet-shelley-genesis.json\",\n  \"RequiresNetworkMagic\": \"RequiresNoMagic\",\n  \"TraceMux\": false,\n  \"defaultScribes\": [[\"StdoutSK\", \"stdout\"]],\n  \"minSeverity\": \"Info\"\n}\n";
//! assert!(izanagi_kit::cardanoconf::detect(b));
//! let c = izanagi_kit::cardanoconf::parse(b).unwrap();
//! assert_eq!(c.entries, 9);
//! assert_eq!(c.trace_keys, 1);
//! assert_eq!(c.genesis_keys, 2);
//! assert_eq!(c.bool_values, 3);
//! assert_eq!(c.number_values, 1);
//! assert_eq!(c.string_values, 4);
//! assert_eq!(c.array_values, 1);
//! ```

use crate::textutil::strip_bom;
/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベル `"key": value` エントリ数。
    pub entries: usize,
    /// `Trace*` プレフィックスのキー数。
    pub trace_keys: usize,
    /// `*GenesisFile`/`*GenesisHash` キー数。
    pub genesis_keys: usize,
    /// 値が `true`/`false` のエントリ数。
    pub bool_values: usize,
    /// 値が文字列のエントリ数。
    pub string_values: usize,
    /// 値が数値のエントリ数。
    pub number_values: usize,
    /// 値が配列のエントリ数。
    pub array_values: usize,
    /// 値がオブジェクトのエントリ数。
    pub object_values: usize,
}

fn key_hint(k: &str) -> bool {
    k.starts_with("Trace")
        || k.ends_with("GenesisFile")
        || k.ends_with("GenesisHash")
        || k.starts_with("TargetNumberOf")
        || matches!(
            k,
            "EnableP2P"
                | "PeerSharing"
                | "defaultScribes"
                | "setupScribes"
                | "hasEKG"
                | "hasPrometheus"
                | "hasGraylog"
                | "minSeverity"
                | "TracingVerbosity"
                | "TurnOnLogging"
                | "TurnOnLogMetrics"
                | "RequiresNetworkMagic"
                | "ApplicationName"
                | "ApplicationVersion"
                | "DefaultPeerTimeout"
                | "DiffusionMode"
                | "useTraceDispatcher"
        )
}

/// テキスト `i` 位置の JSON 文字列終端(閉じ `"` の次)を返す。
fn str_end(t: &str, i: usize) -> Option<usize> {
    let mut j = i + 1;
    let bs = t.as_bytes();
    while j < bs.len() {
        if bs[j] == b'\\' {
            j += 2;
            continue;
        }
        if bs[j] == b'"' {
            return Some(j + 1);
        }
        j += 1;
    }
    None
}

fn skip_ws(t: &str, mut i: usize) -> usize {
    let bs = t.as_bytes();
    while i < bs.len() && (bs[i] == b' ' || bs[i] == b'\t' || bs[i] == b'\n' || bs[i] == b'\r') {
        i += 1;
    }
    i
}

/// `"…"` 文字列直後に `:` が続く位置だけを列挙する(トップレベル)。
fn top_entries(t: &str) -> std::vec::Vec<(std::string::String, char)> {
    let mut out = std::vec::Vec::new();
    let bs = t.as_bytes();
    let mut i = 0usize;
    let mut depth = 0i64;
    while i < bs.len() {
        match bs[i] {
            b'"' => {
                let Some(end) = str_end(t, i) else {
                    break;
                };
                let mut next = end;
                if depth == 1 {
                    let j = skip_ws(t, end);
                    if j < bs.len() && bs[j] == b':' {
                        let k = &t[i + 1..end - 1];
                        let v = skip_ws(t, j + 1);
                        let kind = match bs.get(v) {
                            Some(b't') | Some(b'f') => 'b',
                            Some(b'[') => 'a',
                            Some(b'{') => 'o',
                            Some(ch) if ch.is_ascii_digit() || *ch == b'-' => 'n',
                            _ => 's',
                        };
                        out.push((k.to_string(), kind));
                        next = v;
                    }
                }
                i = next;
            }
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

/// Cardano node config らしさを返す。JSON オブジェクト + 既知キー ≥2。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let t = t.trim_start();
    if !t.starts_with('{') {
        return false;
    }
    top_entries(t).iter().filter(|(k, _)| key_hint(k)).count() >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let Ok(t) = std::str::from_utf8(b) else {
        return None;
    };
    let t = strip_bom(t);
    let mut c = Counts {
        entries: 0,
        trace_keys: 0,
        genesis_keys: 0,
        bool_values: 0,
        string_values: 0,
        number_values: 0,
        array_values: 0,
        object_values: 0,
    };
    for (k, kind) in top_entries(t) {
        c.entries += 1;
        if k.starts_with("Trace") {
            c.trace_keys += 1;
        }
        if k.ends_with("GenesisFile") || k.ends_with("GenesisHash") {
            c.genesis_keys += 1;
        }
        match kind {
            'b' => c.bool_values += 1,
            's' => c.string_values += 1,
            'n' => c.number_values += 1,
            'a' => c.array_values += 1,
            _ => c.object_values += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cardano() {
        let b = b"{\"TraceChainDb\": true, \"ShelleyGenesisFile\": \"x.json\", \"AlonzoGenesisHash\": \"h\", \"minSeverity\": \"Info\", \"TargetNumberOfActivePeers\": 20}";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 5);
        assert_eq!(c.trace_keys, 1);
        assert_eq!(c.genesis_keys, 2);
        assert_eq!(c.number_values, 1);
    }

    #[test]
    fn rejects_other_json() {
        assert!(parse(b"{\"name\": \"x\", \"version\": 1}").is_none());
        assert!(parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
