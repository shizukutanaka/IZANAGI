//! CHIRP `.csv` (メモリチャンネルエクスポート) 検出モジュール。
//!
//! CHIRP (無線機メモリ管理) の CSV エクスポートは先頭行が
//! `Location,Name,Frequency,Duplex,Offset,Tone,rToneFreq,cToneFreq,
//! DtcsCode,DtcsPolarity,Mode,TStep,Skip,Comment,URCALL,RPT1CALL,
//! RPT2CALL` の既知カラム名で構成される。
//!
//! ```
//! let b = br#"Location,Name,Frequency,Duplex,Offset,Tone,rToneFreq,cToneFreq,DtcsCode,DtcsPolarity,Mode,TStep,Skip,Comment
//! 1,CH1,146.520000,,0.000000,,88.5,88.5,023,NN,FM,5.00,,Tokyo
//! "#;
//! let c = izanagi_kit::chirpcsv::parse(b);
//! assert!(izanagi_kit::chirpcsv::detect(b));
//! assert_eq!(c.header_cols, 14);
//! ```

const COLUMNS: &[&str] = &[
    "Bank",
    "Bank Index",
    "Bank Index Low",
    "Bank Index High",
    "cToneFreq",
    "Comment",
    "CrossMode",
    "DtcsCode",
    "DtcsPolarity",
    "Duplex",
    "DTCS Rx Code",
    "Frequency",
    "Location",
    "Mode",
    "Name",
    "Offset",
    "Power",
    "RPT1CALL",
    "RPT2CALL",
    "rToneFreq",
    "Rx DTCS Code",
    "Skip",
    "Tone",
    "ToneSql",
    "TStep",
    "Tx DTCS Code",
    "URCALL",
];

fn is_header_line(t: &str) -> usize {
    t.split(',').filter(|c| COLUMNS.contains(&c.trim())).count()
}

/// `b` が CHIRP CSV に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let first = t.lines().find(|l| !l.trim().is_empty());
    match first {
        Some(l) => is_header_line(l.trim()) >= 5,
        None => false,
    }
}

/// CHIRP CSV の統計。
#[derive(Debug, Default, Clone)]
pub struct ChirpCsv {
    /// ヘッダー行内の既知カラム数。
    pub header_cols: usize,
    /// データ行数。
    pub data_rows: usize,
}

/// `b` を CHIRP CSV として統計する。
pub fn parse(b: &[u8]) -> ChirpCsv {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = ChirpCsv::default();
    let mut it = t.lines().filter(|l| !l.trim().is_empty());
    if let Some(h) = it.next() {
        c.header_cols = is_header_line(h.trim());
    }
    c.data_rows = it.count();
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"Location,Name,Frequency,Duplex,Offset,Tone,rToneFreq,cToneFreq,DtcsCode,DtcsPolarity,Mode,TStep,Skip,Comment
1,CH1,146.52,,0,,88.5,88.5,023,NN,FM,5.00,,
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.data_rows, 1);
    }

    #[test]
    fn detects_minimal_header() {
        let b = br#"Location,Name,Frequency,Duplex,Offset
1,A,145.0,,"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.header_cols, 5);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"Name,Age,City\nTaro,30,Tokyo\n"));
        assert!(!detect(b"Location,Name\n1,A\n"));
        assert!(!detect(b"Location Name Frequency\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.header_cols, 0);
    }
}
