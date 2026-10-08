//! udev ルール(`*.rules`)の認識と計数。
//!
//! udev ルールはカンマ区切りの `KEY op "value"` ペアから成る行で、
//! マッチ側の演算子は `==`/`!=`、代入側は `=`/`+=`/`-=`/`:=`。
//! フィールドは `ACTION`/`SUBSYSTEM`/`KERNEL`/`DEVPATH`/`ATTR{…}`/`ATTRS{…}`/
//! `ENV{…}`/`RESULT`/`PROGRAM`/`RUN`/`GOTO`/`LABEL`/`NAME`/`SYMLINK`/`MODE`/
//! `OWNER`/`GROUP`/`OPTIONS`/`TAG`/`SYSCTL{…}`/`TEST`/`IMPORT{…}` 等。
//!
//! ```
//! let b = b"# 60-persistent.rules\nACTION==\"add\", SUBSYSTEM==\"usb\", ATTR{idVendor}==\"046d\", MODE=\"0666\"\nSUBSYSTEM==\"net\", ACTION==\"add\", RUN+=\"/etc/dhcp.sh\"\nENV{ID_FS_TYPE}==\"vfat\", SYMLINK+=\"usbdisk\"\nLABEL=\"end\"\nGOTO=\"skip\"\n";
//! assert!(izanagi_kit::udevrules::detect(b));
//! let c = izanagi_kit::udevrules::parse(b).unwrap();
//! assert_eq!(c.rules, 5);
//! assert_eq!(c.match_pairs, 6); // ACTION==add, SUBSYSTEM==usb, ATTR==046d, SUBSYSTEM==net, ACTION==add, ENV==vfat
//! assert_eq!(c.assign_pairs, 5); // MODE, RUN+=, SYMLINK+=, LABEL=, GOTO=
//! assert!(c.attr_refs > 0 && c.env_refs > 0);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `KEY op "value"` ペア総数。
    pub pairs: usize,
    /// `==`/`!=` マッチ演算子のペア数。
    pub match_pairs: usize,
    /// `=`/`+=`/`-=`/`:=` 代入演算子のペア数。
    pub assign_pairs: usize,
    /// `ATTR{…}`/`ATTRS{…}` 参照の個数。
    pub attr_refs: usize,
    /// `ENV{…}`/`SYSCTL{…}`/`TEST{…}`/`IMPORT{…}` 参照の個数。
    pub env_refs: usize,
    /// `RUN`/`PROGRAM`/`GOTO`/`LABEL` キーワード出現数。
    pub flow_keys: usize,
    /// 1行以上あるルール行(非コメント・非空)の個数。
    pub rules: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const FLOW: &[&str] = &["RUN", "PROGRAM", "GOTO", "LABEL", "IMPORT"];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `.rules` らしさを返す。`==`/`+=`/`=` ペアが存在し udev 系キーを含む。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        for key in [
            "ACTION",
            "SUBSYSTEM",
            "KERNEL",
            "DEVPATH",
            "ATTR",
            "ENV{",
            "RUN",
            "GOTO",
            "LABEL",
            "NAME",
            "SYMLINK",
            "MODE",
            "OWNER",
            "GROUP",
            "OPTIONS",
            "TAG",
            "RESULT",
            "PROGRAM",
            "TEST{",
            "IMPORT",
            "SYSCTL{",
        ] {
            if s.contains(key) && s.contains('"') {
                hits += 1;
                break;
            }
        }
    }
    hits >= 2
}

/// `KEY op "value"` の1ペアを解析して演算子種別を返す。
/// 戻り値: 0=match(==/!=), 1=assign(=/+=/-=/:=), -1=非ペア。
fn pair_op(seg: &str) -> i8 {
    // `ATTR{x}=="v"` → キー末尾 `{}` 可。まず演算子を検出。
    let ops = [":=", "+=", "-=", "==", "!=", "="];
    for op in ops {
        if let Some(pos) = seg.find(op) {
            // `=` は `==`/`!=`/`+=`/`-=`/`:=` の中に含まれ得るため、
            // 単独 `=` のときだけその位置で判定し直す。
            if op == "=" {
                let before = seg[..pos].chars().last().unwrap_or(' ');
                let after = seg[pos + 1..].chars().next().unwrap_or(' ');
                if before == '='
                    || after == '='
                    || before == '+'
                    || before == '-'
                    || before == ':'
                    || before == '!'
                {
                    continue;
                }
            }
            let key = seg[..pos].trim();
            let val = seg[pos + op.len()..].trim();
            if !key.is_empty() && val.starts_with('"') {
                return if op == "==" || op == "!=" { 0 } else { 1 };
            }
            break;
        }
    }
    -1
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        pairs: 0,
        match_pairs: 0,
        assign_pairs: 0,
        attr_refs: 0,
        env_refs: 0,
        flow_keys: 0,
        rules: 0,
        comments: 0,
    };
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        // カンマ分割。引用符内のカンマは考慮しない(udev は引用符内 `,` 稀)。
        // 行をルールとして数えるのは、少なくとも1つの `KEY op "value"` ペアを
        // 含む場合のみ — ペアを含まない行は「値なしの裸行」であり udev は
        // それをルールとして受理しない。
        let pairs_before = c.pairs;
        for seg in s.split(',') {
            let seg = seg.trim();
            if seg.is_empty() {
                continue;
            }
            let op = pair_op(seg);
            if op < 0 {
                continue;
            }
            c.pairs += 1;
            if op == 0 {
                c.match_pairs += 1;
            } else {
                c.assign_pairs += 1;
            }
            let key = seg
                .split([' ', '\t', '=', '+', '-', ':', '!'])
                .next()
                .unwrap_or("");
            if key.starts_with("ATTR{") || key.starts_with("ATTRS{") {
                c.attr_refs += 1;
            }
            if key.starts_with("ENV{")
                || key.starts_with("SYSCTL{")
                || key.starts_with("TEST{")
                || key.starts_with("IMPORT{")
            {
                c.env_refs += 1;
            }
            for f in FLOW {
                if key == *f || key.starts_with(&format!("{f}{{")) {
                    c.flow_keys += 1;
                }
            }
        }
        if c.pairs > pairs_before {
            c.rules += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(
            b"ACTION==\"add\", SUBSYSTEM==\"usb\"\nKERNEL==\"sda\", MODE=\"0666\"\n"
        ));
        assert!(detect(b"ACTION==\"a\", SUBSYSTEM==\"b\"\nRUN+=\"/x\"\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo = bar\nbaz = 1\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"ACTION==\"add\", SUBSYSTEM==\"usb\", ATTR{id}==\"x\", MODE=\"1\"\nENV{T}==\"v\", RUN+=\"/x\"\nLABEL=\"e\"\n";
        let c = parse(b).unwrap();
        assert_eq!(c.rules, 3);
        assert_eq!(c.match_pairs, 4); // ACTION/SUBSYSTEM/ATTR/ENV
        assert_eq!(c.assign_pairs, 3); // MODE/RUN+/LABEL
        assert_eq!(c.attr_refs, 1);
        assert_eq!(c.env_refs, 1);
        assert!(c.flow_keys >= 2); // RUN + LABEL
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"LABEL=\"x\"\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
