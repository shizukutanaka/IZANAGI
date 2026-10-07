//! Narrowing-cast contract: a `.len()`/`.count()` narrowed by `as u8`
//! or `as u16` silently wraps once the value passes 255/65535. For a
//! decode path that lies about the census; for an encode path it emits
//! a structurally corrupt file — `ZipWriter` wrote a `u16` name length
//! for a longer name and `emit_page` wrapped a 256-entry lacing table
//! into a zero count byte.
//!
//! The sweep flags every such cast in production code (everything above
//! `#[cfg(test)]`, excluding doc comments and `>>`-masked byte
//! extraction, where the cast deliberately selects one byte of a wider
//! field). A site survives only by appearing in `KNOWN_BOUNDED` with
//! the bound that keeps it honest — so a *new* unguarded narrowing
//! fails this test instead of shipping silently.
//!
//! `as u32` casts are out of scope: they only misbehave past 4 GiB of
//! input, which no caller can feed these parsers meaningfully.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// `(file, exact trimmed line)` → the bound that makes the cast safe.
/// Every entry must name a guard a reader can verify in that file.
fn known_bounded() -> BTreeMap<(&'static str, &'static str), ()> {
    BTreeMap::from([
        // tmp is the byte length of a DER length field: at most ~9
        // bytes by construction.
        (("der.rs", "out.push(0x80 | tmp.len() as u8);"), ()),
        // push_name returns None for any label over 63 bytes (DNS max).
        (("dns.rs", "out.push(label.len() as u8);"), ()),
        // header_block returns None for any name over 63 bytes.
        (("hqx.rs", "b.push(h.name.len() as u8);"), ()),
        // the count already fit in a u16 on the wire.
        (
            ("llmnr.rs", "question_count: m.questions.len() as u16,"),
            (),
        ),
        // written only inside the `b.len() <= 0xff` branch.
        (("msgpack.rs", "out.push(b.len() as u8);"), ()),
        // emit_page returns None for a lacing table over 255 entries.
        (("ogg.rs", "out.push(segments.len() as u8);"), ()),
        // low byte of a four-byte u32 length split across lines.
        (("ssh.rs", "f.len() as u8,"), ()),
        // emit refuses fragments over the 16384-byte protocol cap.
        (("tls.rs", "out.push(fragment.len() as u8);"), ()),
        // present is the codebook's symbol set: at most 256 entries,
        // so count-1 always fits a byte (count of 256 is why -1).
        (
            (
                "huffman.rs",
                "out.push((present.len() - 1) as u8); // count-1 so 256 fits",
            ),
            (),
        ),
        // chunks are cut at PAYLOAD_MAX (45) before encoding.
        (("uue.rs", "out.push(0x20 + chunk.len() as u8);"), ()),
        // add_method refuses names over u16::MAX and a full entry
        // table, so both writers only see representable values.
        (("zip.rs", "push16(&mut h, name.len() as u16);"), ()),
        (
            ("zip.rs", "push16(&mut out, self.entries.len() as u16);"),
            (),
        ),
    ])
}

/// Byte offsets in `line` where a `.len()`/`.count()` is narrowed by
/// `as u8`/`as u16` with only arithmetic dressing (`- 1`, `+ 2`, a
/// closing paren) in between. A `.len()` used as an index (`d[..]`) or
/// feeding a different call (`.len().min(..)`) does not match — the
/// `as` there applies to something else.
fn narrowing_positions(line: &str) -> Vec<usize> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    for needle in [".len()", ".count()"] {
        let mut at = 0;
        while let Some(p) = line[at..].find(needle) {
            let mut i = at + p + needle.len();
            while i < b.len() && matches!(b[i], b' ' | b'\t' | b'0'..=b'9' | b'-' | b'+' | b')') {
                i += 1;
            }
            if line[i..].starts_with("as u8") || line[i..].starts_with("as u16") {
                out.push(at + p);
            }
            at += p + needle.len();
        }
    }
    out
}

#[test]
fn no_unguarded_narrowing_casts() {
    let known = known_bounded();
    let mut violations: Vec<String> = Vec::new();
    for entry in fs::read_dir(src_dir()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if !name.ends_with(".rs") {
            continue;
        }
        let code = fs::read_to_string(src_dir().join(&name)).unwrap();
        let prod = code.split("#[cfg(test)]").next().unwrap();
        for (n, line) in prod.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("//!") || t.starts_with("///") || t.contains(">>") {
                continue;
            }
            for pos in narrowing_positions(t) {
                let _ = pos;
                if !known.contains_key(&(name.as_str(), t)) {
                    violations.push(format!("{name}:{}: {t}", n + 1));
                    break;
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "unguarded narrowing casts (add a bound check, then record the site):\n{}",
        violations.join("\n")
    );
}

/// The allowlist itself must stay honest: an entry pointing at a line
/// that no longer exists means the guard review was deleted with it.
#[test]
fn known_bounded_sites_still_exist() {
    let known = known_bounded();
    let mut missing = Vec::new();
    for (file, line) in known.keys() {
        let code = fs::read_to_string(src_dir().join(file)).unwrap();
        if !code
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .lines()
            .any(|l| l.trim() == *line)
        {
            missing.push(format!("{file}: {line}"));
        }
    }
    assert!(
        missing.is_empty(),
        "allowlisted cast sites vanished — re-verify the bound:\n{}",
        missing.join("\n")
    );
}
