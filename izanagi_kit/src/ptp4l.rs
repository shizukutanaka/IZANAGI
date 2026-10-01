//! PTP `ptp4l.cfg` (linuxptp) の検出・カウント。
//!
//! INI: `[global]` + インターフェースセクション (`[eth0]` 等) + `[unicast_master_table]`。
//! IEEE 1588 属性 (`domainNumber`/`priority1`/`clockClass`/`logSyncInterval`/`delay_mechanism`/
//! `time_stamping`/`network_transport`/`twoStepFlag`) を分類。
//!
//! ```
//! let cfg = b"[global]\n\
//!             domainNumber\t0\n\
//!             time_stamping\thardware\n\
//!             twoStepFlag\t1\n\
//!             delay_mechanism\tE2E\n\
//!             network_transport\tUDPv4\n\
//!             [eth0]\n";
//! assert!(izanagi_kit::ptp4l::detect(cfg));
//! let c = izanagi_kit::ptp4l::parse(cfg).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// 時計属性系キー。
const CLOCK_KEYS: &[&str] = &[
    "domainNumber",
    "priority1",
    "priority2",
    "clockClass",
    "clockAccuracy",
    "offsetScaledLogVariance",
    "clockIdentity",
    "clock_type",
    "twoStepFlag",
    "slaveOnly",
    "masterOnly",
    "gmCapable",
    "G.8275.defaultDS.localPriority",
    "G.8275.portDS.localPriority",
    "localPriority",
    "stepsRemoved",
    "timeSource",
    "grandmasterIdentity",
    "free_running",
    "freq_est_interval",
    "initial_delay",
    "sanity_freq_limit",
    "step_threshold",
    "first_step_threshold",
    "max_frequency",
    "max_offset",
    "maxphase",
    "assume_two_step",
    "tx_timestamp_timeout",
    "check_fup_sync",
    "clock_servo",
    "pi_proportional_const",
    "pi_integral_const",
    "pi_proportional_scale",
    "pi_proportional_exponent",
    "pi_proportional_norm_max",
    "pi_integral_scale",
    "pi_integral_exponent",
    "pi_integral_norm_max",
    "pi_proportional_const_f",
    "pi_integral_const_f",
    "pi_offset_const",
    "pi_f_offset_const",
    "clock_servo_state",
    "ntpshm_segment",
    "ntpshm_num",
    "kernel_leap",
    "leapfile",
    "utc_offset",
    "clockLeapSecondFile",
    "timeSource2",
    "transportSpecific",
    "dataset_comparison",
    "description",
    "productInfo",
    "revisionData",
    "userDescription",
    "manufacturerIdentity",
    "inhibit_msrr",
    "asCapable",
    "asCapable2",
    "BMCA",
    "boundary_clock_jbod",
    "hybrid_e2e",
    "inhibit_multicast_service",
    "inhibit_announce",
    "ignore_source_id",
    "ignore_transport_specific",
    "serverOnly",
    "embeste",
    "gmTimeBaseIndicator",
    "lastGmPhaseChange",
    "timeTraceable",
    "frequencyTraceable",
    "ptpTimescale",
    "follow_up_info",
    "path_trace_enabled",
    "ai_proportional_const",
];

/// ポート/ネットワーク系キー。
const PORT_KEYS: &[&str] = &[
    "time_stamping",
    "twoStepFlag2",
    "delay_mechanism",
    "network_transport",
    "ptp_dst_mac",
    "p2p_dst_mac",
    "udp_ttl",
    "udp6_scope",
    "uds_address",
    "uds_ro_address",
    "socket_priority",
    "logging_level",
    "verbose",
    "use_syslog",
    "summary_interval",
    "logAnnounceInterval",
    "logSyncInterval",
    "logMinDelayReqInterval",
    "logMinPdelayReqInterval",
    "logQueryInterval",
    "announceReceiptTimeout",
    "syncReceiptTimeout",
    "delayAsymmetry",
    "fault_reset_interval",
    "fault_badpeernet_interval",
    "unicast_listen",
    "unicast_req_duration",
    "unicast_master_table",
    "unicast_domains",
    "table_id",
    "operLogPdelayReqInterval",
    "operLogSyncInterval",
    "operLogAnnounceInterval",
    "unicast_client",
    "interface_mode",
    "ingressLatency",
    "egressLatency",
    "neighborPropDelayThresh",
    "min_neighbor_prop_delay",
    "min_ppdelay_req_interval",
    "allowedLostResponses",
    "delay_filter",
    "delay_filter_length",
    "tsproc_mode",
    "servo_offset_threshold",
    "servo_num_offset_values",
    "offset_from_master_min",
    "phc_index",
    "phc2sysPollInterval",
    "domainNumber2",
    "msg_interval_request",
    "solo",
    "sync_e2e",
    "synce",
    "dscp_event",
    "dscp_general",
    "clock_update",
    "trace",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション総数。
    pub sections: usize,
    /// `key\tvalue`/`key=value`/`key value` エントリ総数。
    pub entries: usize,
    /// 時計属性系キー数。
    pub clock: usize,
    /// ポート/ネットワーク系キー数。
    pub port: usize,
    /// `[unicast_master_table]` 内エントリ数。
    pub unicast: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が `ptp4l.cfg` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2 && c.clock + c.port >= 1)
}

/// `b` を `ptp4l.cfg` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        clock: 0,
        port: 0,
        unicast: 0,
        misc: 0,
    };
    let mut in_uct = false;
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                let name = &rest[..end];
                in_uct = name == "unicast_master_table";
                c.sections += 1;
                continue;
            }
        }
        let key = line.split([' ', '\t', '=']).next().unwrap_or("");
        if key.is_empty() || key == line.trim() && !line.contains([' ', '\t', '=']) {
            continue;
        }
        c.entries += 1;
        if in_uct {
            c.unicast += 1;
            known += 1;
        } else if CLOCK_KEYS.contains(&key) || key.starts_with("G.8275") || key.starts_with("pi_") {
            c.clock += 1;
            known += 1;
        } else if PORT_KEYS.contains(&key) {
            c.port += 1;
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

    const SAMPLE: &[u8] = b"[global]\n\
        domainNumber\t\t\t24\n\
        time_stamping\t\t\thardware\n\
        twoStepFlag\t\t\t1\n\
        slaveOnly\t\t\t0\n\
        priority1\t\t\t128\n\
        priority2\t\t\t128\n\
        clockClass\t\t\t248\n\
        clockAccuracy\t\t\t0xFE\n\
        offsetScaledLogVariance\t\t0xFFFF\n\
        logMinDelayReqInterval\t\t0\n\
        logAnnounceInterval\t\t1\n\
        announceReceiptTimeout\t\t3\n\
        logSyncInterval\t\t\t0\n\
        delay_mechanism\t\t\tE2E\n\
        network_transport\t\tUDPv4\n\
        ptp_dst_mac\t\t\t01:1B:19:00:00:00\n\
        udp_ttl\t\t\t\t6\n\
        clock_servo\t\t\tpi\n\
        pi_proportional_const\t\t0.0\n\
        pi_integral_const\t\t0.0\n\
        kernel_leap\t\t\t1\n\
        sanity_freq_limit\t\t200000000\n\
        [eth0]\n\
        logAnnounceInterval\t\t1\n\
        [unicast_master_table]\n\
        table_id\t\t\t1\n\
        logQueryInterval\t\t2\n";

    #[test]
    fn detects_ptp4l() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.clock, 13);
        assert_eq!(c.port, 10);
        assert_eq!(c.unicast, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[foo]\nx=1\n"));
        assert!(!detect(b"domainNumber 24\n"));
    }
}
