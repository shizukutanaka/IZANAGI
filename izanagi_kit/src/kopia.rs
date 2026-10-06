//! Kopia repository `repository.config`/policy/snapshot JSON census.
//!
//! `repository.config` JSON keys: `storage` (`type`: `s3`/`gcs`/
//! `azure`/`b2`/`rclone`/`sftp`/`webdav`/`filesystem`/`dropbox`/`gdrive`),
//! `cache` (`cacheDirectory`/`maxCacheSize`/`maxMetadataCacheSize`/
//! `maxListCacheDuration`/`maxUsage`/`minUploadAge`/
//! `minContentDownloadSubjectAge`/`maxUsageOrMinUploadAge`),
//! `encryption` (`encryption`, `masterKey`, `hashedPassword`,
//! `keyDerivation`), `format` (`blockSize`, `chunker`,
//! `encryption`, `hash`, `hmacSecret`, `masterKey`,
//! `mutableParameters`, `splitter`, `uniqueID`, `version`,
//! `eccOverheadPercent`, `enablePasswordChange`, `indexVersion`,
//! `maxPackSize`, `minVersion`, `epochParameters`,
//! `internalLog`, `objectFormat`, `password`, `requiredFeatures`,
//! `retentionMode`, `retentionPeriod`, `blobRetention`,
//! `epochRefreshFrequency`, `fullCheckpointInterval`,
//! `indexCommit`, `indexRefreshFrequency`, `maintenancePeriod`,
//! `minPurgeAge`, `owner`, `shard`, `sharded`, `shardingEnabled`,
//! `sync`, `throttler`, `uiPrefernces`, `version`/`nextUniqueId`),
//! `hostname`, `username`, `clientOptions`, `description`,
//! `uuid`. Policy JSON adds `compression`, `keepLatest`,
//! `keepHourly`, `keepDaily`, `keepWeekly`, `keepMonthly`,
//! `keepAnnual`, `keepVersionsWithin`, `ignoreIdenticalSnapshots`,
//! `retention`, `scheduling`, `files`, `errorHandling`, `upload`,
//! `compressionCompressorName`, `maxFileSize`, `parallel`,
//! `oneFileSystem`, `onlySource`, `ignoreParentDirs`, `ignoreRules`,
//! `dotIgnoreFiles`, `ignoreCacheDirs`, `ignoreWellKnownCacheDirs`,
//! `scanOneFilesystemOnly`, `noParentDotFiles`, `noParentIgnoreFiles`,
//! `noParentDotIgnoreFiles`, `exclude`, `excludeDirs`, `excludeFiles`,
//! `excludeFromParent`, `action`, `command`, `arguments`,
//! `mode`, `timeoutSeconds`, `interval`, `cron`, `random`,
//! `automatic`, `manual`, `snapshotTime`, `snapshotTimeZone`,
//! `timeOfDay`, `splitter`, `osnapshot`, `pinned`, `effective`.
//!
//! ```rust
//! let k = b"{\n \"storage\": {\"type\": \"s3\"},\n \"blockSize\": 1000,\n \"masterKey\": \"x\",\n \"hashedPassword\": \"y\"\n}\n";
//! assert!(izanagi_kit::kopia::detect(k));
//! ```

/// kopia repository/policy census.
#[derive(Debug, Clone)]
pub struct Kopia {
    /// `"key": value` pairs.
    pub pairs: usize,
    /// recognised kopia keys present.
    pub keys: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "cacheDirectory",
    "maxCacheSize",
    "maxMetadataCacheSize",
    "maxListCacheDuration",
    "masterKey",
    "hashedPassword",
    "keyDerivation",
    "blockSize",
    "hmacSecret",
    "mutableParameters",
    "uniqueID",
    "eccOverheadPercent",
    "enablePasswordChange",
    "indexVersion",
    "maxPackSize",
    "epochParameters",
    "requiredFeatures",
    "retentionMode",
    "retentionPeriod",
    "blobRetention",
    "epochRefreshFrequency",
    "fullCheckpointInterval",
    "indexCommit",
    "indexRefreshFrequency",
    "maintenancePeriod",
    "minPurgeAge",
    "shardingEnabled",
    "uiPrefernces",
    "nextUniqueId",
    "keepLatest",
    "keepHourly",
    "keepDaily",
    "keepWeekly",
    "keepMonthly",
    "keepAnnual",
    "keepVersionsWithin",
    "ignoreIdenticalSnapshots",
    "compressionCompressorName",
    "onlySource",
    "ignoreParentDirs",
    "dotIgnoreFiles",
    "ignoreWellKnownCacheDirs",
    "scanOneFilesystemOnly",
    "noParentDotFiles",
    "noParentIgnoreFiles",
    "noParentDotIgnoreFiles",
    "snapshotTime",
    "snapshotTimeZone",
    "osnapshot",
    "splitter",
    "chunker",
    "clientOptions",
    "internalLog",
    "objectFormat",
    "minVersion",
];

const WEAK: &[&str] = &[
    "storage",
    "cache",
    "encryption",
    "format",
    "hash",
    "version",
    "hostname",
    "username",
    "description",
    "uuid",
    "retention",
    "scheduling",
    "files",
    "errorHandling",
    "upload",
    "compression",
    "exclude",
    "excludeDirs",
    "excludeFiles",
    "excludeFromParent",
    "maxFileSize",
    "parallel",
    "oneFileSystem",
    "ignoreRules",
    "ignoreCacheDirs",
    "action",
    "command",
    "arguments",
    "mode",
    "timeoutSeconds",
    "interval",
    "cron",
    "automatic",
    "manual",
    "timeOfDay",
    "pinned",
    "effective",
    "owner",
    "shard",
    "sharded",
    "sync",
    "throttler",
    "password",
    "type",
    "options",
];

fn jkey(line: &str) -> Option<&str> {
    let s = line.trim();
    if !s.starts_with('"') {
        return None;
    }
    let end = s[1..].find('"')? + 1;
    let after = s[end + 1..].trim_start();
    if after.starts_with(':') {
        Some(&s[1..end])
    } else {
        None
    }
}

/// Detect a Kopia `repository.config`/policy file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `masterKey`/`hashedPassword`/`mutableParameters`/`keepHourly`/
    // `cacheDirectory`/`epochParameters`/`blockSize` are kopia-only.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = jkey(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 3)
}

impl Kopia {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            pairs: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = jkey(line) {
                c.pairs += 1;
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"{\n \"storage\": {\"type\": \"s3\"},\n \"blockSize\": 1000,\n \"masterKey\": \"x\",\n \"hashedPassword\": \"y\"\n}\n";
        assert!(detect(b));
        let c = Kopia::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"{\n \"name\": \"x\",\n \"version\": 1\n}\n"));
        assert!(!detect(b"{\n \"cache\": {},\n \"format\": {}\n}\n"));
    }
}
