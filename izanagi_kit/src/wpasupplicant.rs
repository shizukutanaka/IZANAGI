//! wpa_supplicant `wpa_supplicant.conf` の検出・カウント。
//!
//! グローバル `key=value` と `network={ ... }` ブロックから構成される。
//! ネットワークブロック内は `ssid`/`psk`/`key_mgmt`/`eap`/`identity` 等。
//!
//! ```
//! let cfg = b"ctrl_interface=/var/run/wpa_supplicant\n\
//!             update_config=1\n\
//!             country=JP\n\
//!             network={\n\
//!                 ssid=\"HomeNet\"\n\
//!                 psk=\"secret\"\n\
//!                 key_mgmt=WPA-PSK\n\
//!             }\n";
//! assert!(izanagi_kit::wpasupplicant::detect(cfg));
//! let c = izanagi_kit::wpasupplicant::parse(cfg).unwrap();
//! assert_eq!(c.networks, 1);
//! ```

/// グローバル領域で既知のキー。
const GLOBAL_KEYS: &[&str] = &[
    "ctrl_interface",
    "ctrl_interface_group",
    "update_config",
    "country",
    "ap_scan",
    "autoscan",
    "fast_reauth",
    "opensc_engine_path",
    "pkcs11_engine_path",
    "pkcs11_module_path",
    "pcsc_reader",
    "pcsc_pin",
    "driver_param",
    "dot11RSNAConfigPMKLifetime",
    "dot11RSNAConfigPMKReauthThreshold",
    "dot11RSNAConfigSATimeout",
    "uuid",
    "device_name",
    "manufacturer",
    "model_name",
    "model_number",
    "serial_number",
    "device_type",
    "os_version",
    "config_methods",
    "wps_cred_processing",
    "wps_vendor_ext_m1",
    "sec_device_type",
    "p2p_listen_reg_class",
    "p2p_listen_channel",
    "p2p_oper_reg_class",
    "p2p_oper_channel",
    "p2p_go_intent",
    "p2p_ssid_postfix",
    "persistent_reconnect",
    "p2p_intra_bss",
    "p2p_group_idle",
    "p2p_passphrase_len",
    "p2p_pref_chan",
    "p2p_no_go_freq",
    "p2p_add_cli_chan",
    "p2p_optimize_listen_chan",
    "p2p_go_ht40",
    "p2p_go_vht",
    "p2p_disabled",
    "p2p_no_group_iface",
    "p2p_ignore_shared_freq",
    "p2p_cli_probe",
    "p2p_go_freq_change_policy",
    "p2p_device_random_mac_addr",
    "p2p_device_persistent_mac_addr",
    "p2p_interface_random_mac_addr",
    "p2p_search_delay",
    "p2p_go_ila",
    "serial_number2",
    "hessid",
    "access_network_type",
    "pbc_in_m1",
    "autoscan_periodic",
    "filter_ssids",
    "filter_rssi",
    "max_num_sta",
    "disassoc_low_ack",
    "interworking",
    "gas_address3",
    "gas_rand_addr_lifetime",
    "gas_rand_mac_addr",
    "preassoc_mac_addr",
    "key_mgmt_offload",
    "passive_scan",
    "reassoc_same_bss_optim",
    "wps_priority",
    "cert_in_cb",
    "wpa_rsc_relaxation",
    "schedule_scan_interval",
    "sched_scan_interval",
    "sched_scan_start_delay",
    "tdls_external_control",
    "wowlan_triggers",
    "p2p_search_social",
    "scan_cur_freq",
    "scan_res_handler",
    "bgscan",
    "bssid_hint",
    "ignore_old_scan_res",
    "mac_addr",
    "rand_addr_lifetime",
    "preassoc_mac_addr_random",
    "key_mgmt_offload_limit",
    "passive_scan_random",
    "reassoc_same_bss_optim2",
    "ftm_initiator",
    "ftm_responder",
    "pmf",
    "mesh_max_inactivity",
    "dot11RSNAConfigSATimeout2",
    "mbssid",
    "coloc_intf_reporting",
    "mld_connect",
];

