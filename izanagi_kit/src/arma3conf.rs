//! Arma 3 `server.cfg`/`basic.cfg` 検出モジュール。
//!
//! Arma サーバ設定は C-like な `key = value;` 形式で、`hostname`/
//! `password`/`passwordAdmin`/`serverCommandPassword`/`maxPlayers`/
//! `motd[]`/`admins[]`/`voteThreshold`/`voteMissionPlayers`/
//! `allowedVoteCmds[]`/`allowedVotedAdminCmds[]`/`kickDuplicate`/
//! `equalModRequired`/`verifySignatures`/`requiredSecureId`/
//! `allowedFilePatching`/`allowedLoadFileExtensions[]`/
//! `allowedPreprocessFileExtensions[]`/`allowedHTMLLoadExtensions[]`/
//! `allowedHTMLLoadURIs[]`/`disconnectTimeout`/`maxDesync`/`maxPing`/
//! `maxPacketLoss`/`kickClientsOnSlowNetwork[]`/`serverTime`/
//! `serverTimeAcceleration`/`missionWhitelist[]`/`persistent`/
//! `battlEye`/`timeStampFormat`/`logFile`/`forceRotorLibSimulation`/
//! `upnp`/`enablePlayerDiag`/`callExtension`/`headlessClients[]`/
//! `localClient[]`/`loopback`/`dedicatedServerId`/`doubleIdDetected`/
//! `onUserConnected`/`onUserDisconnected`/`onHackedData`/
//! `onDifferentData`/`onUnsignedData`/`regularCheck`/`steamProtocolMaxDataSize`
//! と、basic.cfg の帯域設定 `MinBandwidth`/`MaxBandwidth`/
//! `MaxSizeGuaranteed`/`MaxSizeNonguaranteed`/`MinErrorToSend`/
//! `MinErrorToSendNear`、および
//! `class Missions { class Mission_1 { template = ...;
//! difficulty = ...; }; };`/`class Params` ブロックで構成される。
//!
//! ```
//! let b = b"hostname = \"My Server\";\n\
//!           password = \"pw\";\n\
//!           maxPlayers = 64;\n\
//!           motd[] = {\"a\", \"b\"};\n\
//!           voteThreshold = 0.5;\n\
//!           persistent = 1;\n";
//! let c = izanagi_kit::arma3conf::parse(b);
//! assert!(izanagi_kit::arma3conf::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "admins",
    "allowedFilePatching",
    "allowedHTMLLoadExtensions",
    "allowedHTMLLoadURIs",
    "allowedLoadFileExtensions",
    "allowedPreprocessFileExtensions",
    "allowedVoteCmds",
    "allowedVotedAdminCmds",
    "battlEye",
    "callExtension",
    "dedicatedServerId",
    "difficulty",
    "disconnectTimeout",
    "doubleIdDetected",
    "drawInMap",
    "enablePlayerDiag",
    "equalModRequired",
    "forceRotorLibSimulation",
    "headlessClients",
    "hostname",
    "kickClientsOnSlowNetwork",
    "kickDuplicate",
    "localClient",
    "logFile",
    "loopback",
    "MaxBandwidth",
    "maxCustomFileSize",
    "maxDesync",
    "maxHTMLLoadURIsize",
    "maxPacketLoss",
    "maxPing",
    "maxPlayers",
    "MaxSizeGuaranteed",
    "MaxSizeNonguaranteed",
    "mercenary",
    "MinBandwidth",
    "MinErrorToSend",
    "MinErrorToSendNear",
    "missionWhitelist",
    "motd",
    "onDifferentData",
    "onHackedData",
    "onUnsignedData",
    "onUserConnected",
    "onUserDisconnected",
    "password",
    "passwordAdmin",
    "persistent",
    "recruit",
    "regular",
    "requiredSecureId",
    "serverCommandPassword",
    "serverTime",
    "serverTimeAcceleration",
    "steamProtocolMaxDataSize",
    "template",
    "timeStampFormat",
    "upnp",
    "verifySignatures",
    "veteran",
    "voteMissionPlayers",
    "voteThreshold",
];

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    let k = k.trim_end_matches("[]");
    KEYS.contains(&k) || k.starts_with("class Mission_")
}

fn is_class(t: &str) -> bool {
    t.starts_with("class Missions")
        || t.starts_with("class Params")
        || t.starts_with("class Mission_")
}

/// `b` が arma3 server.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut classes = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("//") || tr == "};" {
            continue;
        }
        if is_class(tr) {
            classes += 1;
        } else if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    keys >= 3 || (classes >= 1 && keys >= 1)
}

/// arma3 server.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct Arma3Conf {
    /// 既知キー行数。
    pub keys: usize,
    /// class ブロック数。
    pub classes: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を arma3 server.cfg として統計する。
pub fn parse(b: &[u8]) -> Arma3Conf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Arma3Conf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr == "};" {
            continue;
        }
        if tr.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if is_class(tr) {
            c.classes += 1;
        } else if tr.contains('=') && is_key(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"hostname = \"x\";\npassword = \"y\";\nmaxPlayers = 64;\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_missions() {
        let b =
            b"hostname = \"x\";\nclass Missions {\nclass Mission_1 {\ntemplate = altis;\n};\n};\n";
        assert!(detect(b));
    }

    #[test]
    fn detects_basic_cfg() {
        let b = b"MinBandwidth = 131072;\nMaxBandwidth = 524288;\nMaxSizeGuaranteed = 512;\nMaxSizeNonguaranteed = 256;\nMinErrorToSend = 0.001;\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 5);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"hostname = \"x\";\npassword = \"y\";\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
        assert!(!detect(b"int x = 1;\nfloat y = 2;\nchar z = 3;\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
