//! RTF (Rich Text Format) plain-text extraction: walk the `{\rtf1 …}`
//! group tree honouring control words — `\par`/`\line` → newline,
//! `\tab` → tab, `\'hh` → a raw byte of the document codepage (kept
//! as Latin-1 here — the only lossless 8-bit map), `\uN` → the
//! Unicode scalar (the following `\uc` fallback characters are
//! skipped as the spec requires), and `\{`/`\}`/`\\` escapes → the
//! literal glyph. Destination groups (`\fonttbl`, `\colortbl`,
//! `\stylesheet`, `\info`, `\pict`, `\*\…`, headers/footers,
//! `xmlnstbl`…) contribute no text. [`text`] is total — `None` only
//! when the input does not start with `{\rtf`.
//!
//! ```
//! use izanagi_kit::rtf::text;
//! let doc = r"{\rtf1\ansi Hello \par {\b World}}";
//! assert_eq!(text(doc).unwrap(), "Hello \nWorld");
//! ```

/// Control words whose group carries no document text.
const DESTINATIONS: &[&str] = &[
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "object",
    "header",
    "footer",
    "headerl",
    "headerr",
    "headerf",
    "footerl",
    "footerr",
    "footerf",
    "footnote",
    "annotation",
    "xmlnstbl",
    "listtable",
    "listoverridetable",
    "revtbl",
    "rsidtbl",
    "generator",
    "datastore",
    "themedata",
    "colorschememapping",
    "latentstyles",
    "filetbl",
    "nonshppict",
    "shppict",
];

struct St {
    skip: bool,
    uc: usize,
    pend: usize,
}

/// Extract plain text from an RTF document; `None` when `rtf` does
/// not open with `{\rtf`. Malformed tails (unbalanced `}`, truncated
/// hex) degrade to whatever text was collected.
pub fn text(rtf: &str) -> Option<String> {
    let b = rtf.as_bytes();
    if !rtf.starts_with("{\\rtf") {
        return None;
    }
    let mut out = String::new();
    let mut stack: Vec<St> = vec![St {
        skip: false,
        uc: 1,
        pend: 0,
    }];
    let mut i = 1; // past '{'
    let mut at_group_start = false;
    while i < b.len() {
        match b[i] {
            b'{' => {
                let uc = stack.last().map(|s| s.uc).unwrap_or(1);
                stack.push(St {
                    skip: false,
                    uc,
                    pend: 0,
                });
                at_group_start = true;
                i += 1;
            }
            b'}' => {
                if stack.len() > 1 {
                    stack.pop();
                }
                at_group_start = false;
                i += 1;
            }
            b'\\' => {
                i += 1;
                if i >= b.len() {
                    break;
                }
                match b[i] {
                    b'{' | b'}' | b'\\' => {
                        emit_or_pend(&mut stack, &mut out, b[i] as char);
                        i += 1;
                    }
                    b'~' => {
                        emit_or_pend(&mut stack, &mut out, '\u{00A0}');
                        i += 1;
                    }
                    b'-' | b'_' => {
                        emit_or_pend(
                            &mut stack,
                            &mut out,
                            if b[i] == b'_' { '\u{2011}' } else { '-' },
                        );
                        i += 1;
                    }
                    b'*' => {
                        // ignorable-destination marker
                        if let Some(t) = stack.last_mut() {
                            t.skip = true;
                        }
                        i += 1;
                    }
                    b'\'' => {
                        let h = b.get(i + 1).copied().unwrap_or(0);
                        let l = b.get(i + 2).copied().unwrap_or(0);
                        if let (Some(hi), Some(lo)) = (hex(h), hex(l)) {
                            emit_or_pend(&mut stack, &mut out, (hi * 16 + lo) as u8 as char);
                        }
                        i += 3.min(b.len() - i);
                    }
                    _ => {
                        // control word: letters then optional signed digits
                        let st = i;
                        while i < b.len() && b[i].is_ascii_lowercase() {
                            i += 1;
                        }
                        let word = &rtf[st..i];
                        let num_st = i;
                        if i < b.len() && b[i] == b'-' {
                            i += 1;
                        }
                        while i < b.len() && b[i].is_ascii_digit() {
                            i += 1;
                        }
                        let num: i32 = rtf[num_st..i].parse().unwrap_or(0);
                        if i < b.len() && b[i] == b' ' {
                            i += 1; // the delimiter space is consumed
                        }
                        apply(&mut stack, &mut out, word, num, at_group_start);
                        at_group_start = false;
                    }
                }
            }
            b'\r' | b'\n' => {
                i += 1; // raw newlines are markup whitespace, not text
            }
            c => {
                emit_or_pend(&mut stack, &mut out, c as char);
                at_group_start = false;
                i += 1;
            }
        }
    }
    Some(out)
}

