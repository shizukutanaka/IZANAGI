//! Singer(Singer spec JSONL)メッセージストリームの検出と構造カウント。
//!
//! 1行1メッセージの JSONL: `{"type": "SCHEMA"|"RECORD"|"STATE"|"ACTIVATE_VERSION"|
//! "METRIC"|"BATCH"}` を識別し、スキーマ/レコード/状態メッセージを別カウントする。
//!
//! ```
//! let c = izanagi_kit::singerconf::parse(
//!     b"{\"type\": \"SCHEMA\", \"stream\": \"users\", \"schema\": {\"properties\": {\"id\": {\"type\": \"integer\"}}}, \"key_properties\": [\"id\"]}\n{\"type\": \"RECORD\", \"stream\": \"users\", \"record\": {\"id\": 1}}\n{\"type\": \"STATE\", \"value\": {\"users\": 1}}\n").unwrap();
//! assert_eq!(c.schemas, 1);
//! assert_eq!(c.records, 1);
//! assert_eq!(c.states, 1);
//! assert!(izanagi_kit::singerconf::detect(
//!     b"{\"type\":\"SCHEMA\",\"stream\":\"s\",\"schema\":{}}\n{\"type\":\"STATE\",\"value\":{}}\n"));
//! ```

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// SCHEMA メッセージ数。
    pub schemas: usize,
    /// RECORD メッセージ数。
    pub records: usize,
    /// STATE メッセージ数。
    pub states: usize,
    /// その他 Singer メッセージ数(ACTIVATE_VERSION/METRIC/BATCH 等)。
    pub messages: usize,
    /// その他の行数。
    pub misc: usize,
}

/// 行の Singer メッセージ種別(0=非, 1=SCHEMA, 2=RECORD, 3=STATE, 4=その他既知型)。
fn line_kind(t: &str) -> u8 {
    if !(t.starts_with('{') && t.contains("\"type\"")) {
        return 0;
    }
    if t.contains("\"type\":\"SCHEMA\"") || t.contains("\"type\": \"SCHEMA\"") {
        1
    } else if t.contains("\"type\":\"RECORD\"") || t.contains("\"type\": \"RECORD\"") {
        2
    } else if t.contains("\"type\":\"STATE\"") || t.contains("\"type\": \"STATE\"") {
        3
    } else if t.contains("\"type\":\"ACTIVATE_VERSION\"")
        || t.contains("\"type\": \"ACTIVATE_VERSION\"")
        || t.contains("\"type\":\"METRIC\"")
        || t.contains("\"type\": \"METRIC\"")
        || t.contains("\"type\":\"BATCH\"")
        || t.contains("\"type\": \"BATCH\"")
    {
        4
    } else {
        0
    }
}

/// Singer ストリームらしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if line_kind(t) != 0 {
            hits += 1;
            if hits >= 2 {
                return true;
            }
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        schemas: 0,
        records: 0,
        states: 0,
        messages: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        match line_kind(t) {
            1 => c.schemas += 1,
            2 => c.records += 1,
            3 => c.states += 1,
            4 => c.messages += 1,
            _ => c.misc += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\"type\": \"SCHEMA\", \"stream\": \"users\", \"schema\": {\"properties\": {\"id\": {\"type\": \"integer\"}, \"email\": {\"type\": \"string\"}}}, \"key_properties\": [\"id\"]}\n{\"type\": \"RECORD\", \"stream\": \"users\", \"record\": {\"id\": 1, \"email\": \"a@b\"}}\n{\"type\": \"RECORD\", \"stream\": \"users\", \"record\": {\"id\": 2, \"email\": \"c@d\"}}\n{\"type\": \"ACTIVATE_VERSION\", \"stream\": \"users\", \"version\": 1}\n{\"type\": \"STATE\", \"value\": {\"bookmarks\": {\"users\": 2}}}\n{\"type\": \"METRIC\", \"metric\": \"counter\", \"value\": 2}\n{\"not\": \"singer\"}\n";

    #[test]
    fn singerconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.schemas, 1);
        assert_eq!(c.records, 2);
        assert_eq!(c.states, 1);
        assert_eq!(c.messages, 2);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_singer() {
        assert!(!detect(b"{\"a\":1}\n{\"b\":2}\n"));
        assert!(parse(b"text\n").is_none());
    }
}
