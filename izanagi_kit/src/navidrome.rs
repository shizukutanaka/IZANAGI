//! Navidrome `navidrome.toml` census.
//!
//! Keys: `MusicFolder`, `DataFolder`, `CacheFolder`, `Address`,
//! `Port`, `BaseURL`, `UnixSocketPerm`, `LogLevel`, `LogFile`,
//! `ScanSchedule`, `ScanInterval`, `TranscodingCacheSize`,
//! `ImageCacheSize`, `EnableLogRedacting`, `AuthRequestLimit`,
//! `AuthWindowLength`, `PasswordEncryptionKey`, `SessionTimeout`,
//! `EnableInsightsCollector`, `DevLogSourceLine`, `DevLogLevels`,
//! `EnableTranscodingConfig`, `EnableDownloads`, `EnableSharing`,
//! `ShareURL`, `SharingDefaultDescription`, `SharingDefaultExpiration`,
//! `MaxUploads`, `NewAlbumsLimit`, `RecentlyAddedByTime`,
//! `RecentlyAddedByUser`, `IgnoredArticles`, `SearchFullString`,
//! `DefaultTheme`, `DefaultLanguage`, `DefaultDownsamplingFormat`,
//! `Jukebox.Enabled`, `Jukebox.Devices`, `Jukebox.Default`,
//! `AutoImportPlaylists`, `DefaultPlaylistPublic`, `Playlist.Path`,
//! `PlaylistsPath`, `AutoTranscode.*`, `Backup.*` (`Path`, `Schedule`,
//! `Count`, `RestorePath`), `Prometheus.*` (`Enabled`, `Path`,
//! `MetricsPath`), `Subsonic.*` (`AppendSubtitle`, `ArtistParticipations`,
//! `DefaultReportRealPath`, `LegacyClients`, `AppendCroppedCoverArt`),
//! `LastFM.*` (`Enabled`, `ApiKey`, `Secret`, `Language`),
//! `Spotify.*` (`ID`, `Secret`), `ListenBrainz.*` (`Enabled`,
//! `BaseURL`), `Agents`, `CoverArtPriority`, `ImagePreCacheSize`,
//! `EnableExternalServices`, `EnableFfmpeg`, `FFmpegPath`,
//! `MPVPath`, `MPVCmdTemplate`, `TranscodingConfig`,
//! `ArtistArtPriority`, `AlbumArtPriority`, `CoverJpegQuality`,
//! `Scanner.*` (`Extractor`, `GenreSeparators`, `GroupAlbumEditions`,
//! `ArtistOnline`, `Schedule`, `WatcherWait`), `EnableSorting`,
//! `PrefersMultiDiscAlbums`, `EnableStarRating`, `EnableMediaFileSharing`,
//! `EnableUserEditing`, `AutoEnableUserDeletion`, `DefaultUIVolume`,
//! `DevEnableProfiler`, `DevFastAccessCoverArt`, `DevInitialMainLoopTimeout`,
//! `DevActivityPanel`, `DevSidebarPlaylists`, `DevBufferSize`,
//! `DevEventStreamBatchSize`, `DevWebPlaylistsEnabled`,
//! `DevOptimizeUpdatePlaylist`, `DevThreads`, `DevLogLevelCategories`,
//! `DevDownsamplingDebug`, `DevOldBackend`, `DevNewScanner`,
//! `DevCacheWarmup`, `DevPlayLimit`, `DevSerializeUISessions`.
//!
//! ```rust
//! let k = b"MusicFolder = \"/music\"\nDataFolder = \"/data\"\nPort = 4533\nScanSchedule = \"@every 1h\"\n";
//! assert!(izanagi_kit::navidrome::detect(k));
//! ```

/// navidrome.toml census.
#[derive(Debug, Clone)]
pub struct Navidrome {
    /// `key = value` assignments.
    pub settings: usize,
    /// recognised navidrome keys present.
    pub keys: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "MusicFolder",
    "DataFolder",
    "CacheFolder",
    "ScanSchedule",
    "ScanInterval",
    "TranscodingCacheSize",
    "ImageCacheSize",
    "EnableLogRedacting",
    "AuthRequestLimit",
    "AuthWindowLength",
    "PasswordEncryptionKey",
    "SessionTimeout",
    "EnableInsightsCollector",
    "DevLogSourceLine",
    "DevLogLevels",
    "EnableTranscodingConfig",
    "EnableDownloads",
    "EnableSharing",
    "ShareURL",
    "SharingDefaultDescription",
    "SharingDefaultExpiration",
    "MaxUploads",
    "NewAlbumsLimit",
    "RecentlyAddedByTime",
    "RecentlyAddedByUser",
    "IgnoredArticles",
    "SearchFullString",
    "DefaultTheme",
    "DefaultLanguage",
    "DefaultDownsamplingFormat",
    "AutoImportPlaylists",
    "DefaultPlaylistPublic",
    "CoverArtPriority",
    "ImagePreCacheSize",
    "EnableExternalServices",
    "EnableFfmpeg",
    "FFmpegPath",
    "MPVPath",
    "MPVCmdTemplate",
    "TranscodingConfig",
    "ArtistArtPriority",
    "AlbumArtPriority",
    "CoverJpegQuality",
    "EnableSorting",
    "PrefersMultiDiscAlbums",
    "EnableStarRating",
    "EnableMediaFileSharing",
    "EnableUserEditing",
    "AutoEnableUserDeletion",
    "DefaultUIVolume",
    "DevEnableProfiler",
    "DevFastAccessCoverArt",
    "DevInitialMainLoopTimeout",
    "DevActivityPanel",
    "DevSidebarPlaylists",
    "DevBufferSize",
    "DevEventStreamBatchSize",
    "DevWebPlaylistsEnabled",
    "DevOptimizeUpdatePlaylist",
    "DevThreads",
    "DevLogLevelCategories",
    "DevDownsamplingDebug",
    "DevOldBackend",
    "DevNewScanner",
    "DevCacheWarmup",
    "DevPlayLimit",
    "DevSerializeUISessions",
    "UnixSocketPerm",
    "LogLevel",
    "LogFile",
    "Address",
    "Port",
    "BaseURL",
    "AutoTranscode",
    "Jukebox",
    "LastFM",
    "Spotify",
    "ListenBrainz",
    "Subsonic",
    "Scanner",
    "Prometheus",
    "Backup",
    "Playlist",
    "Agents",
    "GenreSeparators",
    "GroupAlbumEditions",
    "ArtistOnline",
    "WatcherWait",
];

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with(';') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a `navidrome.toml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `MusicFolder`/`DataFolder`/`TranscodingCacheSize`/
    // `PasswordEncryptionKey`/`ScanSchedule`/`CoverArtPriority` are
    // navidrome-exclusive camel-case keys.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if KEYS.contains(&k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Navidrome {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = assign_key(line) {
                c.settings += 1;
                if KEYS.contains(&k) {
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
        let b = b"MusicFolder = \"/music\"\nDataFolder = \"/data\"\nPort = 4533\nScanSchedule = \"@every 1h\"\n";
        assert!(detect(b));
        let c = Navidrome::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"port = 1\nhost = \"x\"\n"));
        assert!(!detect(
            b"# MusicFolder = \"/x\"\n# DataFolder = \"/y\"\n# Port = 1\nz = 1\n"
        ));
    }
}
