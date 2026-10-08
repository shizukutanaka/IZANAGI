//! `/etc/sysctl.conf` および `sysctl.d/*.conf` 設定の認識と計数。
//!
//! カーネル sysctl 設定は `dotted.key = value` の1行1代入形式。
//! キーは `kernel.*`/`vm.*`/`net.*`/`fs.*`/`dev.*`/`debug.*`/`abi.*`/`hw.*`/
//! `security.*`/`user.*`/`sunrpc` 等ドット区切り階層で表され、
//! `kernel.panic = 10` のように空白付き `=` を伴う。
//! `-`/`_`/`.` を含むキー名(例 `net.ipv4.tcp_congestion_control`)と
//! `*` ワイルドカード(`net.ipv4.conf.all.*`)がある。コメントは `#`/`;`。
//!
//! ```
//! let b = b"# sysctl.conf sample\nkernel.panic = 10\nkernel.hostname = router\nnet.ipv4.ip_forward = 1\nnet.ipv4.conf.all.rp_filter = 1\nnet.ipv4.tcp_congestion_control = bbr\nvm.swappiness = 10\nfs.file-max = 524288\nnet.bridge.bridge-nf-call-iptables = 0\n";
//! assert!(izanagi_kit::sysctlconf::detect(b));
//! let c = izanagi_kit::sysctlconf::parse(b).unwrap();
//! assert_eq!(c.assigns, 8);
//! assert_eq!(c.top_groups, 4); // kernel/net/vm/fs
//! assert_eq!(c.wildcards, 0);
//! assert!(c.numeric > 0 && c.comments > 0);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` 代入行の個数。
    pub assigns: usize,
    /// トップレベルグループ(`kernel`/`net`/`vm`/`fs`/`dev`/`debug`/`abi`/
    /// `hw`/`security`/`user`/`sunrpc` 等、第一段階目)の種類数。
    pub top_groups: usize,
    /// `*` ワイルドカードを含む代入の個数。
    pub wildcards: usize,
    /// 数値のみ値(`=` 右辺が数字と `-`/`+` 許容)の個数。
    pub numeric: usize,
    /// ブール風値(`0`/`1` のみ)の個数(numeric に含まれる)。
    pub bool01: usize,
    /// `#`/`;` コメント行の個数。
    pub comments: usize,
}

/// sysctl.conf らしさを返す。ドット区切りキー `=` 値が複数あること。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut dotted = 0usize;
    let mut sysctl_root = 0usize;
    for l in t.lines() {
        let s = l.trim();
        let Some(eq) = s.find('=') else {
            continue;
        };
        let k = s[..eq].trim_end();
        if k.len() > 3
            && k.contains('.')
            && k.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '*' | '[' | ']')
            })
        {
            dotted += 1;
            // `a.b.c=v` は Java properties 等他形式でも現れる — 先頭要素が
            // sysctl の既知サブツリーであることを要求する。
            let root = k.split('.').next().unwrap_or("");
            if matches!(
                root,
                "kernel" | "vm" | "net" | "fs" | "dev" | "debug" | "abi" | "user" | "sunrpc"
            ) {
                sysctl_root += 1;
            }
        }
    }
    dotted >= 2 && sysctl_root >= 1
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        assigns: 0,
        top_groups: 0,
        wildcards: 0,
        numeric: 0,
        bool01: 0,
        comments: 0,
    };
    let mut groups: std::vec::Vec<&str> = std::vec::Vec::new();
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        let Some(eq) = s.find('=') else {
            continue;
        };
        let k = s[..eq].trim_end();
        if k.is_empty()
            || !k.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '*' | '[' | ']')
            })
        {
            continue;
        }
        c.assigns += 1;
        if k.contains('*') {
            c.wildcards += 1;
        }
        let v = s[eq + 1..].trim();
        if !v.is_empty()
            && v.chars()
                .all(|x| x.is_ascii_digit() || x == '-' || x == '+')
        {
            c.numeric += 1;
            if v == "0" || v == "1" {
                c.bool01 += 1;
            }
        }
        let top = k.split('.').next().unwrap_or(k);
        if !groups.contains(&top) {
            groups.push(top);
            c.top_groups += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(b"kernel.panic = 10\nnet.ipv4.ip_forward = 1\n"));
        assert!(detect(b"fs.file-max=1\nvm.swappiness=10\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"FOO = bar\nBAZ = 1\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"# c\nkernel.panic = 10\nkernel.hostname = r\nnet.ipv4.conf.all.* = x\nvm.swappiness = 1\n";
        let c = parse(b).unwrap();
        assert_eq!(c.assigns, 4);
        assert_eq!(c.top_groups, 3);
        assert_eq!(c.wildcards, 1);
        assert_eq!(c.numeric, 2);
        assert_eq!(c.bool01, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"kernel.panic = 1\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}
