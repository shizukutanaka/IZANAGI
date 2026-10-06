//! Parser for AWS CloudFormation templates (`.template` / `.yaml` / `.json`).
//!
//! Locates top-level sections, counts `Type: …::…` resource declarations,
//! intrinsic-function usages (`!Ref`, `Fn::…`), and comments.
//!
//! ```
//! let b = b"AWSTemplateFormatVersion: \"2010-09-09\"\nResources:\n  Bucket:\n    Type: AWS::S3::Bucket\nOutputs:\n  Name:\n    Value: !Ref Bucket\n";
//! assert!(izanagi_kit::cfn::detect(b));
//! let c = izanagi_kit::cfn::Cfn::parse(b).unwrap();
//! assert_eq!(c.resources, 1);
//! assert_eq!(c.outputs, 1);
//! assert!(c.intrinsics >= 1);
//! ```

/// Parsed CloudFormation template summary.
#[derive(Debug, Clone)]
pub struct Cfn {
    /// `AWSTemplateFormatVersion` value (empty when absent).
    pub version: String,
    /// `Description` key present.
    pub description: bool,
    /// Entries under `Parameters:`.
    pub parameters: usize,
    /// Entries under `Mappings:`.
    pub mappings: usize,
    /// Entries under `Conditions:`.
    pub conditions: usize,
    /// Entries under `Rules:`.
    pub rules: usize,
    /// `Type:` declarations containing `::` inside `Resources:`.
    pub resources: usize,
    /// Entries under `Outputs:`.
    pub outputs: usize,
    /// `Transform:` key lines (`AWS::Serverless-…`, `AWS::Include`).
    pub transforms: usize,
    /// Uses the Serverless Application Model (`AWS::Serverless`/`AWS::Include`).
    pub sam: bool,
    /// `Globals:` section present.
    pub globals: bool,
    /// `Metadata:` section present.
    pub metadata: bool,
    /// Intrinsic function usages (`Ref`, `Fn::…`, `!…` shorthand).
    pub intrinsics: usize,
    /// `#`, `//`, or `/*` comment lines.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim_start(), key))
}

/// Bodies of every `key:`-headed block (until a line with indent <= header).
fn blocks<'a>(t: &'a str, key: &str) -> Vec<Vec<&'a str>> {
    let mut out: Vec<Vec<&'a str>> = Vec::new();
    let mut cur: Option<(usize, Vec<&'a str>)> = None;
    for l in t.lines() {
        let tr = l.trim_start();
        let i = l.len() - tr.len();
        if let Some((d, v)) = cur.as_mut() {
            if !tr.is_empty() && i <= *d {
                out.push(core::mem::take(v));
                cur = None;
            } else {
                v.push(l);
                continue;
            }
        }
        if cur.is_none() && !tr.is_empty() && is_key(tr, key) {
            cur = Some((i, Vec::new()));
        }
    }
    if let Some((_, v)) = cur.take() {
        out.push(v);
    }
    out
}

fn section<'a>(t: &'a str, key: &str) -> Vec<&'a str> {
    blocks(t, key).into_iter().next().unwrap_or_default()
}

/// Non-blank lines at the shallowest indent inside each block.
fn child_items(t: &str, key: &str) -> usize {
    blocks(t, key)
        .iter()
        .map(|b| {
            let at = b
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            b.iter()
                .filter(|l| !l.trim().is_empty() && l.len() - l.trim_start().len() == at)
                .count()
        })
        .sum()
}

fn val_after<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for l in t.lines() {
        let tr = l.trim();
        let kp = tr.trim_start_matches(['"', '\'']);
        if let Some(rest) = kp.strip_prefix(key) {
            let rest = rest.trim_start_matches(['"', '\'']).trim_start();
            if let Some(v) = rest.strip_prefix(':') {
                let v = v
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .trim_end_matches(',');
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}
fn jkey(t: &str, key: &str) -> bool {
    // `"key"` が値位置ではなくキー位置(直後が `:`)にあるかを確認。
    let pat = format!("\"{key}\"");
    let mut rest = t;
    while let Some(i) = rest.find(&pat) {
        rest = &rest[i + pat.len()..];
        if rest.trim_start().starts_with(':') {
            return true;
        }
    }
    false
}

/// Returns `true` when `b` looks like a CloudFormation template.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    if has_key(t, "AWSTemplateFormatVersion") || jkey(t, "AWSTemplateFormatVersion") {
        return true;
    }
    let res = section(t, "Resources");
    !res.is_empty() && res.iter().any(|l| l.contains("AWS::"))
}

impl Cfn {
    /// Parses `b` as a CloudFormation template.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let res = section(t, "Resources");
        let resources = res
            .iter()
            .filter(|l| {
                let tr = l.trim_start();
                (tr.starts_with("Type:") || tr.starts_with("\"Type\"")) && l.contains("::")
            })
            .count();
        const INTRINSIC: &[&str] = &[
            "!Ref",
            "!GetAtt",
            "!Sub",
            "!Join",
            "!Select",
            "!FindInMap",
            "!ImportValue",
            "!If",
            "!Equals",
            "!And",
            "!Or",
            "!Not",
            "!Base64",
            "!Cidr",
            "!Condition",
            "!Split",
            "!Transform",
            "!GetAZs",
            "Fn::",
            "\"Ref\"",
        ];
        let intrinsics = INTRINSIC.iter().map(|k| t.matches(k).count()).sum();
        let comments = t
            .lines()
            .filter(|l| {
                let tr = l.trim_start();
                tr.starts_with('#') || tr.starts_with("//") || tr.starts_with("/*")
            })
            .count();
        let transforms = t
            .lines()
            .filter(|l| {
                let tr = l.trim_start();
                tr.starts_with("Transform:") || tr.starts_with("\"Transform\":")
            })
            .count();
        Some(Self {
            version: val_after(t, "AWSTemplateFormatVersion")
                .unwrap_or("")
                .to_string(),
            description: has_key(t, "Description"),
            parameters: child_items(t, "Parameters"),
            mappings: child_items(t, "Mappings"),
            conditions: child_items(t, "Conditions"),
            rules: child_items(t, "Rules"),
            resources,
            outputs: child_items(t, "Outputs"),
            transforms,
            sam: t.contains("AWS::Serverless") || t.contains("AWS::Include"),
            globals: has_key(t, "Globals"),
            metadata: has_key(t, "Metadata"),
            intrinsics,
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# AWSTemplateFormatVersion: 2010-09-09\n"));
        assert!(!detect(b"{\"note\": \"AWSTemplateFormatVersion\"}\n"));
    }

    const SRC: &[u8] = b"# demo
AWSTemplateFormatVersion: \"2010-09-09\"
Description: demo stack
Parameters:
  Env:
    Type: String
Conditions:
  IsProd:
    Fn::Equals: [!Ref Env, prod]
Resources:
  Bucket:
    Type: AWS::S3::Bucket
  Fn:
    Type: AWS::Serverless::Function
Outputs:
  Name:
    Value: !Ref Bucket
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"foo: bar"));
    }

    #[test]
    fn counts_sections() {
        let c = Cfn::parse(SRC).unwrap();
        assert_eq!(c.version, "2010-09-09");
        assert!(c.description);
        assert_eq!(c.parameters, 1);
        assert_eq!(c.conditions, 1);
        assert_eq!(c.resources, 2);
        assert_eq!(c.outputs, 1);
        assert!(c.sam);
        assert_eq!(c.intrinsics, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Cfn::parse(b"\x01\x02").is_none());
    }
}