/// `network={}` ブロック内で既知のキー。
const NETWORK_KEYS: &[&str] = &[
    "ssid",
    "scan_ssid",
    "bssid",
    "bssid_hint",
    "bssid_blacklist",
    "bssid_whitelist",
    "psk",
    "mem_only_psk",
    "sae_password",
    "sae_password_id",
    "sae_pwe",
    "proto",
    "key_mgmt",
    "pairwise",
    "group",
    "group_mgmt",
    "auth_alg",
    "ieee80211w",
    "eapol_flags",
    "eap_workaround",
    "eap",
    "identity",
    "anonymous_identity",
    "password",
    "ca_cert",
    "ca_cert2",
    "ca_path",
    "client_cert",
    "client_cert2",
    "private_key",
    "private_key2",
    "private_key_passwd",
    "dh_file",
    "subject_match",
    "subject_match2",
    "altsubject_match",
    "altsubject_match2",
    "domain_suffix_match",
    "domain_suffix_match2",
    "domain_match",
    "domain_match2",
    "phase1",
    "phase2",
    "pcsc",
    "pin",
    "engine_id",
    "key_id",
    "cert_id",
    "ca_cert_id",
    "key2_id",
    "pin2",
    "engine2_id",
    "cert2_id",
    "ca_cert2_id",
    "engine",
    "engine2",
    "openssl_ciphers",
    "openssl_ecdh_curves",
    "erp",
    "priority",
    "sim_num",
    "realm",
    "preferred_lang",
    "country",
    "freq_list",
    "scan_freq",
    "bgscan",
    "ignore_broadcast_ssid",
    "disabled",
    "id_str",
    "mode",
    "frequency",
    "fixed_freq",
    "scan_freq",
    "ht40",
    "vht",
    "he",
    "max_oper_chwidth",
    "ht",
    "pbss",
    "mcast_rate",
    "ht_MCS1",
    "key_mgmt2",
    "go_p2p_dev_addr",
    "p2p_client_list",
    "psk_list_entry",
    "ocv",
    "beacon_prot",
    "transition_disable",
    "sae_pk",
    "sae_pk_only",
    "owe_group",
    "owe_only",
    "owe_ptk_workaround",
    "multi_ap_backhaul_sta",
    "ft_eap_pmksa_caching",
    "dpp_connector",
    "dpp_netaccesskey",
    "dpp_netaccesskey_expiry",
    "dpp_csign",
    "dpp_pp_key",
    "dpp_pfs",
    "dpp_pfs_fallback",
    "mesh_iface",
    "mesh_basic_rates",
    "mesh_fwding",
    "mesh_rssi_threshold",
    "mesh_gate_announcements",
    "mesh_hwmp_rootmode",
    "mesh_max_peer_links",
    "mesh_aoi",
    "mesh_rsn_pairwise",
    "mesh_ttl",
    "mesh_element_ttl",
    "auto_reconnect",
    "dot11MeshMaxRetries",
    "dot11MeshRetryTimeout",
    "dot11MeshConfirmTimeout",
    "dot11MeshHoldingTimeout",
    "ssid2",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key=value` エントリ総数(グローバル + ネットワーク内)。
    pub entries: usize,
    /// グローバル領域のエントリ数。
    pub global: usize,
    /// `network={}` ブロック数。
    pub networks: usize,
    /// ネットワーク内エントリ数。
    pub network_keys: usize,
    /// ネットワーク内 `ssid`/`bssid` 指定数。
    pub ssid: usize,
    /// ネットワーク内 `psk`/`sae_*`/`wep_key*` 資格数。
    pub credentials: usize,
    /// ネットワーク内 `eap`/`identity`/`ca_*`/`client_cert`/`phase*` 802.1X 数。
    pub eap: usize,
    /// `cred={}`/`p2p_*`/`homebrew` など別種ブロック数。
    pub other_blocks: usize,
}

/// `b` が `wpa_supplicant.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.networks >= 1 || c.global >= 3)
}

/// `b` を `wpa_supplicant.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        global: 0,
        networks: 0,
        network_keys: 0,
        ssid: 0,
        credentials: 0,
        eap: 0,
        other_blocks: 0,
    };
    let mut in_network = false;
    let mut in_other = false;
    let mut global_known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if in_network {
            if line.starts_with('}') {
                in_network = false;
                continue;
            }
            let Some((key, _)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            c.entries += 1;
            c.network_keys += 1;
            if NETWORK_KEYS.contains(&key) {
                if matches!(key, "ssid" | "ssid2" | "bssid" | "bssid_hint") {
                    c.ssid += 1;
                } else if matches!(
                    key,
                    "psk"
                        | "sae_password"
                        | "sae_password_id"
                        | "wep_key0"
                        | "wep_key1"
                        | "wep_key2"
                        | "wep_key3"
                        | "wep_tx_keyidx"
                        | "password"
                ) {
                    c.credentials += 1;
                } else if matches!(
                    key,
                    "eap"
                        | "identity"
                        | "anonymous_identity"
                        | "ca_cert"
                        | "ca_cert2"
                        | "ca_path"
                        | "client_cert"
                        | "client_cert2"
                        | "private_key"
                        | "private_key2"
                        | "private_key_passwd"
                        | "phase1"
                        | "phase2"
                        | "pcsc"
                        | "pin"
                        | "domain_suffix_match"
                        | "domain_match"
                        | "subject_match"
                        | "altsubject_match"
                ) {
                    c.eap += 1;
                }
            }
            continue;
        }
        if in_other {
            if line.starts_with('}') {
                in_other = false;
            }
            continue;
        }
        if line == "network={" || line.starts_with("network={") {
            c.networks += 1;
            in_network = true;
            continue;
        }
        if line.starts_with("cred={")
            || line.starts_with("hlr_auc_gw={")
            || line.starts_with("homebrew={")
        {
            c.other_blocks += 1;
            in_other = true;
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        c.entries += 1;
        c.global += 1;
        if GLOBAL_KEYS.contains(&key) || key.starts_with("p2p_") || key.starts_with("wps_") {
            global_known += 1;
        }
    }
    if c.networks >= 1 || global_known >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"ctrl_interface=/var/run/wpa_supplicant\n\
        ctrl_interface_group=wheel\n\
        update_config=1\n\
        country=JP\n\
        \n\
        network={\n\
            ssid=\"Home\"\n\
            psk=\"secret\"\n\
            key_mgmt=WPA-PSK\n\
            proto=RSN\n\
            pairwise=CCMP\n\
            priority=5\n\
        }\n\
        network={\n\
            ssid=\"Corp\"\n\
            key_mgmt=WPA-EAP\n\
            eap=PEAP\n\
            identity=\"user@corp\"\n\
            password=\"pass\"\n\
            ca_cert=\"/etc/cert/ca.pem\"\n\
            phase1=\"peapver=0\"\n\
            phase2=\"MSCHAPV2\"\n\
            priority=10\n\
        }\n";

    #[test]
    fn detects_wpasupplicant() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 19);
        assert_eq!(c.global, 4);
        assert_eq!(c.networks, 2);
        assert_eq!(c.network_keys, 15);
        assert_eq!(c.ssid, 2);
        assert_eq!(c.credentials, 2);
        assert_eq!(c.eap, 5);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"interface=wlan0\nssid=x\n"));
    }
}
