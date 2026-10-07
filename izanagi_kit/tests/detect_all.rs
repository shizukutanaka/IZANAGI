//! `detect_all` (P1: census 集約 API)の契約テスト。
//!
//! - 返す名前は全て `DETECTORS` 登録済みであり、その detect が実際に合致する
//! - 全件総当りと結果が一致する(集約実装の手抜き・重複・欠落を検出)
//! - 既知入力で期待するモジュール名を含む

use izanagi_kit::{detect_all, detector_by_name, parser_by_name, DETECTORS, PARSERS};

/// 手作業入力の小コーパス。panic 非発火は detect_never_panics が担保済みなので
/// ここでは集約の正しさのみを見る。
fn cases() -> Vec<Vec<u8>> {
    vec![
        Vec::new(),
        b"\x00".to_vec(),
        b"{\"key\": \"value\"}\n".to_vec(),
        b"local x = self.y\nlocal z = std.extVar('a')\n".to_vec(),
        b"[section]\nkey = value\n".to_vec(),
        b"#!/bin/sh\necho hello\n".to_vec(),
        b"PK\x03\x04\x14\x00\x00\x00\x08\x00".to_vec(),
        b"\x89PNG\r\n\x1a\n".to_vec(),
        vec![0xFF; 4096],
    ]
}

#[test]
fn detect_all_returns_only_registered_names_that_agree() {
    for input in cases() {
        for name in detect_all(&input) {
            let (_, f) = DETECTORS
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("{name} not in DETECTORS"));
            assert!(f(&input), "{name} listed but detect() disagrees");
        }
    }
}

#[test]
fn detect_all_matches_exhaustive_sweep() {
    for input in cases() {
        let manual: Vec<&str> = DETECTORS
            .iter()
            .filter(|(_, f)| f(&input))
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(detect_all(&input), manual);
    }
}

#[test]
fn detect_all_is_sorted_and_duplicate_free() {
    for input in cases() {
        let hits = detect_all(&input);
        let mut sorted = hits.clone();
        sorted.sort_unstable();
        assert_eq!(hits, sorted, "detect_all must preserve DETECTORS order");
        sorted.dedup();
        assert_eq!(hits.len(), sorted.len(), "duplicate name in {hits:?}");
    }
}

#[test]
fn detect_all_finds_a_known_format() {
    // `local ` + `self.`/`std.` は jsonnet::detect の必須・十分マーカー。
    let hits = detect_all(b"local x = self.y\nlocal z = std.extVar('a')\n");
    assert!(hits.contains(&"jsonnet"), "expected jsonnet in {hits:?}");
}

#[test]
fn detect_all_result_count_never_exceeds_registry() {
    for input in cases() {
        assert!(detect_all(&input).len() <= DETECTORS.len());
    }
}

#[test]
fn detectors_table_is_strictly_sorted() {
    // `detector_by_name` は二分探索に依存する — 昇順が崩れると
    // 誤った None を返しうるので整列性をテストで固定する。
    for w in DETECTORS.windows(2) {
        assert!(
            w[0].0 < w[1].0,
            "DETECTORS not sorted at {:?} / {:?}",
            w[0].0,
            w[1].0
        );
    }
}

#[test]
fn detector_by_name_finds_every_registered_detector() {
    for (name, f) in DETECTORS {
        let got = detector_by_name(name).unwrap_or_else(|| panic!("{name} not found"));
        // 関数ポインタ比較は不安定なので挙動で同一性を見る。
        for input in cases() {
            assert_eq!(
                got(&input),
                f(&input),
                "detector_by_name({name}) returned a different function"
            );
        }
    }
}

#[test]
fn parser_by_name_finds_every_registered_parser() {
    for (name, _) in PARSERS {
        assert!(parser_by_name(name).is_some(), "{name} not found");
    }
}

#[test]
fn by_name_helpers_return_none_for_unknown_names() {
    assert!(detector_by_name("no_such_format").is_none());
    assert!(detector_by_name("").is_none());
    assert!(detector_by_name("ZZZZZ").is_none());
    assert!(parser_by_name("no_such_format").is_none());
}
