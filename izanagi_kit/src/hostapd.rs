//! hostapd `hostapd.conf` の検出・カウント。
//!
//! `key=value` フラット設定。インターフェース/SSID、無線 (802.11n/ac/ax)、
//! WPA セキュリティ、RADIUS、BSS 追加、WPS、インテアワーキング系を分類する。
//!
//! ```
//! let cfg = b"interface=wlan0\n\
//!             driver=nl80211\n\
//!             ssid=MyNet\n\
//!             hw_mode=g\n\
//!             channel=6\n\
//!             wpa=2\n\
//!             wpa_passphrase=secret123\n\
//!             wpa_key_mgmt=WPA-PSK\n";
//! assert!(izanagi_kit::hostapd::detect(cfg));
//! let c = izanagi_kit::hostapd::parse(cfg).unwrap();
//! assert_eq!(c.iface, 3);
//! assert_eq!(c.security, 3);
//! ```

/// インターフェース/SSID 系キー。
const IFACE_KEYS: &[&str] = &[
    "interface",
    "driver",
    "bridge",
    "ssid",
    "ssid2",
    "utf8_ssid",
    "bssid",
    "ctrl_interface",
    "ctrl_interface_group",
    "logger_syslog",
    "logger_syslog_level",
    "logger_stdout",
    "logger_stdout_level",
    "eapol_version",
    "daemonize",
    "pid_file",
];

/// 無線/チャネル系キー。
const RADIO_KEYS: &[&str] = &[
    "channel",
    "hw_mode",
    "op_class",
    "country_code",
    "country3",
    "ieee80211d",
    "ieee80211h",
    "ieee80211n",
    "ieee80211ac",
    "ieee80211ax",
    "ieee80211be",
    "ht_capab",
    "vht_capab",
    "vht_oper_chwidth",
    "vht_oper_centr_freq_seg0_idx",
    "vht_oper_centr_freq_seg1_idx",
    "he_capab",
    "he_op_chwidth",
    "he_bss_color",
    "he_mu_edca",
    "eht_oper_chwidth",
    "beacon_int",
    "dtim_period",
    "rts_threshold",
    "fragm_threshold",
    "preamble",
    "max_listen_interval",
    "max_num_sta",
    "tx_queue_data0",
    "tx_queue_data1",
    "tx_queue_data2",
    "tx_queue_data3",
    "wmm_enabled",
    "wmm_ac_be_aifsn",
    "wmm_ac_be_ecwmin",
    "wmm_ac_be_ecwmax",
    "wmm_ac_be_txop_limit",
    "airtime_mode",
    "airtime_update_interval",
    "chanlist",
    "acs_num_scans",
    "acs_chan_bias",
    "acs_exclude_dfs",
    "min_tx_power",
];

/// WPA/802.1X セキュリティ系キー。
const SECURITY_KEYS: &[&str] = &[
    "wpa",
    "wpa_passphrase",
    "wpa_psk",
    "wpa_psk_file",
    "wpa_key_mgmt",
    "wpa_pairwise",
    "rsn_pairwise",
    "rsn_override",
    "rsn_override_2",
    "auth_algs",
    "ieee8021x",
    "eapol_key_index_workaround",
    "eap_server",
    "eap_user_file",
    "wep_default_key",
    "wep_key0",
    "wep_key1",
    "wep_key2",
    "wep_key3",
    "wep_key_len_broadcast",
    "wep_key_len_unicast",
    "wep_rekey_period",
    "ap_isolate",
    "ap_max_inactivity",
    "skip_inactivity_poll",
    "disassoc_low_ack",
    "ignore_broadcast_ssid",
    "macaddr_acl",
    "accept_mac_file",
    "deny_mac_file",
    "sae_password",
    "sae_password_file",
    "sae_groups",
    "sae_require_mfp",
    "sae_pwe",
    "sae_anti_clogging_threshold",
    "sae_sync",
    "sae_reflection",
    "owe_groups",
    "wpa_group_rekey",
    "wpa_strict_rekey",
    "wpa_gmk_rekey",
    "wpa_ptk_rekey",
    "gtk_rsc_override",
    "igtk_rsc_override",
    "assoc_sa_query_max_timeout",
    "assoc_sa_query_retry_timeout",
    "ocv",
    "beacon_prot",
    "pasn_groups",
];

/// RADIUS/認証サーバ系キー。
const RADIUS_KEYS: &[&str] = &[
    "own_ip_addr",
    "nas_identifier",
    "auth_server_addr",
    "auth_server_port",
    "auth_server_shared_secret",
    "acct_server_addr",
    "acct_server_port",
    "acct_server_shared_secret",
    "radius_server_clients",
    "radius_server_ipv6",
    "radius_das_port",
    "radius_das_client",
    "radius_das_time_window",
    "radius_das_require_event_timestamp",
    "radius_das_require_message_authenticator",
    "radius_auth_req_attr_sqlite",
    "radius_acct_req_attr_sqlite",
    "radius_server_auth_port",
    "radius_server_acct_port",
    "pac_opaque_encr_key",
    "eap_fast_a_id",
    "eap_fast_a_id_info",
    "eap_fast_prov",
    "eap_sim_aka_result_ind",
    "tnc",
    "iapp_interface",
    "iapp_routing_table",
    "iapp_local_udp_port",
    "iapp_local_ipaddr",
    "iapp_peer",
];

