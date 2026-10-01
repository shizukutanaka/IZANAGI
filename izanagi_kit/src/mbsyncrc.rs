//! isync/mbsync `.mbsyncrc` の検出・カウント。
//!
//! `IMAPAccount name`/`IMAPStore name`/`MaildirStore name`/`Channel name`/
//! `Group name` のオブジェクトブロックと、続くインデント無しの設定行
//! (`Host`/`User`/`PassCmd`/`SSLType`/`Far`/`Near`/`Patterns`/`Sync`/`Expunge`
//! `Create`/`SyncState`/`MaxMessages`/`CopyArrivalDate` 等)。
//!
//! ```
//! let cfg = b"IMAPAccount work\nHost imap.example.com\nUser me@example.com\n\
//!             PassCmd \"pass show mail\"\nSSLType IMAPS\n\n\
//!             IMAPStore work-remote\nAccount work\n\n\
//!             Channel work\nFar :work-remote:\nPatterns *\nSync All\n";
//! assert!(izanagi_kit::mbsyncrc::detect(cfg));
//! let c = izanagi_kit::mbsyncrc::parse(cfg).unwrap();
//! assert_eq!(c.blocks, 3);
//! assert_eq!(c.entries, 8);
//! assert_eq!(c.channel_blocks, 1);
//! ```

/// ブロック開始語。
const BLOCK_HEADS: &[&str] = &[
    "IMAPAccount",
    "IMAPStore",
    "MaildirStore",
    "Channel",
    "Group",
];

/// 既知の設定キー。
const KNOWN_KEYS: &[&str] = &[
    "Host",
    "Port",
    "User",
    "UserCmd",
    "Pass",
    "PassCmd",
    "UseKeychain",
    "Auth",
    "AuthMechs",
    "SSLType",
    "SSLVersions",
    "SystemCertificates",
    "CertificateFile",
    "ClientCertificate",
    "ClientKey",
    "CipherString",
    "PipelineDepth",
    "DisableCRAM",
    "Timeout",
    "InfoMarker",
    "Path",
    "PathDelimiter",
    "Namespace",
    "Trash",
    "TrashNewMessages",
    "TrashRemoteNew",
    "SubFolders",
    "Inbox",
    "Flatten",
    "MapInbox",
    "Far",
    "Near",
    "Pattern",
    "Patterns",
    "Sync",
    "Expunge",
    "Create",
    "Remove",
    "SyncState",
    "MaxMessages",
    "MaxSize",
    "ExpireUnread",
    "CopyArrivalDate",
    "Filter",
    "Account",
    "InboxPath",
    "MaildirFlag",
    "Info",
    "Detail",
    "Debug",
    "FSync",
    "ScaleDoubleBuffered",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// オブジェクトブロック開始行数。
    pub blocks: usize,
    /// `Channel` ブロック数。
    pub channel_blocks: usize,
    /// `IMAPStore`/`MaildirStore`/`Group` ストア/グループブロック数。
    pub store_blocks: usize,
    /// `IMAPAccount` ブロック数。
    pub account_blocks: usize,
    /// ブロック内の `Key value` 行数。
    pub entries: usize,
    /// 既知キーの行数。
    pub known_entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// `b` が `.mbsyncrc` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.blocks >= 1 && c.known_entries >= 3
}

/// `b` を `.mbsyncrc` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        blocks: 0,
        channel_blocks: 0,
        store_blocks: 0,
        account_blocks: 0,
        entries: 0,
        known_entries: 0,
        comments: 0,
    };
    let mut in_block = false;
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let head = s.split([' ', '\t']).next().unwrap_or("");
        if BLOCK_HEADS.contains(&head) {
            c.blocks += 1;
            in_block = true;
            match head {
                "Channel" => c.channel_blocks += 1,
                "IMAPAccount" => c.account_blocks += 1,
                _ => c.store_blocks += 1,
            }
            continue;
        }
        if !in_block {
            continue;
        }
        // `SyncState *` や `Patterns *` 等、値を伴う行
        if head.bytes().all(|c| c.is_ascii_alphanumeric()) {
            c.entries += 1;
            if KNOWN_KEYS.contains(&head) {
                c.known_entries += 1;
            }
        }
    }
    if c.blocks == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mbsyncrc() {
        let cfg = b"# isync\nIMAPAccount fastmail\nHost imap.fastmail.com\n\
                    User me@fastmail.com\nPassCmd \"pass fastmail\"\nSSLType IMAPS\n\
                    CertificateFile /etc/ssl/certs/ca-certificates.crt\n\n\
                    MaildirStore local\nPath ~/mail/\nInbox ~/mail/INBOX\nSubFolders Verbatim\n\n\
                    Channel fastmail\nFar :fastmail-remote:\nNear :local:\nPatterns INBOX Sent\n\
                    Sync All\nCreate Near\nExpunge None\nSyncState *\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.blocks, 3);
        assert_eq!(c.account_blocks, 1);
        assert_eq!(c.store_blocks, 1);
        assert_eq!(c.channel_blocks, 1);
        assert_eq!(c.known_entries, 15);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_plain() {
        assert!(!detect(b"host=localhost\nport=993\n"));
    }
}
