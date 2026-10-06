//! Velero(`velero.io/`)バックアップ/リストアマニフェストの検出と
//! 構造カウント。
//!
//! `kind:` が `Backup`/`Restore`/`Schedule`/`BackupStorageLocation`/
//! `VolumeSnapshotLocation`/`DeleteBackupRequest`/`PodVolumeBackup`/
//! `PodVolumeRestore`/`ServerStatusRequest`/`DownloadRequest`/`DataUpload`/
//! `DataDownload`/`BackupRepository`。
//!
//! ```
//! let b = b"apiVersion: velero.io/v1\nkind: Backup\nmetadata:\n  name: daily\n  namespace: velero\nspec:\n  includedNamespaces: [\"*\"]\n  ttl: 720h\n  storageLocation: default\n";
//! assert!(izanagi_kit::velero::detect(b));
//! let c = izanagi_kit::velero::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "Backup");
//! ```

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `key` の値を `key: value` 行から取り出す。
fn yaml_val<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let l = line.trim_start_matches(['"', '\'']);
    let r = l
        .strip_prefix(key)?
        .trim_start_matches(['"', '\''])
        .trim_start();
    r.strip_prefix(':')
        .map(|v| v.trim().trim_matches('"').trim_matches('\''))
}

/// `kind:` の値が Velero リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Velero `kind:` 値。
const KINDS: &[&str] = &[
    "Backup",
    "BackupRepository",
    "BackupStorageLocation",
    "DataDownload",
    "DataUpload",
    "DeleteBackupRequest",
    "DownloadRequest",
    "PodVolumeBackup",
    "PodVolumeRestore",
    "Restore",
    "Schedule",
    "ServerStatusRequest",
    "VolumeSnapshotLocation",
];

/// `spec:` 配下の代表的な Velero キー。
const SPEC_KEYS: &[&str] = &[
    "backupName",
    "credential",
    "csiSnapshotTimeout",
    "defaultVolumesToFsBackup",
    "excludedNamespaces",
    "excludedResources",
    "hooks",
    "includedNamespaces",
    "includedResources",
    "itemOperationTimeout",
    "labelSelector",
    "orLabelSelectors",
    "orderedResources",
    "provider",
    "resourceModifier",
    "restorePVs",
    "schedule",
    "snapshotMoveData",
    "snapshotVolumes",
    "storageLocation",
    "template",
    "ttl",
    "uploaderConfig",
    "volumeSnapshotLocations",
];

/// Velero マニフェストの構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 検出された `kind:` 値(非検出時は空文字列)。
    pub kind: String,
    /// `spec:` 配下の既知キー行数。
    pub spec_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が Velero マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.starts_with("velero.io/")));
    api && kind_val(t).is_some()
}

/// `b` を Velero マニフェストとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        kind: kind_val(t).unwrap_or("").to_string(),
        spec_keys: 0,
        comments: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if SPEC_KEYS.iter().any(|k| is_key(tr, k)) {
            c.spec_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"apiVersion: velero.io/v1\nkind: Backup\nmetadata:\n  name: daily\n  namespace: velero\nspec:\n  includedNamespaces: [\"*\"]\n  ttl: 720h\n  storageLocation: default\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: velero.io/v1\nkind: Schedule\nmetadata:\n  name: s\nspec:\n  schedule: \"0 7 * * *\"\n"
        ));
        assert!(!detect(
            b"apiVersion: velero.io/v1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "Backup");
        assert!(c.spec_keys >= 3);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}
