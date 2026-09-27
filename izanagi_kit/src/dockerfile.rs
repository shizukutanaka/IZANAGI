//! Dockerfile instruction parsing.
//!
//! Line-oriented: `#` comments, `\`-continuations, instructions
//! are `KEYWORD args` (case-insensitive). Exposes instruction
//! kinds plus parsed `FROM`, `EXPOSE`, `ENV`, `LABEL` fields.
//!
//! ```
//! use izanagi_kit::dockerfile;
//! let d = b"# demo\nFROM alpine:3.19\nEXPOSE 80 \\\n    443/tcp\nENV A=1 B=2\nCMD [\"sh\"]\n";
//! let f = dockerfile::parse(d).unwrap();
//! assert_eq!(f.stages, 1);
//! assert_eq!(f.instructions.len(), 4);
//! ```

use std::string::String;
use std::vec::Vec;

/// Dockerfile instruction kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// FROM.
    From,
    /// RUN.
    Run,
    /// CMD.
    Cmd,
    /// ENTRYPOINT.
    Entrypoint,
    /// COPY.
    Copy,
    /// ADD.
    Add,
    /// ENV.
    Env,
    /// ARG.
    Arg,
    /// LABEL.
    Label,
    /// EXPOSE.
    Expose,
    /// USER.
    User,
    /// WORKDIR.
    Workdir,
    /// VOLUME.
    Volume,
    /// HEALTHCHECK.
    Healthcheck,
    /// SHELL.
    Shell,
    /// STOPSIGNAL.
    Stopsignal,
    /// ONBUILD.
    Onbuild,
    /// Unrecognised instruction.
    Other,
}

/// One instruction (continuations already joined).
#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    /// Kind.
    pub kind: Kind,
    /// Keyword as written (uppercased).
    pub keyword: String,
    /// Raw arguments text.
    pub args: String,
    /// 1-based line where the instruction starts.
    pub line: usize,
}

/// A parsed Dockerfile.
#[derive(Clone, Debug, PartialEq)]
pub struct Dockerfile {
    /// All instructions.
    pub instructions: Vec<Instruction>,
    /// Number of `FROM` stages.
    pub stages: usize,
    /// `FROM` image references (`image:tag` or `image@digest`).
    pub images: Vec<String>,
    /// Exposed ports (`80`, `443/tcp`…).
    pub ports: Vec<String>,
    /// `ENV`/`ARG` keys.
    pub vars: Vec<String>,
}

fn kind_of(k: &str) -> Kind {
    match k {
        "FROM" => Kind::From,
        "RUN" => Kind::Run,
        "CMD" => Kind::Cmd,
        "ENTRYPOINT" => Kind::Entrypoint,
        "COPY" => Kind::Copy,
        "ADD" => Kind::Add,
        "ENV" => Kind::Env,
        "ARG" => Kind::Arg,
        "LABEL" => Kind::Label,
        "EXPOSE" => Kind::Expose,
        "USER" => Kind::User,
        "WORKDIR" => Kind::Workdir,
        "VOLUME" => Kind::Volume,
        "HEALTHCHECK" => Kind::Healthcheck,
        "SHELL" => Kind::Shell,
        "STOPSIGNAL" => Kind::Stopsignal,
        "ONBUILD" => Kind::Onbuild,
        _ => Kind::Other,
    }
}

/// Parses a Dockerfile: joins `\`-continuations, skips blank/comment
/// lines, splits `KEYWORD args`; requires ≥1 instruction and ≥1 FROM.
pub fn parse(d: &[u8]) -> Option<Dockerfile> {
    let text = std::str::from_utf8(d).ok()?;
    // join continuations (strip `\` + newline)
    let mut joined = String::new();
    let mut lines = Vec::new();
    let mut cur_start = 1usize;
    for (line_no, raw) in (1usize..).zip(text.split('\n')) {
        let line = raw.trim_end_matches('\r');
        if joined.is_empty() {
            cur_start = line_no;
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            joined.push_str(stripped);
            joined.push(' ');
        } else {
            joined.push_str(line);
            lines.push((cur_start, std::mem::take(&mut joined)));
        }
    }
    if !joined.is_empty() {
        lines.push((cur_start, joined));
    }
    let mut instructions = Vec::new();
    let mut images = Vec::new();
    let mut ports = Vec::new();
    let mut vars = Vec::new();
    for (line, text) in lines {
        let t = text.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let sp = t.find([' ', '\t'])?;
        let keyword = t[..sp].to_uppercase();
        let args = t[sp..].trim().to_string();
        if keyword.is_empty() {
            return None;
        }
        let kind = kind_of(&keyword);
        match kind {
            Kind::From => {
                let img = args.split(' ').next().unwrap_or("");
                images.push(img.to_string());
            }
            Kind::Expose => {
                for p in args.split(' ') {
                    let p = p.trim();
                    if !p.is_empty() {
                        ports.push(p.to_string());
                    }
                }
            }
            Kind::Env | Kind::Arg => {
                for kv in args.split(' ') {
                    if let Some((k, _)) = kv.split_once('=') {
                        if !k.is_empty() {
                            vars.push(k.to_string());
                        }
                    } else if !kv.is_empty() {
                        vars.push(kv.to_string());
                    }
                }
            }
            _ => {}
        }
        instructions.push(Instruction {
            kind,
            keyword,
            args,
            line,
        });
    }
    if instructions.is_empty() || images.is_empty() {
        return None;
    }
    Some(Dockerfile {
        stages: images.len(),
        instructions,
        images,
        ports,
        vars,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"# c\nFROM alpine\nRUN echo hi\n  # comment\nEXPOSE 80 443/tcp\nENV A=1 B=2\nCMD sh -c run\n";
        let f = parse(d).unwrap();
        assert_eq!(f.stages, 1);
        assert_eq!(f.images, vec!["alpine".to_string()]);
        assert_eq!(f.ports, vec!["80".to_string(), "443/tcp".to_string()]);
        assert_eq!(f.vars, vec!["A".to_string(), "B".to_string()]);
        assert_eq!(f.instructions.len(), 5);
    }

    #[test]
    fn continuation() {
        let d = b"FROM x\nRUN echo a \\\n    b\n";
        let f = parse(d).unwrap();
        assert_eq!(f.instructions[1].args, "echo a      b");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"# only comments\n").is_none());
        assert!(parse(b"RUN echo\n").is_none()); // no FROM
        assert!(parse(&[0xFF]).is_none());
    }
}
