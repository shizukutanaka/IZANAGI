//! IMS QTI 2.x assessment items — `<assessmentItem>` with
//! `<choiceInteraction>` / `<simpleChoice>` answers, plus
//! `<responseDeclaration>` correct-answer mapping.
//!
//! ```
//! use izanagi_kit::qti::parse;
//!
//! let x = br#"<assessmentItem><itemBody><choiceInteraction responseIdentifier="R">
//!   <prompt>Pick one</prompt><simpleChoice identifier="a">A</simpleChoice>
//!   <simpleChoice identifier="b">B</simpleChoice></choiceInteraction></itemBody>
//!   <responseDeclaration identifier="R"><correctResponse><value>b</value></correctResponse></responseDeclaration>
//!   </assessmentItem>"#;
//! let q = parse(x).unwrap();
//! assert_eq!(q.choices.len(), 2);
//! assert_eq!(q.correct, vec!["b"]);
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed QTI assessment item.
#[derive(Clone, Debug)]
pub struct Item {
    /// `identifier` attribute when present.
    pub identifier: Option<String>,
    /// `<prompt>` text (first one).
    pub prompt: String,
    /// `<simpleChoice>` entries `(identifier, text)`.
    pub choices: Vec<(String, String)>,
    /// `<responseDeclaration>` correct values.
    pub correct: Vec<String>,
}

fn find_tag(d: &[u8], tag: &[u8], from: usize) -> Option<(usize, usize)> {
    // returns (start-of-open-tag, position after `>` of open tag)
    let mut i = from;
    loop {
        let p = d[i..]
            .windows(tag.len() + 1)
            .position(|w| w[0] == b'<' && &w[1..] == tag)
            .map(|p| i + p)?;
        let after = p + tag.len() + 1;
        // boundary: next byte must not be name-char (avoids <simpleChoiceX>)
        match d.get(after) {
            Some(c) if c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-' || *c == b':' => {
                i = p + 1;
                continue;
            }
            None => return None,
            _ => {}
        }
        let gt = d[after..].iter().position(|c| *c == b'>')? + after;
        return Some((p, gt + 1));
    }
}

fn find_close(d: &[u8], tag: &[u8], from: usize) -> Option<usize> {
    // position of `</tag`
    let mut pat = Vec::with_capacity(tag.len() + 2);
    pat.push(b'<');
    pat.push(b'/');
    pat.extend_from_slice(tag);
    d[from..]
        .windows(pat.len())
        .position(|w| w == pat.as_slice())
        .map(|p| from + p)
}

fn attr(tag_bytes: &[u8], name: &[u8]) -> Option<String> {
    let mut pat = Vec::new();
    pat.extend_from_slice(name);
    pat.push(b'=');
    pat.push(b'"');
    let at = tag_bytes
        .windows(pat.len())
        .position(|w| w == pat.as_slice())?;
    let v0 = at + pat.len();
    let e = tag_bytes[v0..].iter().position(|c| *c == b'"')? + v0;
    Some(String::from_utf8_lossy(&tag_bytes[v0..e]).into_owned())
}

/// Parse a QTI `<assessmentItem>` document.
pub fn parse(d: &[u8]) -> Option<Item> {
    let (ai, ai_body) = find_tag(d, b"assessmentItem", 0)?;
    let identifier = attr(&d[ai..ai_body], b"identifier");

    let mut item = Item {
        identifier,
        prompt: String::new(),
        choices: Vec::new(),
        correct: Vec::new(),
    };

    if let Some((_, pb)) = find_tag(d, b"prompt", ai_body) {
        if let Some(pe) = find_close(d, b"prompt", pb) {
            item.prompt = String::from_utf8_lossy(&d[pb..pe])
                .split('<')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
        }
    }

    let mut i = ai_body;
    while let Some((ts, tb)) = find_tag(d, b"simpleChoice", i) {
        let id = attr(&d[ts..tb], b"identifier").unwrap_or_default();
        let te = find_close(d, b"simpleChoice", tb)?;
        let text = String::from_utf8_lossy(&d[tb..te]).trim().to_string();
        item.choices.push((id, text));
        i = te;
    }

    if let Some((_, rb)) = find_tag(d, b"correctResponse", ai_body) {
        if let Some(re) = find_close(d, b"correctResponse", rb) {
            let mut j = rb;
            while let Some((_, vb)) = find_tag(d, b"value", j) {
                if vb > re {
                    break;
                }
                let ve = find_close(d, b"value", vb)?;
                item.correct
                    .push(String::from_utf8_lossy(&d[vb..ve]).trim().to_string());
                j = ve;
            }
        }
    }

    if item.choices.is_empty() && item.correct.is_empty() && item.prompt.is_empty() {
        return None;
    }
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_item() {
        let x = br#"<assessmentItem identifier="q1"><itemBody>
          <choiceInteraction><prompt>Cap?</prompt>
          <simpleChoice identifier="a">London</simpleChoice>
          <simpleChoice identifier="b">Paris</simpleChoice></choiceInteraction></itemBody>
          <responseDeclaration><correctResponse><value>b</value></correctResponse></responseDeclaration>
          </assessmentItem>"#;
        let q = parse(x).unwrap();
        assert_eq!(q.identifier.as_deref(), Some("q1"));
        assert_eq!(q.prompt, "Cap?");
        assert_eq!(q.choices[1], ("b".to_string(), "Paris".to_string()));
        assert_eq!(q.correct, vec!["b".to_string()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<notqti/>").is_none());
        // no choices/prompt/correct → None
        assert!(parse(b"<assessmentItem></assessmentItem>").is_none());
    }
}
