//! Project Zomboid `servertest.ini` 検出モジュール。
//!
//! PZ サーバ設定は平坦な `key=value` 形式で、`DefaultPort=`/
//! `UDPPort=`/`ResetID=`/`Mods=`/`Map=`/`DoLuaChecksum=`/`Public=`/
//! `PublicName=`/`PublicDescription=`/`MaxPlayers=`/`PVP=`/
//! `PauseEmpty=`/`GlobalChat=`/`Open=`/`ServerWelcomeMessage=`/
//! `LogLocalChat=`/`AutoCreateUserInWhiteList=`/`DisplayUserName=`/
//! `ShowFirstAndLastName=`/`SpawnPoint=`/`SafetySystem=`/`ShowSafety=`/
//! `SafetyToggleTimer=`/`SafetyCooldownTimer=`/`SpawnItems=`/
//! `HoursForLootRespawn=`/`MaxItemsForLootRespawn=`/
//! `ConstructionPreventsLootRespawn=`/`DropOffWhiteListAfterDeath=`/
//! `NoFire=`/`AnnounceDeath=`/`MinutesPerPage=`/
//! `SaveWorldEveryMinutes=`/`PlayerSafehouse=`/`AdminSafehouse=`/
//! `SafehouseAllowTrepass=`/`SafehouseAllowFire=`/`SafehouseAllowLoot=`/
//! `SafehouseAllowRespawn=`/`SafehouseDaySurvivedToClaim=`/
//! `SafeHouseRemovalTime=`/`AllowDestruction=`/
//! `SledgehammerOnlyInSafehouse=`/`KickFastPlayers=`/`ServerPlayerID=`/
//! `RCONPort=`/`RCONPassword=`/`RCON=`/`Password=`/
//! `MaxAccountsPerUser=`/`SleepAllowed=`/`SleepNeeded=`/
//! `SteamPort1=`/`SteamPort2=`/`WorkshopItems=`/`SteamScoreboard=`/
//! `UPnP=`/`UPnPLeaseTime=`/`UPnPZeroLeaseTimeFallback=`/`UPnPForce=`/
//! `CoopServerLaunchTimeout=`/`CoopMasterPingTimeout=`/`Discord*=`/
//! `Voice*=`/`LoginQueue*=`/`PlayerRespawnWithSelf=`/
//! `PlayerRespawnWithOther=`/`FastForwardMultiplier=`/
//! `DisableSafehouseWhenPlayerConnected=`/`Faction*=`/
//! `AllowTradeUI=`/`WorldItemRemovalList=`/`HoursForWorldItemRemoval=`/
//! `ItemRemovalListBlacklistToggle=`/`DisableRadio*=`/
//! `MapRemotePlayerVisibility=`/`Backups*=`/`ClientActionLogs=`/
//! `PerkLog=`/`ItemNumbersLimitPerContainer=`/
//! `BloodSplatLifespanDays=`/`AllowNonAsciiUsername=`/
//! `BanKnockedDown=`/`PVPLogWhileInTrade=`/
//! `CarEngineAttractionModifier=`/`PlayerBumpPlayer=`/`Anticheat*`/
//! `Limit*` 等のキーで構成される。
//!
//! ```
//! let b = b"DefaultPort=16261\n\
//!           MaxPlayers=16\n\
//!           PVP=true\n\
//!           PauseEmpty=true\n\
//!           ServerWelcomeMessage=hi\n\
//!           Public=true\n";
//! let c = izanagi_kit::pzserver::parse(b);
//! assert!(izanagi_kit::pzserver::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "AdminSafehouse",
    "AllowDestruction",
    "AllowNonAsciiUsername",
    "AllowTradeUI",
    "AnnounceDeath",
    "AnticheatProtectionType1",
    "AnticheatProtectionType10",
    "AnticheatProtectionType11",
    "AnticheatProtectionType12",
    "AnticheatProtectionType13",
    "AnticheatProtectionType14",
    "AnticheatProtectionType15",
    "AnticheatProtectionType16",
    "AnticheatProtectionType17",
    "AnticheatProtectionType18",
    "AnticheatProtectionType19",
    "AnticheatProtectionType2",
    "AnticheatProtectionType20",
    "AnticheatProtectionType21",
    "AnticheatProtectionType22",
    "AnticheatProtectionType23",
    "AnticheatProtectionType24",
    "AnticheatProtectionType3",
    "AnticheatProtectionType4",
    "AnticheatProtectionType5",
    "AnticheatProtectionType6",
    "AnticheatProtectionType7",
    "AnticheatProtectionType8",
    "AnticheatProtectionType9",
    "AutoCreateUserInWhiteList",
    "BackupsCount",
    "BackupsOnStart",
    "BackupsOnVersionChange",
    "BackupsPeriod",
    "BanKnockedDown",
    "BloodSplatLifespanDays",
    "CarEngineAttractionModifier",
    "ClientActionLogs",
    "ClientCommandFilter",
    "ConstructionPreventsLootRespawn",
    "CoopMasterPingTimeout",
    "CoopServerLaunchTimeout",
    "DefaultPort",
    "DisableRadioAdmin",
    "DisableRadioGM",
    "DisableRadioInvisible",
    "DisableRadioOther",
    "DisableRadioOverhead",
    "DisableSafehouseWhenPlayerConnected",
    "DiscordChannel",
    "DiscordChannelID",
    "DiscordEnable",
    "DiscordToken",
    "DisplayUserName",
    "DoLuaChecksum",
    "DropOffWhiteListAfterDeath",
    "Faction",
    "FactionDaySurvivedToCreate",
    "FactionPlayersRequiredForTag",
    "FastForwardMultiplier",
    "GlobalChat",
    "HoursForLootRespawn",
    "HoursForWorldItemRemoval",
    "ItemNumbersLimitPerContainer",
    "ItemRemovalListBlacklistToggle",
    "KickFastPlayers",
    "KnockedDownAllowed",
    "LimitPhysicalExercise",
    "LoginQueueConnectTimeout",
    "LoginQueueEnabled",
    "LogLocalChat",
    "Map",
    "MapRemotePlayerVisibility",
    "MaxAccountsPerUser",
    "MaxItemsForLootRespawn",
    "MaxPlayers",
    "MinutesPerPage",
    "Mods",
    "MouseOverToSeeDisplayName",
    "NoFire",
    "Open",
    "PVP",
    "PVPLogWhileInTrade",
    "Password",
    "PauseEmpty",
    "PerkLog",
    "PingLimit",
    "PlayerBumpPlayer",
    "PlayerRespawnWithOther",
    "PlayerRespawnWithSelf",
    "PlayerSafehouse",
    "Public",
    "PublicDescription",
    "PublicName",
    "RCON",
    "RCONPassword",
    "RCONPort",
    "ResetID",
    "SafetyCooldownTimer",
    "SafetySystem",
    "SafetyToggleTimer",
    "SafehouseAllowFire",
    "SafehouseAllowLoot",
    "SafehouseAllowRespawn",
    "SafehouseAllowTrepass",
    "SafehouseDaySurvivedToClaim",
    "SafeHouseRemovalTime",
    "SaveWorldEveryMinutes",
    "ServerPlayerID",
    "ServerWelcomeMessage",
    "ShowFirstAndLastName",
    "ShowSafety",
    "SleepAllowed",
    "SleepNeeded",
    "SneakModeHideFromOtherPlayers",
    "SpawnItems",
    "SpawnPoint",
    "SpeedLimit",
    "SteamPort1",
    "SteamPort2",
    "SteamScoreboard",
    "UDPPort",
    "UPnP",
    "UPnPForce",
    "UPnPLeaseTime",
    "UPnPZeroLeaseTimeFallback",
    "Voice3D",
    "VoiceEnable",
    "VoiceMaxDistance",
    "VoiceMinDistance",
    "WorkshopItems",
    "WorkshopScoreboard",
    "WorkshopVisibilityOverride",
    "WorldItemRemovalList",
    "server_browser_announced_ip",
];

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
        || k.starts_with("AnticheatProtectionType")
        || k.starts_with("Backups")
        || k.starts_with("Discord")
        || k.starts_with("DisableRadio")
        || k.starts_with("LoginQueue")
        || k.starts_with("Voice")
        || k.starts_with("Faction")
        || k.starts_with("Safehouse")
        || k.starts_with("UPnP")
        || k.starts_with("Workshop")
        || k.starts_with("Steam")
}

/// `b` が PZ servertest.ini に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    keys >= 4
}

/// PZ servertest.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct PzServer {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を servertest.ini として統計する。
pub fn parse(b: &[u8]) -> PzServer {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = PzServer::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.contains('=') && is_key(tr) {
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
        let b = b"DefaultPort=16261\nMaxPlayers=16\nPVP=true\nPauseEmpty=true\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"DefaultPort=16261\nMaxPlayers=16\nPVP=true\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# DefaultPort=1\n# MaxPlayers=2\n# PVP=true\n# PauseEmpty=true\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 4);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
