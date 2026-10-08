//! Precision ratchet for canonical *generic* snippets — tiny inputs that
//! carry no format-exclusive vocabulary (`name = value`, `set foo bar`, …).
//! Each detector that fires on one of these is doing keyword-only matching;
//! the counts are snapshotted so they can only go down.

/// `(label, bytes, max allowed detector hits)`.
const SNIPPETS: &[(&str, &[u8], usize)] = &[
    ("name = value", b"name = value\n", 3),
    ("key: value", b"key: value\n", 1),
    ("[section] + kv", b"[section]\nkey = value\n", 5),
    ("comment + kv", b"# comment\nfoo = bar\n", 5),
    ("three words", b"foo bar baz\n", 2),
    ("option line", b"option foo bar\n", 2),
    ("set line", b"set foo bar\n", 3),
    ("json object", b"{\n  \"key\": \"value\"\n}\n", 1),
    ("python-ish", b"import foo\nprint(1)\n", 0),
    ("package main", b"package main\n", 1),
    ("csv pair", b"a,b,c\n1,2,3\n", 1),
    ("brace block", b"x { y }\n", 1),
];

#[test]
fn generic_snippets_stay_below_ceilings() {
    let mut over: Vec<String> = Vec::new();
    for (label, data, cap) in SNIPPETS {
        let hits: Vec<&str> = izanagi_kit::DETECTORS
            .iter()
            .filter(|(_, d)| d(data))
            .map(|(n, _)| *n)
            .collect();
        if hits.len() > *cap {
            over.push(format!(
                "{label:?}: {} hits {hits:?} (ceiling {cap})",
                hits.len()
            ));
        }
    }
    assert!(over.is_empty(), "{}", over.join("\n"));
}
