//! Moodle GIFT format — `// comments`, `::title::` question labels,
//! `{=correct ~wrong}` answer blocks, `####general feedback`,
//! `->` fill-in... only the structural subset: title, stem text and the
//! `{...}` answer list (true/false, multiple choice, matching, numeric).
//!
//! ```
//! use izanagi_kit::gift::parse;
//!
//! let q = parse("::q1::The sky is blue. {=TRUE ~FALSE}\n").unwrap();
//! assert_eq!(q[0].title.as_deref(), Some("q1"));
//! assert_eq!(q[0].answers.len(), 2);
//! ```

use std::string::String;
use std::vec::Vec;

/// One parsed GIFT question.
#[derive(Clone, Debug)]
pub struct Question {
    /// Optional `::title::` label.
    pub title: Option<String>,
    /// Stem text (before the `{` block; may be empty for `{T}` forms).
    pub stem: String,
    /// Answer entries `(marker, text)`: `=` correct, `~` wrong,
    /// `#` numeric-tolerance, `%` fraction, `=`/`~` inside `->` pairs.
    pub answers: Vec<Answer>,
}

/// One `{...}` entry.
#[derive(Clone, Debug)]
pub struct Answer {
    /// True when the marker is `=` (correct) or the text is `TRUE`.
    pub correct: bool,
    /// Fraction weight for `%x%` entries (percent points ×1000).
    pub weight: i64,
    /// Answer text (escapes `\=`, `\~`, `\#`, `\}` un-escaped).
    pub text: String,
}

fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            if let Some(n) = it.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse a GIFT file (one question per `{...}` block).
pub fn parse(text: &str) -> Option<Vec<Question>> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut title: Option<String> = None;
    let mut stem = String::new();
    while i < b.len() {
        match b[i] {
            b'/' if b[i..].starts_with(b"//") => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'#' if b[i..].starts_with(b"####") => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b':' if i + 1 < b.len() && b[i + 1] == b':' => {
                let start = i + 2;
                let end = b[start..]
                    .windows(2)
                    .position(|w| w == b"::")
                    .map(|p| start + p)?;
                title = Some(unesc(&text[start..end]));
                i = end + 2;
            }
            b'{' => {
                let depth_start = i;
                i += 1;
                let mut answers = Vec::new();
                loop {
                    while i < b.len()
                        && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r')
                    {
                        i += 1;
                    }
                    if i >= b.len() {
                        return None;
                    }
                    if b[i] == b'}' {
                        i += 1;
                        break;
                    }
                    let marker = match b[i] {
                        b'=' => true,
                        b'~' => false,
                        b'%' => {
                            // %NN%weight%text
                            i += 1;
                            let e = b[i..].iter().position(|c| *c == b'%')? + i;
                            let pct: i64 = text[i..e].trim().parse().ok()?;
                            i = e + 1;
                            let t0 = i;
                            let mut j = i;
                            while j < b.len() && b[j] != b'}' && b[j] != b'~' && b[j] != b'=' {
                                if b[j] == b'\\' {
                                    j += 1;
                                }
                                j += 1;
                            }
                            answers.push(Answer {
                                correct: pct > 0,
                                weight: pct,
                                text: unesc(text[t0..j].trim()),
                            });
                            i = j;
                            continue;
                        }
                        b'#' => {
                            // numeric with tolerance — marker falls through
                            i += 1;
                            let t0 = i;
                            while i < b.len() && b[i] != b'}' && b[i] != b'~' && b[i] != b'=' {
                                if b[i] == b'\\' {
                                    i += 1;
                                }
                                i += 1;
                            }
                            answers.push(Answer {
                                correct: true,
                                weight: 1000,
                                text: unesc(text[t0..i].trim()),
                            });
                            continue;
                        }
                        _ => {
                            // bare word: TRUE/FALSE single answer
                            let t0 = i;
                            while i < b.len() && b[i] != b'}' && b[i] != b'~' && b[i] != b'=' {
                                i += 1;
                            }
                            let w = text[t0..i].trim();
                            answers.push(Answer {
                                correct: w == "TRUE" || w == "T",
                                weight: if w == "TRUE" || w == "T" { 1000 } else { 0 },
                                text: unesc(w),
                            });
                            continue;
                        }
                    };
                    i += 1;
                    let t0 = i;
                    while i < b.len() && b[i] != b'}' && b[i] != b'~' && b[i] != b'=' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    answers.push(Answer {
                        correct: marker,
                        weight: if marker { 1000 } else { 0 },
                        text: unesc(text[t0..i].trim()),
                    });
                }
                let _ = depth_start;
                out.push(Question {
                    title: title.take(),
                    stem: stem.trim().to_string(),
                    answers,
                });
                stem.clear();
            }
            _ => {
                // accumulate stem text until `{`, `}`, comment, or EOF
                let t0 = i;
                while i < b.len() && b[i] != b'{' && b[i] != b'}' && b[i] != b':' {
                    if b[i] == b'/' && b[i + 1..].first() == Some(&b'/') {
                        break;
                    }
                    i += 1;
                }
                let chunk = text[t0..i].split_whitespace().collect::<Vec<_>>().join(" ");
                if !chunk.is_empty() {
                    if !stem.is_empty() {
                        stem.push(' ');
                    }
                    stem.push_str(&chunk);
                }
            }
        }
        while i < b.len() && (b[i] == b'\n' || b[i] == b'\r' || b[i] == b' ' || b[i] == b'\t') {
            i += 1;
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tf_and_mc() {
        let q = parse("// c\n::tf::Sky is blue. {TRUE}\n::mc::Pick {=right ~wrong1 ~wrong2}\n")
            .unwrap();
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].title.as_deref(), Some("tf"));
        assert_eq!(q[0].answers[0].text, "TRUE");
        assert!(q[0].answers[0].correct);
        assert_eq!(q[1].answers.iter().filter(|a| a.correct).count(), 1);
    }

    #[test]
    fn numeric_and_escaped() {
        let q = parse("When? {#1970 :1}\nEsc {=a\\~b ~x}\n").unwrap();
        assert_eq!(q[0].answers[0].text, "1970 :1");
        assert_eq!(q[1].answers[0].text, "a~b");
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("no braces at all\n").is_none());
        assert!(parse("{=never closed").is_none());
    }
}
