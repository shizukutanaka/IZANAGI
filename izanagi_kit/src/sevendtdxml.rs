//! 7 Days to Die `serverconfig.xml` 検出モジュール。
//!
//! `<ServerSettings>` ルート + `<property name="..." value="..."/>`
//! プロパティ列で構成される。主なプロパティ: `ServerName`/
//! `ServerDescription`/`ServerWebsiteURL`/`ServerPassword`/
//! `ServerPort`/`ServerVisibility`/`ServerDisabledNetworkProtocols`/
//! `ServerMaxWorldTransferSpeedKiBs`/`ServerMaxPlayerCount`/
//! `ServerReservedSlots`/`ServerAdminSlots`/`ServerAdminSlotsPermission`/
//! `GameWorld`/`GameName`/`GameDifficulty`/`GameMode`/
//! `DropOnDeath`/`DropOnQuit`/`BloodMoonFrequency`/`BloodMoonRange`/
//! `BloodMoonWarning`/`BloodMoonEnemyCount`/`DayNightLength`/
//! `DayLightLength`/`ZombieMove`/`ZombieFeralMove`/`ZombieBMMove`/
//! `LootAbundance`/`LootRespawnDays`/`AirDropFrequency`/`AirDropMarker`/
//! `PlayerKillingMode`/`LandClaimCount`/`LandClaimSize`/
//! `LandClaimDeadZone`/`LandClaimExpiryTime`/`LandClaimDecayMode`/
//! `LandClaimOnlineDurabilityModifier`/`LandClaimOfflineDurabilityModifier`/
//! `LandClaimOfflineDelay`/`BedrollDeadZoneSize`/`BedrollExpiryTime`/
//! `MaxSpawnedZombies`/`MaxSpawnedAnimals`/`MaxUncoveredMapChunksPerPlayer`/
//! `PersistentPlayerProfiles`/`EACEnabled`/`HideCommandExecutionLog`/
//! `AdminFileName`/`UserDataFolder`/`WebDashboardEnabled`/
//! `WebDashboardPort`/`WebDashboardUrl`/`EnableMapRendering`/
//! `TelnetEnabled`/`TelnetPort`/`TelnetPassword`/`TelnetFailedLoginLimit`/
//! `TelnetFailedLoginsBlocktime`/`TerminalWindowEnabled`/`XPMultiplier`/
//! `BlockDamagePlayer`/`BlockDamageAI`/`BlockDamageAIBM`/
//! `DisableFT`/`AllowCheatLog`/`ServerAllowCrossplay`/`SaveGameFolder`/
//! `TwitchServerPermission`/`TwitchBloodMoonAllowed`/`DiscordBotEnabled`
//! 等。
//!
//! ```
//! let b = b"<?xml version=\"1.0\"?>\n<ServerSettings>\n\
//!           <property name=\"ServerName\" value=\"My Server\"/>\n\
//!           <property name=\"ServerPort\" value=\"26900\"/>\n\
//!           <property name=\"GameWorld\" value=\"Navezgane\"/>\n\
//!           <property name=\"MaxSpawnedZombies\" value=\"64\"/>\n\
//!           </ServerSettings>\n";
//! let c = izanagi_kit::sevendtdxml::parse(b);
//! assert!(izanagi_kit::sevendtdxml::detect(b));
//! assert_eq!(c.props, 4);
//! ```

const PROPS: &[&str] = &[
    "AdminFileName",
    "AirDropFrequency",
    "AirDropMarker",
    "AllowCheatLog",
    "BedrollDeadZoneSize",
    "BedrollExpiryTime",
    "BlockDamageAI",
    "BlockDamageAIBM",
    "BlockDamagePlayer",
    "BloodMoonEnemyCount",
    "BloodMoonFrequency",
    "BloodMoonRange",
    "BloodMoonWarning",
    "DayLightLength",
    "DayNightLength",
    "DisableFT",
    "DiscordBotEnabled",
    "DropOnDeath",
    "DropOnQuit",
    "EACEnabled",
    "EnableMapRendering",
    "GameDifficulty",
    "GameMode",
    "GameName",
    "GameWorld",
    "HideCommandExecutionLog",
    "LandClaimCount",
    "LandClaimDeadZone",
    "LandClaimDecayMode",
    "LandClaimExpiryTime",
    "LandClaimOfflineDelay",
    "LandClaimOfflineDurabilityModifier",
    "LandClaimOnlineDurabilityModifier",
    "LandClaimSize",
    "LootAbundance",
    "LootRespawnDays",
    "MaxSpawnedAnimals",
    "MaxSpawnedZombies",
    "MaxUncoveredMapChunksPerPlayer",
    "PersistentPlayerProfiles",
    "PlayerKillingMode",
    "SaveGameFolder",
    "ServerAdminSlots",
    "ServerAdminSlotsPermission",
    "ServerAllowCrossplay",
    "ServerDescription",
    "ServerDisabledNetworkProtocols",
    "ServerMaxWorldTransferSpeedKiBs",
    "ServerMaxPlayerCount",
    "ServerName",
    "ServerPassword",
    "ServerPort",
    "ServerReservedSlots",
    "ServerReservedSlotsPermission",
    "ServerVisibility",
    "ServerWebsiteURL",
    "TelnetEnabled",
    "TelnetFailedLoginLimit",
    "TelnetFailedLoginsBlocktime",
    "TelnetPassword",
    "TelnetPort",
    "TerminalWindowEnabled",
    "TwitchBloodMoonAllowed",
    "TwitchServerPermission",
    "UserDataFolder",
    "WebDashboardEnabled",
    "WebDashboardPort",
    "WebDashboardUrl",
    "WorldGenSeed",
    "WorldGenSize",
    "XPMultiplier",
    "ZombieBMMove",
    "ZombieFeralMove",
    "ZombieMove",
];

fn prop_name(l: &str) -> Option<String> {
    let pos = l.find("name=\"")? + 6;
    let end = l[pos..].find('"')? + pos;
    Some(l[pos..end].to_string())
}

/// `b` が 7dtd serverconfig.xml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut props = 0usize;
    for l in t.lines() {
        if let Some(n) = prop_name(l) {
            if PROPS.contains(&n.as_str()) {
                props += 1;
            }
        }
    }
    props >= 2 || (props >= 1 && t.contains("<ServerSettings"))
}

/// serverconfig.xml の統計。
#[derive(Debug, Default, Clone)]
pub struct SevenDtdXml {
    /// 既知プロパティ行数。
    pub props: usize,
}

/// `b` を serverconfig.xml として統計する。
pub fn parse(b: &[u8]) -> SevenDtdXml {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SevenDtdXml::default();
    for l in t.lines() {
        if let Some(n) = prop_name(l) {
            if PROPS.contains(&n.as_str()) {
                c.props += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"<property name=\"ServerPort\" value=\"26900\"/>\n<property name=\"GameWorld\" value=\"N\"/>\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.props, 2);
    }

    #[test]
    fn detects_root() {
        let b =
            b"<ServerSettings>\n<property name=\"ServerPort\" value=\"1\"/>\n</ServerSettings>\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"<property name=\"ServerPort\" value=\"1\"/>\n"));
        assert!(!detect(
            b"<property name=\"foo\" value=\"1\"/>\n<property name=\"bar\" value=\"2\"/>\n"
        ));
        assert!(!detect(b"key=value\nfoo=bar\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.props, 0);
    }
}