/// BSS/VLAN 系キー。
const BSS_KEYS: &[&str] = &[
    "bss",
    "vlan",
    "vlan_file",
    "vlan_tagged_interface",
    "vlan_bridge",
    "vlan_naming",
    "vlan_no_bridge",
    "dynamic_vlan",
    "vlan_required",
    "per_sta_vif",
    "mcast_inet4_addr",
    "mcast_inet6_addr",
    "bss_load_update_period",
    "bss_load_test",
    "multicast_to_unicast",
    "proxy_arp",
    "na_mcast_to_ucast",
    "presp_to_proxy",
];

/// WPS/UPnP 系キー。
const WPS_KEYS: &[&str] = &[
    "wps_state",
    "wps_independent",
    "wps_pin_requests",
    "ap_setup_locked",
    "config_methods",
    "device_name",
    "device_type",
    "manufacturer",
    "model_name",
    "model_number",
    "serial_number",
    "manufacturer_url",
    "model_description",
    "model_url",
    "uuid",
    "os_version",
    "friendly_name",
    "upnp_iface",
    "wps_rf_bands",
    "pbc_in_m1",
    "wps_cred_processing",
    "ap_pin",
    "skip_cred_build",
    "extra_cred",
    "wps_cred_add_sae",
    "multi_ap",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key=value` エントリ総数。
    pub entries: usize,
    /// インターフェース/SSID/デーモン系キー数。
    pub iface: usize,
    /// 無線/チャネル系キー数。
    pub radio: usize,
    /// WPA/802.1X セキュリティ系キー数。
    pub security: usize,
    /// RADIUS 系キー数。
    pub radius: usize,
    /// BSS/VLAN 系キー数。
    pub bss: usize,
    /// WPS/UPnP 系キー数。
    pub wps: usize,
    /// `radius_*`/`eap_*`/`hs20*`/`anqp_*`/`interworking*` 等長接頭辞キー数。
    pub prefixed: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が `hostapd.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を `hostapd.conf` として解析し、キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        iface: 0,
        radio: 0,
        security: 0,
        radius: 0,
        bss: 0,
        wps: 0,
        prefixed: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
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
        if IFACE_KEYS.contains(&key) {
            c.iface += 1;
            known += 1;
        } else if RADIO_KEYS.contains(&key) {
            c.radio += 1;
            known += 1;
        } else if SECURITY_KEYS.contains(&key) {
            c.security += 1;
            known += 1;
        } else if RADIUS_KEYS.contains(&key) {
            c.radius += 1;
            known += 1;
        } else if BSS_KEYS.contains(&key) {
            c.bss += 1;
            known += 1;
        } else if WPS_KEYS.contains(&key) {
            c.wps += 1;
            known += 1;
        } else if key.starts_with("radius_")
            || key.starts_with("eap_")
            || key.starts_with("hs20")
            || key.starts_with("anqp_")
            || key.starts_with("interworking")
            || key.starts_with("mbo_")
            || key.starts_with("gas_")
            || key.starts_with("fils_")
            || key.starts_with("osen")
            || key.starts_with("tls_")
            || key.starts_with("ocsp_")
            || key.starts_with("crl_")
            || key.starts_with("dh_")
            || key.starts_with("openssl_")
        {
            c.prefixed += 1;
            known += 1;
        } else {
            c.misc += 1;
        }
    }
    if known >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# hostapd.conf\n\
        interface=wlan0\n\
        driver=nl80211\n\
        bridge=br0\n\
        ssid=MyNetwork\n\
        utf8_ssid=1\n\
        country_code=JP\n\
        hw_mode=g\n\
        channel=6\n\
        beacon_int=100\n\
        dtim_period=2\n\
        ieee80211n=1\n\
        wmm_enabled=1\n\
        wpa=2\n\
        wpa_passphrase=verysecret\n\
        wpa_key_mgmt=WPA-PSK\n\
        wpa_pairwise=CCMP\n\
        rsn_pairwise=CCMP\n\
        auth_algs=1\n\
        macaddr_acl=0\n\
        auth_server_addr=192.168.0.10\n\
        auth_server_port=1812\n\
        auth_server_shared_secret=radiussecret\n\
        acct_server_addr=192.168.0.10\n\
        bss=wlan0_0\n\
        ssid=GuestNet\n\
        wps_state=2\n\
        device_name=Wireless AP\n\
        hs20_operating_class=51\n";

    #[test]
    fn detects_hostapd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 28);
        assert_eq!(c.iface, 6);
        assert_eq!(c.radio, 7);
        assert_eq!(c.security, 7);
        assert_eq!(c.radius, 4);
        assert_eq!(c.bss, 1);
        assert_eq!(c.wps, 2);
        assert_eq!(c.prefixed, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"interface=wlan0\n"));
    }
}
