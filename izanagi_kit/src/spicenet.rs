//! SPICE 系ネットリスト (`.cir`/`.sp`/`.net`) の検出・カウント。
//!
//! 先頭行タイトル、`*` コメント、`.` ディレクティブ、
//! `R`/`C`/`L`/`K` 受動素子、`V`/`I`/`E`/`F`/`G`/`H`/`B` 電源、
//! `D`/`Q`/`M`/`J`/`Z` 半導体、`X`/`S`/`T`/`U`/`W` サブ回路・伝送線、
//! `+` 継続行を分類する。
//!
//! ```
//! let cfg = b"Test circuit\nV1 in 0 DC 5\nR1 in out 10k\nC1 out 0 1u\n.end\n";
//! assert!(izanagi_kit::spicenet::detect(cfg));
//! let c = izanagi_kit::spicenet::parse(cfg).unwrap();
//! assert_eq!(c.elements, 3);
//! ```

/// ディレクティブ既知名。
const DIRECTIVES: &[&str] = &[
    "subckt",
    "ends",
    "model",
    "param",
    "control",
    "endc",
    "tran",
    "ac",
    "dc",
    "op",
    "end",
    "include",
    "inc",
    "lib",
    "global",
    "probe",
    "print",
    "plot",
    "option",
    "options",
    "nodeset",
    "ic",
    "save",
    "four",
    "meas",
    "measure",
    "func",
    "title",
    "temp",
    "step",
    "data",
    "vec",
    "alias",
    "connect",
    "csparam",
    "noise",
    "pz",
    "sens",
    "tf",
    "disto",
    "st",
    "alter",
    "del",
    "dlib",
    "protect",
    "unprotect",
    "width",
    "dcmatch",
    "net",
    "mach",
    "echo",
    "new",
    "goto",
    "continue",
    "checkall",
    "conv",
    "chargert",
    "backanno",
    "savebias",
    "loadbias",
    "psf",
    "setplot",
    "mc",
    "external",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 有効行総数 (空白・コメント除く)。
    pub entries: usize,
    /// 素子行総数 (`R`/`C`/`L`/`K`/`V`/`I`/`E`/`F`/`G`/`H`/`B`/`D`/`Q`/`M`/`J`/`Z`/`X`/`S`/`T`/`U`/`W`/`A`/`O`/`P`/`N`)。
    pub elements: usize,
    /// `.xxx` ディレクティブ行数。
    pub directives: usize,
    /// `.subckt`/`.ends` サブ回路定義数。
    pub subckts: usize,
    /// `.model`/`.param`/`.func` モデル・パラメータ数。
    pub models: usize,
    /// `*` コメント行数。
    pub comments: usize,
    /// `+` 継続行数。
    pub continuations: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が SPICE ネットリストかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.elements >= 1 && c.directives >= 1)
}

/// `b` を SPICE ネットリストとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        elements: 0,
        directives: 0,
        subckts: 0,
        models: 0,
        comments: 0,
        continuations: 0,
        misc: 0,
    };
    let mut title_seen = false;
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        let head = line.trim_start();
        if head.starts_with('*') {
            c.comments += 1;
            continue;
        }
        if head.starts_with('+') {
            c.entries += 1;
            c.continuations += 1;
            continue;
        }
        if !title_seen {
            title_seen = true;
            if !head.starts_with('.') {
                // 先頭の非コメント非ディレクティブ行はタイトル行。
                continue;
            }
        }
        if let Some(rest) = head.strip_prefix('.') {
            let name = rest
                .split(|ch: char| ch.is_ascii_whitespace())
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            c.entries += 1;
            c.directives += 1;
            if name == "subckt" || name == "ends" {
                c.subckts += 1;
            } else if matches!(name.as_str(), "model" | "param" | "func") {
                c.models += 1;
            }
            if DIRECTIVES.contains(&name.as_str()) {
                known += 1;
            }
            continue;
        }
        let Some(first) = head.chars().next() else {
            continue;
        };
        if first.is_ascii_alphabetic() {
            let code = first.to_ascii_uppercase();
            if matches!(
                code,
                'R' | 'C'
                    | 'L'
                    | 'K'
                    | 'V'
                    | 'I'
                    | 'E'
                    | 'F'
                    | 'G'
                    | 'H'
                    | 'B'
                    | 'D'
                    | 'Q'
                    | 'M'
                    | 'J'
                    | 'Z'
                    | 'X'
                    | 'S'
                    | 'T'
                    | 'U'
                    | 'W'
                    | 'A'
                    | 'O'
                    | 'P'
                    | 'N'
            ) {
                c.entries += 1;
                c.elements += 1;
                known += 1;
            } else {
                c.misc += 1;
            }
        } else {
            c.misc += 1;
        }
    }
    if known >= 3 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"RC lowpass filter\n\
        * comment line\n\
        .title RC demo\n\
        V1 in 0 DC 5\n\
        R1 in out 10k\n\
        C1 out 0 1u\n\
        L1 out mid 1m\n\
        D1 mid 0 DMOD\n\
        Q1 c b e QMOD\n\
        M1 d g s s MMOD\n\
        X1 a b amp\n\
        .model DMOD D (is=1e-14)\n\
        .param rvar=1k\n\
        .subckt amp a b\n\
        Rin a b 1Meg\n\
        .ends amp\n\
        .tran 1u 1m\n\
        .option reltol=1e-3\n\
        .end\n";

    #[test]
    fn detects_spice() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.elements, 9);
        assert_eq!(c.directives, 8);
        assert_eq!(c.subckts, 2);
        assert_eq!(c.models, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_plain_text() {
        assert!(!detect(b"hello\nworld\nfoo bar\n"));
        assert!(!detect(b"R1 a b 1k\n"));
    }
}
