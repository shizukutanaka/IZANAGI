//! NEXUS (Maddison/Swofford/Maddison 1997, PAUP\\* / MrBayes /
//! Mesquite): `#NEXUS` first token (case-insensitive),
//! `begin <block>; … end;` block structure (`taxa`,
//! `characters`, `data`, `trees`, `sets`, `assumptions`,
//! `codons`, `notes`, `distances`, `unaligned`), commands
//! `dimensions ntax=… nchar=…;`, `matrix`, `tree <name> = (…)`,
//! `translate`.
//!
//! ```
//! let d = b"#NEXUS\nbegin taxa;\ndimensions ntax=3;\ntaxlabels a b c;\nend;\n\
//! begin trees;\ntree t1 = ((a,b),c);\nend;\n";
//! let n = izanagi_kit::nexus::parse(d).unwrap();
//! assert_eq!(n.blocks, 2);
//! assert_eq!(n.trees, 1);
//! assert_eq!(n.ntax, Some(3));
//! assert!(izanagi_kit::nexus::detect(d));
//! ```

/// Census of a NEXUS stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Nexus {
    /// `begin … end` blocks seen.
    pub blocks: u32,
    /// Distinct block names (lowercased).
    pub block_names: u32,
    /// `tree`/`utree` commands.
    pub trees: u32,
    /// `dimensions ntax=`.
    pub ntax: Option<u32>,
    /// `dimensions nchar=`.
    pub nchar: Option<u32>,
    /// `matrix` commands.
    pub matrices: u32,
    /// `translate` tables inside trees blocks.
    pub translate: u32,
    /// `taxlabels`/`charlabels`/`statelabels` commands.
    pub labels: u32,
}

fn first_word(line: &str) -> &str {
    line.split(|c: char| c.is_ascii_whitespace() || c == ';')
        .next()
        .unwrap_or("")
}

fn dim_val(line: &str, key: &str) -> Option<u32> {
    let low = line.to_ascii_lowercase();
    let i = low.find(key)?;
    let rest = &low[i + key.len()..];
    let rest = rest.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

/// `true` on a `#NEXUS` first token.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.trim_start()
        .get(..6)
        .is_some_and(|h| h.eq_ignore_ascii_case("#nexus"))
        && s.trim_start()[..]
            .chars()
            .nth(6)
            .map_or(true, |c| c.is_ascii_whitespace() || c == '\n')
}

/// Census; `None` without `#NEXUS`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nexus> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut n = Nexus {
        blocks: 0,
        block_names: 0,
        trees: 0,
        ntax: None,
        nchar: None,
        matrices: 0,
        translate: 0,
        labels: 0,
    };
    let mut names = std::collections::BTreeSet::new();
    let mut in_tree_block = false;
    // Commands may share a line: scan each statement split on ';'.
    for stmt in s.split(';') {
        for raw in stmt.split('\n') {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('[') {
                continue;
            }
            let word = first_word(line).to_ascii_lowercase();
            match word.as_str() {
                "begin" => {
                    n.blocks += 1;
                    let name = line
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .trim_end_matches(';')
                        .to_ascii_lowercase();
                    in_tree_block = name == "trees";
                    if !name.is_empty() {
                        names.insert(name);
                    }
                }
                "end" | "endblock" => in_tree_block = false,
                "dimensions" => {
                    if n.ntax.is_none() {
                        n.ntax = dim_val(line, "ntax");
                    }
                    if n.nchar.is_none() {
                        n.nchar = dim_val(line, "nchar");
                    }
                }
                "tree" | "utree" => n.trees += 1,
                "matrix" => n.matrices += 1,
                "translate" => {
                    if in_tree_block {
                        n.translate += 1;
                    }
                }
                "taxlabels" | "charlabels" | "statelabels" | "charstatelabels" => {
                    n.labels += 1;
                }
                _ => {}
            }
        }
    }
    n.block_names = names.len() as u32;
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"#NEXUS\n\
begin taxa;\n\
dimensions ntax=4;\n\
taxlabels Alpha Beta Gamma Delta;\n\
end;\n\
begin characters;\n\
dimensions nchar=8;\n\
format datatype=dna gap=-;\n\
matrix\n\
Alpha   ACGTACGT\n\
Beta    ACGTACGT\n\
Gamma   ACGTACGA\n\
Delta   TCGAACGT\n\
;\n\
end;\n\
begin trees;\n\
translate 1 Alpha, 2 Beta, 3 Gamma, 4 Delta;\n\
tree t1 = ((1,2),(3,4));\n\
tree t2 = ((1,3),(2,4));\n\
end;\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"#nexus\n"));
        assert!(!detect(b"# NEXUS spaced"));
        assert!(!detect(b"begin taxa;"));
    }

    #[test]
    fn parses() {
        let n = parse(D).unwrap();
        assert_eq!(n.blocks, 3);
        assert_eq!(n.block_names, 3);
        assert_eq!(n.trees, 2);
        assert_eq!(n.ntax, Some(4));
        assert_eq!(n.nchar, Some(8));
        assert_eq!(n.matrices, 1);
        assert_eq!(n.translate, 1);
        assert_eq!(n.labels, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
