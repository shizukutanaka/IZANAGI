//! Ledger/hledger/beancount ジャーナル形式 検出モジュール。
//!
//! plaintext accounting ジャーナルは `YYYY-MM-DD`(or `YYYY/MM/DD`)
//! で始まる取引ヘッダと、インデントされた転記行
//! `    Account:Sub  $10.00`、および `account`/`commodity`/
//! `payee`/`include`/`P`/`D`/`Y`/`apply account`/`!include`/
//! `option`/`document`/`balance`/`pad`/`note` ディレクティブで
//! 構成される。
//!
//! ```
//! let b = b"2024-01-15 * \"Grocery Store\"\n  Expenses:Food    $52.30\n  Assets:Checking\n2024-01-16 * \"Paycheck\"\n  Assets:Checking   $1200.00\n  Income:Salary\n";
//! let c = izanagi_kit::ledgerjournal::parse(b);
//! assert!(izanagi_kit::ledgerjournal::detect(b));
//! assert_eq!(c.transactions, 2);
//! ```

const DIRECTIVES: &[&str] = &[
    "account",
    "apply",
    "balance",
    "bucket",
    "check",
    "close",
    "commodity",
    "custom",
    "D",
    "define",
    "document",
    "end",
    "event",
    "include",
    "note",
    "open",
    "option",
    "P",
    "pad",
    "payee",
    "plugin",
    "price",
    "push",
    "query",
    "tag",
    "test",
    "Y",
    "year",
];

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && (b[4] == b'-' || b[4] == b'/' || b[4] == b'.')
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
        && (b[7] == b'-' || b[7] == b'/' || b[7] == b'.')
        && b[8].is_ascii_digit()
        && b[9].is_ascii_digit()
}

fn is_posting(l: &str) -> bool {
    // 先頭が空白/タブで、金額(数字)か勘定科目(`:`区切り)を含む。
    if !(l.starts_with(' ') || l.starts_with('\t')) {
        return false;
    }
    let t = l.trim();
    t.len() > 1
        && (t.chars().any(|c| c.is_ascii_digit())
            || t.split_whitespace().next().is_some_and(|w| w.contains(':')))
}

fn is_directive(t: &str) -> bool {
    let w = t.split_whitespace().next().unwrap_or("");
    let w = w.trim_start_matches('!');
    DIRECTIVES.contains(&w) || DIRECTIVES.contains(&w.to_ascii_lowercase().as_str())
}

/// `b` が ledger ジャーナルに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut txns = 0usize;
    let mut posts = 0usize;
    let mut dirs = 0usize;
    for l in t.lines() {
        if l.trim().is_empty() || l.trim_start().starts_with(';') {
            continue;
        }
        if is_posting(l) {
            posts += 1;
        } else if is_date(l.trim_start()) {
            txns += 1;
        } else if is_directive(l.trim()) {
            dirs += 1;
        }
    }
    (txns >= 1 && posts >= 3) || (txns >= 2 && posts >= 2) || dirs >= 3 || (dirs >= 1 && txns >= 2)
}

/// ledger ジャーナルの統計。
#[derive(Debug, Default, Clone)]
pub struct LedgerJournal {
    /// 取引ヘッダ行数。
    pub transactions: usize,
    /// 転記行数。
    pub postings: usize,
    /// ディレクティブ行数。
    pub directives: usize,
    /// コメント行数(`;`/`#`/`%`/`*`始まり)。
    pub comments: usize,
}

/// `b` を ledger ジャーナルとして統計する。
pub fn parse(b: &[u8]) -> LedgerJournal {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = LedgerJournal::default();
    for l in t.lines() {
        let tr = l.trim_start();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with(';') || tr.starts_with('#') || tr.starts_with('%') {
            c.comments += 1;
            continue;
        }
        if is_posting(l) {
            c.postings += 1;
        } else if is_date(tr) {
            c.transactions += 1;
        } else if is_directive(l.trim()) {
            c.directives += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"2024-01-15 * \"Store\"\n  Expenses:Food  $10\n  Assets:Cash\n2024-01-16 * \"Bank\"\n  Assets:Cash  $20\n  Income:Misc\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.transactions, 2);
        assert_eq!(c.postings, 4);
    }

    #[test]
    fn detects_directives() {
        let b = b"account Expenses:Food\ncommodity $\npayee Store\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"2024-01-15\n"));
        assert!(!detect(b"key = value\nfoo = bar\nbaz = quux\n"));
        assert!(!detect(b"  indented 1\n  indented 2\n  indented 3\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.transactions, 0);
    }
}
