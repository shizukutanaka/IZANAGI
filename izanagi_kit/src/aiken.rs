//! Moodle Aiken import format — blocks of `QUESTION` text lines
//! followed by `A. choice` lines and a final `ANSWER: X` line.
//!
//! ```
//! use izanagi_kit::aiken::parse;
//!
//! let q = parse("The sky is?\nA. Green\nB. Blue\nC. Red\nANSWER: B\n").unwrap();
//! assert_eq!(q[0].answer, 'B');
//! ```

use std::string::String;
use std::vec::Vec;

/// One Aiken question.
#[derive(Clone, Debug)]
pub struct Question {
    /// Stem text (all lines before the first option, joined by `\n`).
    pub stem: String,
    /// Options `(letter, text)` in order.
    pub options: Vec<(char, String)>,
    /// Answer letter.
    pub answer: char,
}

fn option_line(line: &str) -> Option<(char, String)> {
    if line.len() < 3 {
        return None;
    }
    let c = line.as_bytes()[0];
    if !c.is_ascii_uppercase() || line.as_bytes()[1] != b'.' {
        return None;
    }
    let rest = line[2..].trim_start();
    Some((c as char, rest.to_string()))
}

/// Parse an Aiken file (blank line separates questions).
pub fn parse(text: &str) -> Option<Vec<Question>> {
    let mut out = Vec::new();
    let mut stem: Vec<String> = Vec::new();
    let mut options: Vec<(char, String)> = Vec::new();
    let mut answer: Option<char> = None;
    let mut flush = |stem: &mut Vec<String>,
                     options: &mut Vec<(char, String)>,
                     answer: &mut Option<char>|
     -> Option<()> {
        if stem.is_empty() && options.is_empty() {
            *answer = None;
            return Some(());
        }
        let a = answer.take()?;
        if options.is_empty() || !options.iter().any(|(c, _)| *c == a) {
            return None;
        }
        out.push(Question {
            stem: stem.join("\n"),
            options: std::mem::take(options),
            answer: a,
        });
        stem.clear();
        Some(())
    };
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            flush(&mut stem, &mut options, &mut answer)?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("ANSWER:") {
            let a = rest.trim();
            if a.len() != 1 || !a.as_bytes()[0].is_ascii_uppercase() {
                return None;
            }
            answer = Some(a.as_bytes()[0] as char);
            continue;
        }
        if let Some((c, t)) = option_line(line) {
            if answer.is_some() || options.iter().any(|(o, _)| *o == c) {
                return None;
            }
            options.push((c, t));
            continue;
        }
        if !options.is_empty() || answer.is_some() {
            return None; // stem text after options is illegal
        }
        stem.push(line.to_string());
    }
    flush(&mut stem, &mut options, &mut answer)?;
    if out.is_empty() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_and_multi() {
        let q = parse(
            "Capital of France\nA. London\nB. Paris\nANSWER: B\n\nSecond?\nA. X\nB. Y\nANSWER: A\n",
        )
        .unwrap();
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].stem, "Capital of France");
        assert_eq!(q[0].options[1].1, "Paris");
        assert_eq!(q[1].answer, 'A');
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("Q\nA. x\n").is_none()); // no ANSWER
        assert!(parse("Q\nA. x\nANSWER: Z\n").is_none()); // answer not an option
        assert!(parse("Q\nA. x\nB. y\nANSWER: a\n").is_none()); // lowercase
        assert!(parse("A. oops\nQ\nANSWER: A\n").is_none()); // option before stem
    }
}