fn cur_skip(stack: &[St]) -> bool {
    stack.iter().any(|s| s.skip)
}

/// Push `c` unless a destination group suppresses output or a pending
/// `\u` fallback is still being consumed.
fn emit_or_pend(stack: &mut [St], out: &mut String, c: char) {
    if let Some(t) = stack.last_mut() {
        if t.pend > 0 {
            t.pend -= 1;
            return;
        }
    }
    if !cur_skip(stack) {
        out.push(c);
    }
}

fn hex(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a' + 10) as u32),
        b'A'..=b'F' => Some((c - b'A' + 10) as u32),
        _ => None,
    }
}

fn apply(stack: &mut [St], out: &mut String, w: &str, n: i32, group_start: bool) {
    if group_start && DESTINATIONS.contains(&w) {
        if let Some(t) = stack.last_mut() {
            t.skip = true;
        }
        return;
    }
    if w == "uc" && n >= 0 {
        if let Some(t) = stack.last_mut() {
            t.uc = n as usize;
        }
        return;
    }
    if cur_skip(stack) {
        return;
    }
    match w {
        "par" | "line" => out.push('\n'),
        "tab" => out.push('\t'),
        "u" => {
            let cp = n.rem_euclid(0x110000) as u32;
            if let Some(ch) = char::from_u32(cp) {
                out.push(ch);
            }
            if let Some(t) = stack.last_mut() {
                t.pend = t.uc;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        assert_eq!(text(r"{\rtf1 hello}").unwrap(), "hello");
        assert_eq!(text("not rtf"), None);
        assert_eq!(text("{\\rtf").unwrap(), "");
    }

    #[test]
    fn escapes() {
        assert_eq!(text(r"{\rtf1 a\{b\}c\\d}").unwrap(), "a{b}c\\d");
        assert_eq!(text(r"{\rtf1 caf\'e9}").unwrap(), "café");
    }

    #[test]
    fn paragraphs() {
        assert_eq!(
            text(r"{\rtf1 one\par two\tab three}").unwrap(),
            "one\ntwo\tthree"
        );
    }

    #[test]
    fn unicode_control() {
        // \u65? → 'A' plus a single skipped fallback char
        assert_eq!(text(r"{\rtf1 \u65?}").unwrap(), "A");
        // U+23376 = 子; the \'3f fallback is consumed by \uc bookkeeping
        assert_eq!(text(r"{\rtf1 \u23376\'3f}").unwrap(), "\u{5B50}");
        // \uc2: two fallback bytes
        assert_eq!(text(r"{\rtf1\uc2 \u945\'ce\'b1!}").unwrap(), "α!");
    }

    #[test]
    fn destinations_skipped() {
        let doc = r"{\rtf1{\fonttbl\f0 Arial;}plain{\colortbl;\red255\green0\blue0;}tail}";
        assert_eq!(text(doc).unwrap(), "plaintail");
        let ign = r"{\rtf1{\*\unknown data}kept}";
        assert_eq!(text(ign).unwrap(), "kept");
    }

    #[test]
    fn nested_groups() {
        let doc = r"{\rtf1 a{\b b{\i c}}d}";
        assert_eq!(text(doc).unwrap(), "abcd");
    }

    #[test]
    fn control_symbols() {
        // \~ is the NBSP control symbol; a bare ~ is literal text
        assert_eq!(text(r"{\rtf1 a~b\-c\_d}").unwrap(), "a~b-c\u{2011}d");
        assert_eq!(text(r"{\rtf1 a\~b}").unwrap(), "a\u{00A0}b");
    }

    #[test]
    fn determinism() {
        let doc = r"{\rtf1\ansi x \u945? y}";
        assert_eq!(text(doc), text(doc));
    }
}
