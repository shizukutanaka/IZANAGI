//! `modprobe.d/*.conf` および `modprobe.conf` 設定の認識と計数。
//!
//! モジュール設定は `alias <alias> <module>`、`options <module> <k=v>…`、
//! `blacklist <module>`、`install <module> <command…>`、
//! `remove <module> <command…>`、`softdep <module> pre: <mods> post: <mods>`
//! (modprobe.d は `softdep` を継承する `use`、`pre:`/`post:`/`both:` 節)、
//! `include <dir>`、`depmod`、`prune`/`forbid` 系ディレクティブで構成される。
//! コメントは `#`、行末継続は `\`。
//!
//! ```
//! let b = b"# modprobe.d sample\nalias eth0 e1000\nalias pci:v00008086d0000100B e1000\noptions e1000 debug=1 InterruptThrottleRate=1\noptions cfg80211 ieee80211_regdom=JP\nblacklist evbug\nblacklist pcspkr\ninstall scsi_hostadapter /sbin/modprobe sd_mod; /sbin/modprobe st\nremove snd-hda-intel { /usr/sbin/alsactl store 0; }\nsoftdep scsi_hostadapter pre: sd_mod post: st\n";
//! assert!(izanagi_kit::modprobeconf::detect(b));
//! let c = izanagi_kit::modprobeconf::parse(b).unwrap();
//! assert_eq!(c.directives, 9);
//! assert_eq!(c.aliases, 2);
//! assert_eq!(c.options, 2);
//! assert_eq!(c.blacklists, 2);
//! assert_eq!(c.option_kv, 3); // debug=1, InterruptThrottleRate=1, ieee80211_regdom=JP
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 有効ディレクティブ行の個数(非コメント・非空)。
    pub directives: usize,
    /// `alias` 行の個数。
    pub aliases: usize,
    /// `options` 行の個数。
    pub options: usize,
    /// `options` 行内の `k=v` ペア総数。
    pub option_kv: usize,
    /// `blacklist` 行の個数。
    pub blacklists: usize,
    /// `install`/`remove` 行の個数。
    pub install_remove: usize,
    /// `softdep`/`use`/`include`/`depmod`/`prune`/`forbid` 等その他行の個数。
    pub misc: usize,
    /// `\` 継続行の個数(行内が `\` 終端)。
    pub continuations: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// modprobe.d らしさを返す。alias/options/blacklist 等が複数あること。
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
        let head = s.split_whitespace().next().unwrap_or("");
        if matches!(
            head,
            "alias"
                | "options"
                | "blacklist"
                | "install"
                | "remove"
                | "softdep"
                | "use"
                | "include"
                | "depmod"
                | "prune"
                | "forbid"
                | "allow"
                | "deny"
        ) {
            hits += 1;
        }
    }
    hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        directives: 0,
        aliases: 0,
        options: 0,
        option_kv: 0,
        blacklists: 0,
        install_remove: 0,
        misc: 0,
        continuations: 0,
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
        c.directives += 1;
        if s.ends_with('\\') {
            c.continuations += 1;
        }
        let mut toks = s.split_whitespace();
        let head = toks.next().unwrap_or("");
        match head {
            "alias" => c.aliases += 1,
            "options" => {
                c.options += 1;
                for tok in toks {
                    if tok.contains('=') && !tok.starts_with('#') {
                        c.option_kv += 1;
                    }
                }
            }
            "blacklist" => c.blacklists += 1,
            "install" | "remove" => c.install_remove += 1,
            "softdep" | "use" | "include" | "depmod" | "prune" | "forbid" | "allow" | "deny" => {
                c.misc += 1;
            }
            _ => {}
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(b"alias a b\noptions m k=v\n"));
        assert!(detect(b"blacklist x\ninstall m /sbin/y\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo bar\nbaz qux\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b =
            b"alias a b\noptions m a=1 b=2\nblacklist x\ninstall m /y\nsoftdep m pre: a post: b\n";
        let c = parse(b).unwrap();
        assert_eq!(c.directives, 5);
        assert_eq!(c.aliases, 1);
        assert_eq!(c.options, 1);
        assert_eq!(c.option_kv, 2);
        assert_eq!(c.blacklists, 1);
        assert_eq!(c.install_remove, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"alias a b\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
