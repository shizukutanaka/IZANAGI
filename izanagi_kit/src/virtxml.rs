//! libvirt domain XML format.
//!
//! libvirt domain XML starts `<domain type='…'>` and carries `<name>`,
//! `<vcpu>`, `<memory>`, `<os>`/`<type>`, `<features>`, `<cpu>`,
//! `<devices>` with `<disk>`/`<interface>`/`<graphics>`/`<input>`/
//! `<serial>`/`<console>`/`<video>`/`<sound>`/`<hostdev>`/`<rng>`/
//! `<watchdog>`/`<memballoon>` devices, `<emulator>` path, `<on_poweroff>`/
//! `<on_reboot>`/`<on_crash>` actions, `<seclabel>`, `<pm>`, `<clock>`.
//!
//! ```
//! let b = concat!(
//!     "<domain type='kvm'>\n",
//!     "  <name>vm1</name>\n",
//!     "  <memory>1048576</memory>\n",
//!     "  <vcpu>2</vcpu>\n",
//!     "  <os><type>hvm</type></os>\n",
//!     "  <devices>\n",
//!     "    <disk type='file'/>\n",
//!     "    <interface type='network'/>\n",
//!     "  </devices>\n",
//!     "</domain>\n"
//! ).as_bytes();
//! assert!(izanagi_kit::virtxml::detect(b));
//! let c = izanagi_kit::virtxml::Virtxml::parse(b).unwrap();
//! assert_eq!(c.devices, 2);
//! ```

/// Parsed libvirt domain XML summary.
#[derive(Debug, Clone)]
pub struct Virtxml {
    /// `<name>` occurrences.
    pub name: usize,
    /// `<vcpu>` occurrences.
    pub vcpu: usize,
    /// `<memory>`/`<currentMemory>` occurrences.
    pub memory: usize,
    /// `<features>` children (`acpi`, `apic`, `pae`, `privnet`, `hap`, `viridian`, `hyperv`, `kvm`, `pmu`, `vmport`, `smm`, `ioapic`, `hyperv_*`, `msrs`, `kvm_hidden`, `pvspinlock`, `gic`, `smm`, `tcg`, `xen`, `vmcoreinfo`, `ras`, `ps2`, `pv`, `pv_eoi`, `pv_ipi`, `pv_mmu`, `pv_tlb_flush`, `pv_evmcs`, `pv_poll_control`, `pv_schedyield`, `pv_sti`, `pv_synic`, `pv_vpindex`, `pv_frequencies`, `pv_reenlightenment`, `pv_spinlocks`, `hv_*` sub-features).
    pub features: usize,
    /// `<cpu>` attributes/children.
    pub cpu: usize,
    /// Device elements under `<devices>` (`disk`, `interface`, `graphics`, `input`, `serial`, `console`, `video`, `sound`, `hostdev`, `rng`, `watchdog`, `memballoon`, `controller`, `channel`, `smartcard`, `parallel`, `lease`, `filesystem`, `redirdev`, `hub`, `tpm`, `vsock`, `shmem`, `memorydev`, `iommu`, `panic`, `nvdimm`, `audio`, `crypto`, `sev`).
    pub devices: usize,
    /// `<emulator>` path lines.
    pub emulator: usize,
    /// `<on_poweroff>`/`<on_reboot>`/`<on_crash>` actions.
    pub on_actions: usize,
    /// `<seclabel>` elements.
    pub seclabel: usize,
    /// `<clock>` occurrences.
    pub clock: usize,
}

/// Whether the buffer looks like libvirt domain XML.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<domain") && (t.contains("<devices") || t.contains("<vcpu"))
}

const DEVICE_KW: &[&str] = &[
    "disk",
    "interface",
    "graphics",
    "input",
    "serial",
    "console",
    "video",
    "sound",
    "hostdev",
    "rng",
    "watchdog",
    "memballoon",
    "controller",
    "channel",
    "smartcard",
    "parallel",
    "lease",
    "filesystem",
    "redirdev",
    "hub",
    "tpm",
    "vsock",
    "shmem",
    "memorydev",
    "iommu",
    "panic",
    "nvdimm",
    "audio",
    "crypto",
    "sev",
];
const FEATURE_KW: &[&str] = &[
    "acpi",
    "apic",
    "pae",
    "privnet",
    "hap",
    "viridian",
    "hyperv",
    "kvm",
    "pmu",
    "vmport",
    "smm",
    "ioapic",
    "gic",
    "xen",
    "vmcoreinfo",
    "ras",
    "ps2",
    "pv",
    "msrs",
    "kvm_hidden",
    "pvspinlock",
    "pv_eoi",
    "tcg",
];

fn tag_count(t: &str, n: &str) -> usize {
    t.matches(&format!("<{n} ")).count()
        + t.matches(&format!("<{n}/")).count()
        + t.matches(&format!("<{n}>")).count()
}

impl Virtxml {
    /// Parses a libvirt domain XML summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let c = Self {
            name: tag_count(t, "name"),
            vcpu: tag_count(t, "vcpu"),
            memory: tag_count(t, "memory") + tag_count(t, "currentMemory"),
            features: FEATURE_KW.iter().map(|k| tag_count(t, k)).sum(),
            cpu: tag_count(t, "cpu"),
            devices: DEVICE_KW.iter().map(|k| tag_count(t, k)).sum(),
            emulator: tag_count(t, "emulator"),
            on_actions: tag_count(t, "on_poweroff")
                + tag_count(t, "on_reboot")
                + tag_count(t, "on_crash"),
            seclabel: tag_count(t, "seclabel"),
            clock: tag_count(t, "clock"),
        };
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_virtxml() {
        let b = concat!(
            "<domain type='kvm'>\n",
            "  <name>vm1</name>\n",
            "  <memory>1048576</memory>\n",
            "  <currentMemory>524288</currentMemory>\n",
            "  <vcpu>2</vcpu>\n",
            "  <os><type>hvm</type></os>\n",
            "  <features><acpi/><apic/><pae/></features>\n",
            "  <cpu mode='host-passthrough'/>\n",
            "  <clock offset='utc'/>\n",
            "  <on_poweroff>destroy</on_poweroff>\n",
            "  <devices>\n",
            "    <emulator>/usr/bin/qemu</emulator>\n",
            "    <disk type='file'/>\n",
            "    <interface type='network'/>\n",
            "    <graphics type='vnc'/>\n",
            "    <serial type='pty'/>\n",
            "    <memballoon model='virtio'/>\n",
            "  </devices>\n",
            "  <seclabel type='none'/>\n",
            "</domain>\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Virtxml::parse(b).unwrap();
        assert_eq!(c.name, 1);
        assert_eq!(c.vcpu, 1);
        assert_eq!(c.memory, 2);
        assert_eq!(c.features, 3);
        assert_eq!(c.cpu, 1);
        assert_eq!(c.devices, 5);
        assert_eq!(c.emulator, 1);
        assert_eq!(c.on_actions, 1);
        assert_eq!(c.seclabel, 1);
        assert_eq!(c.clock, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"<html/>\n"));
        assert!(Virtxml::parse(b"x").is_none());
    }
}
