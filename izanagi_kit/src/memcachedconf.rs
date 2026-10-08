//! memcached.conf(-o file 形式)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::memcachedconf::parse(b"-m 64\n-p 11211\n-u memcache\n-c 1024\n-o lru_crawler\n").unwrap();
//! assert_eq!(c.options, 5);
//! assert_eq!(c.memory, 1);
//! assert_eq!(c.net, 2);
//! ```

/// `-` フラグの既知オプション(接続系)。
const NET: &[char] = &['p', 'l', 'U', 'x', 'b', 'c', 's', 'a', 'B', 'n'];
/// メモリ・スラブ系。
const MEM: &[char] = &['m', 'M', 'I', 'f', 'F', 'y', 'G'];
/// 実行・ログ・ユーザ系。
const RUN: &[char] = &[
    'u', 'd', 'r', 'P', 'v', 'k', 'g', 'S', 't', 'w', 'R', 'W', 'j', 'K', 'D', 'C',
];
/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `-` オプション行の総数。
    pub options: usize,
    /// 接続・ソケット系オプション数。
    pub net: usize,
    /// メモリ・スラブ系オプション数。
    pub memory: usize,
    /// 実行・ログ系オプション数。
    pub runtime: usize,
    /// `-o` 拡張オプション数。
    pub extended: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が memcached.conf かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.options >= 3)
}

/// `b` を memcached.conf として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        net: 0,
        memory: 0,
        runtime: 0,
        extended: 0,
        misc: 0,
    };
    let mut any_line = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        any_line = true;
        // flag form is `-x`/`--long` glued to the flag name; `- item`
        // (markdown bullet) is not an option
        if t.starts_with('-') && !t[1..].starts_with(char::is_whitespace) {
            c.options += 1;
            let fch = t
                .strip_prefix('-')
                .unwrap_or(t)
                .chars()
                .next()
                .unwrap_or(' ');
            if NET.contains(&fch) {
                c.net += 1;
            } else if MEM.contains(&fch) {
                c.memory += 1;
            } else if RUN.contains(&fch) {
                c.runtime += 1;
            } else if fch == 'o' {
                c.extended += 1;
            } else {
                c.misc += 1;
            }
        } else {
            c.misc += 1;
        }
    }
    (any_line && c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memcached() {
        let cfg = b"-m 64\n-p 11211\n-u memcache\n-l 127.0.0.1\n-c 1024\n-t 4\n-o lru_crawler\n-o modern\n# comment\n-v\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.options, 9);
        assert_eq!(c.net, 3);
        assert_eq!(c.memory, 1);
        assert_eq!(c.runtime, 3);
        assert_eq!(c.extended, 2);
    }

    #[test]
    fn not_memcached() {
        assert!(parse(b"key = value\n").is_none());
    }
}
