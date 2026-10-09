//! Slurm `slurm.conf`/`slurmdbd.conf` census.
//!
//! slurm.conf is `Key=Value` lines (`#` comments); `NodeName=` /
//! `PartitionName=` entries describe compute nodes and queues.
//! Well-known keys include `SlurmctldHost`/`SlurmUser`/`SlurmdUser`/
//! `SlurmctldPort`/`SlurmdPort`/`StateSaveLocation`/`SlurmdSpoolDir`/
//! `ProctrackType`/`TaskPlugin`/`SchedulerType`/`SelectType`/
//! `SelectTypeParameters`/`MaxJobCount`/`KillWait`/`ReturnToService`/
//! `SlurmctldTimeout`/`SlurmdTimeout`/`MinJobAge`/`DebugFlags`/
//! `Prolog`/`Epilog`/`PrologSlurmctld`/`EpilogSlurmctld`/
//! `SuspendProgram`/`ResumeProgram`/`SuspendTime`/`SuspendRate`/
//! `ResumeRate`/`GresTypes`/`CpuBind`/`DefMemPerCPU`/`DefMemPerGPU`/
//! `MaxMemPerCPU`/`MaxMemPerNode`/`TmpFS`/`JobRequeue`/`PreemptMode`/
//! `PreemptType`/`PriorityType`/`PriorityDecayHalfLife`/
//! `PriorityWeightAge`/`PriorityWeightFairshare`/`PriorityWeightJobSize`/
//! `PriorityWeightPartition`/`PriorityWeightQOS`/`PriorityWeightTRES`/
//! `ClusterName`/`MpiDefault`/`AuthType`/`CryptoType`/
//! `AccountingStorageType`/`AccountingStorageHost`/`AcctGather*`/
//! `JobCompType`/`MailProg`/`MailDomain`/`TreeWidth`/`UsePAM`/
//! `SrunPortRange`/`Licenses`/`PowerPlugin`/`TopologyPlugin`.
//!
//! ```rust
//! let c = izanagi_kit::slurm::Slurm::parse(b"SlurmctldHost=ctl\nSlurmUser=slurm\nSelectType=x\nNodeName=n1 CPUs=4\n").unwrap();
//! assert_eq!(c.node_defs, 1);
//! ```

use crate::textutil::strip_bom;
/// slurm.conf census.
#[derive(Debug, Clone)]
pub struct Slurm {
    /// `Key=Value` entries.
    pub entries: usize,
    /// Entries matching the known Slurm parameter list.
    pub named: usize,
    /// `NodeName=` definitions.
    pub node_defs: usize,
    /// `PartitionName=` definitions.
    pub partition_defs: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "SlurmctldHost",
    "SlurmUser",
    "SlurmdUser",
    "SlurmctldPort",
    "SlurmdPort",
    "SlurmctldPidFile",
    "SlurmdPidFile",
    "SlurmdSpoolDir",
    "StateSaveLocation",
    "PluginDir",
    "PrivateData",
    "ProctrackType",
    "SwitchType",
    "TaskPlugin",
    "TaskPluginParam",
    "SchedulerType",
    "SelectType",
    "SelectTypeParameters",
    "MaxJobCount",
    "MaxArraySize",
    "KillWait",
    "KillOnBadExit",
    "MaxStepCount",
    "ReturnToService",
    "SlurmctldTimeout",
    "SlurmdTimeout",
    "InactiveLimit",
    "MinJobAge",
    "Waittime",
    "CompleteWait",
    "SlurmctldDebug",
    "SlurmdDebug",
    "DebugFlags",
    "Prolog",
    "Epilog",
    "PrologSlurmctld",
    "EpilogSlurmctld",
    "SuspendProgram",
    "ResumeProgram",
    "SuspendTime",
    "SuspendRate",
    "ResumeRate",
    "SuspendExcNodes",
    "SuspendExcParts",
    "GresTypes",
    "CpuBind",
    "DefCpuPerGPU",
    "DefMemPerCPU",
    "DefMemPerGPU",
    "DefMemPerNode",
    "MaxMemPerCPU",
    "MaxMemPerNode",
    "TmpFS",
    "JobRequeue",
    "RebootProgram",
    "PreemptMode",
    "PreemptType",
    "PreemptExemptTime",
    "PriorityType",
    "PriorityDecayHalfLife",
    "PriorityCalcPeriod",
    "PriorityFavorSmall",
    "PriorityFlags",
    "PriorityMaxAge",
    "PriorityUsageResetPeriod",
    "PriorityWeightAge",
    "PriorityWeightFairshare",
    "PriorityWeightJobSize",
    "PriorityWeightPartition",
    "PriorityWeightQOS",
    "PriorityWeightTRES",
    "ClusterName",
    "MpiDefault",
    "AuthType",
    "CryptoType",
    "AccountingStorageType",
    "AccountingStorageHost",
    "AccountingStorageUser",
    "AccountingStoragePort",
    "AcctGatherEnergyType",
    "AcctGatherInterconnectType",
    "AcctGatherFilesystemType",
    "AcctGatherProfileType",
    "JobCompType",
    "JobCompLoc",
    "MailProg",
    "MailDomain",
    "TreeWidth",
    "UsePAM",
    "DisableRootJobs",
    "EnforcePartLimits",
    "FirstJobId",
    "JobContainerType",
    "LaunchParameters",
    "LaunchType",
    "Licenses",
    "LogTimeFormat",
    "MessageTimeout",
    "PowerParameters",
    "PowerPlugin",
    "PropagateResourceLimits",
    "ResvOverRun",
    "SchedulerParameters",
    "SlurmctldLogFile",
    "SlurmdLogFile",
    "SlurmctldSyslogDebug",
    "SlurmdSyslogDebug",
    "SyslogFacility",
    "TopologyParam",
    "TopologyPlugin",
    "SrunPortRange",
    "SrunEpilog",
    "SrunProlog",
    "TokenTTL",
    "TrackWCKey",
];

fn key_of(s: &str) -> &str {
    s.split('=').next().unwrap_or("").trim()
}

/// Whether the buffer looks like slurm.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .filter(|l| {
            let s = l.trim_start();
            !s.is_empty() && !s.starts_with('#') && s.contains('=') && KEYS.contains(&key_of(s))
        })
        .count()
        >= 3
}

impl Slurm {
    /// Parse slurm.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            entries: 0,
            named: 0,
            node_defs: 0,
            partition_defs: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if !s.contains('=') {
                continue;
            }
            c.entries += 1;
            match key_of(s) {
                "NodeName" => c.node_defs += 1,
                "PartitionName" => c.partition_defs += 1,
                k => {
                    if KEYS.contains(&k) {
                        c.named += 1;
                    }
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
    fn parses_slurm_conf() {
        let b = concat!(
            "# slurm.conf\n",
            "SlurmctldHost=ctl1\n",
            "SlurmUser=slurm\n",
            "SelectType=select/cons_tres\n",
            "ProctrackType=proctrack/cgroup\n",
            "NodeName=n[01-04] CPUs=8 RealMemory=32768\n",
            "NodeName=n05 CPUs=8\n",
            "PartitionName=main Nodes=n[01-05] Default=YES MaxTime=INFINITE\n",
        );
        let c = Slurm::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 7);
        assert_eq!(c.named, 4);
        assert_eq!(c.node_defs, 2);
        assert_eq!(c.partition_defs, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Slurm::parse(b"a=1\nb=2\nc=3\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
