//! systemd-tmpfiles(`tmpfiles.d/*.conf`)の認識と計数。
//!
//! tmpfiles.d の行は `Type Path Mode UID GID Age Argument` の7欄形式:
//! `d /run/app 0755 user group 10d`(ディレクトリ作成)や
//! `L+ /var/run/link - - - - /target`(シンボリックリンク強制)のように
//! 型文字で始まる。型文字は `d`/`D`(dir)/`e`/`v`/`q`/`Q`(subvol)/
//! `f`/`F`(file)/`w`/`W`(write)/`L`/`L+`/`C`/`C+`(symlink/copy)/
//! `p`/`p+`(pipe)/`c`/`c+`/`b`/`b+`(dev)/`m`(fifo)/
//! `z`/`Z`/`t`/`T`/`h`/`H`/`a`/`A`/`a+`/`A+`(属性・ACL)/
//! `r`/`R`/`e`/`x`/`X`/`D`(除去・調整) で、接頭に `!`(boot only)/
//! `+`(mkdirs)/`-`(ignore)/`=`(older)/`~`(interpret name) を付けられる。
//! Age 欄は `10d`/`2h`/`30min` のような期限指定、`-` は未設定。
//!
//! ```
//! let b = b"# tmpfiles sample\nd /run/myapp 0755 myuser mygroup 10d\nd /var/lib/myapp 0750 myuser mygroup -\nL+ /etc/myapp.conf - - - - /usr/share/myapp/default.conf\nf /var/log/myapp.log 0640 myuser mygroup -\nx /run/myapp/tmp-*\nr /var/tmp/myapp\n";
//! assert!(izanagi_kit::tmpfilesd::detect(b));
//! let c = izanagi_kit::tmpfilesd::parse(b).unwrap();
//! assert_eq!(c.entries, 6);
//! assert_eq!(c.dirs, 2); // d
//! assert_eq!(c.removes, 2); // x, r
//! assert_eq!(c.links, 1); // L+
//! assert_eq!(c.files, 1); // f
//! assert!(c.with_mode >= 3 && c.with_age >= 1);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 型文字で始まるエントリ行の個数。
    pub entries: usize,
    /// `d`/`D`/`e`/`v`/`q`/`Q` ディレクトリ・サブボリューム系の個数。
    pub dirs: usize,
    /// `f`/`F`/`w`/`W`/`m`/`p`/`c`/`b` ファイル・特殊ファイル系の個数。
    pub files: usize,
    /// `L`/`L+`/`C`/`C+` シンボリックリンク・コピー系の個数。
    pub links: usize,
    /// `r`/`R`/`x`/`X`/`D` 除去系の個数。
    pub removes: usize,
    /// `z`/`Z`/`t`/`T`/`h`/`H`/`a`/`A`/`a+`/`A+`/`a-`/`A-` 属性・ACL 調整系の個数。
    pub adjusts: usize,
    /// `!`/`+`/`-`/`=`/`~` 修飾付きエントリの個数。
    pub modified: usize,
    /// 3欄目にモード(8進等)が指定されたエントリの個数。
    pub with_mode: usize,
    /// 6欄目に Age(`\d+[dhmsw]` 等)が指定されたエントリの個数。
    pub with_age: usize,
    /// 最終欄に引数が付いたエントリの個数。
    pub with_arg: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const TYPE_CHARS: &[char] = &[
    'd', 'D', 'e', 'v', 'q', 'Q', 'f', 'F', 'w', 'W', 'L', 'C', 'p', 'm', 'c', 'b', 'z', 'Z', 't',
    'T', 'h', 'H', 'a', 'A', 'r', 'R', 'x', 'X',
];

const MOD_CHARS: &[char] = &['!', '+', '-', '=', '~'];

/// tmpfiles.d らしさを返す。型文字+絶対パスの行が複数あること。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        let first = s.chars().next().unwrap_or(' ');
        let mut i = 0usize;
        let bs = s.as_bytes();
        while i < bs.len() && MOD_CHARS.contains(&(bs[i] as char)) {
            i += 1;
        }
        if i < bs.len() && TYPE_CHARS.contains(&(bs[i] as char)) && first != '#' {
            // `L+`/`f+`/`p+` 等の型接尾 `+` を読み飛ばす。
            let mut j = i + 1;
            while j < bs.len() && bs[j] == b'+' {
                j += 1;
            }
            let rest = s[j..].trim_start();
            if rest.starts_with('/') || rest.starts_with('-') || rest.starts_with('%') {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        dirs: 0,
        files: 0,
        links: 0,
        removes: 0,
        adjusts: 0,
        modified: 0,
        with_mode: 0,
        with_age: 0,
        with_arg: 0,
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
        let mut toks = s.split_whitespace();
        let Some(head) = toks.next() else {
            continue;
        };
        let bs = head.as_bytes();
        let mut i = 0usize;
        let mut modded = false;
        while i < bs.len() && MOD_CHARS.contains(&(bs[i] as char)) {
            i += 1;
            modded = true;
        }
        if i >= bs.len() || !TYPE_CHARS.contains(&(bs[i] as char)) {
            continue;
        }
        let ty = bs[i] as char;
        c.entries += 1;
        if modded {
            c.modified += 1;
        }
        match ty {
            'd' | 'D' | 'e' | 'v' | 'q' | 'Q' => c.dirs += 1,
            'f' | 'F' | 'w' | 'W' | 'm' | 'p' | 'c' | 'b' => c.files += 1,
            'L' | 'C' => c.links += 1,
            'r' | 'R' | 'x' | 'X' => c.removes += 1,
            'z' | 'Z' | 't' | 'T' | 'h' | 'H' | 'a' | 'A' => c.adjusts += 1,
            _ => {}
        }
        // split_whitespace の先頭は型トークン、以降が Path/Mode/UID/GID/Age/Argument。
        let f: std::vec::Vec<&str> = std::iter::once(head).chain(toks).collect();
        if f.len() >= 3 && f[2] != "-" {
            c.with_mode += 1;
        }
        if f.len() >= 6 && f[5] != "-" {
            c.with_age += 1;
        }
        if f.len() >= 7 {
            c.with_arg += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(b"d /run/x 0755 u g -\nL /a - - - - /b\n"));
        assert!(detect(b"f+ /tmp/x 0644 - - -\nw /a - - - - hi\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo bar baz\n1 2 3\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"d /run/x 0755 u g 10d\nL+ /a - - - - /b\nf /log 0640 u g -\nx /tmp/a-*\nr /var/x\nz /d 0700 u u -\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 6);
        assert_eq!(c.dirs, 1);
        assert_eq!(c.links, 1);
        assert_eq!(c.files, 1);
        assert_eq!(c.removes, 2);
        assert_eq!(c.adjusts, 1);
        assert_eq!(c.with_mode, 3);
        assert_eq!(c.with_age, 1);
        assert_eq!(c.with_arg, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"d /x 0755 u g -\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}
