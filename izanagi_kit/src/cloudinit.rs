//! cloud-init `#cloud-config` format.
//!
//! cloud-config files begin `#cloud-config` and set top-level keys
//! per module: `hostname`, `users`, `ssh_authorized_keys`, `packages`,
//! `runcmd`, `write_files`, `growpart`, `resize_rootfs`, `mounts`,
//! `ntp`, `timezone`, `locale`, `keyboard`, `chpasswd`,
//! `ssh_pwauth`, `disable_root`, `package_update`, `package_upgrade`,
//! `final_message`, `power_state`, `bootcmd`, `snap`, `apt`, `zypper`,
//! `yum_repos`, `groups`, `manage_etc_hosts`, `fqdn`, `ssh_genkeytypes`,
//! `ssh_deletekeys`, `ssh_quirk_keygen`, `ssh_import_id`, `landscape`,
//! `puppet`, `chef`, `salt_minion`, `mcollective`, `resolv_conf`,
//! `ca_certs`, `disk_setup`, `fs_setup`, `lxd`, `fan`, `grub_dpkg`,
//! `rhn_subscription`, `spacewalk`, `ubuntu_advantage`, `ubuntu_pro`,
//! `updates`, `vendor_data`, `wireguard`, `runcmd`, `mount_default_fields`,
//! `package_reboot_if_required`, `package_update`, `package_upgrade`,
//! `install_hotplug`, `ssh`, `ssh_svcname`, `ssh_deletekeys`,
//! `ssh_genkeytypes`, `ssh_quirk_keygen`, `ssh_publish_hostkeys`,
//! `ssh_redirect_log`, `syslog_fix_perms`, `system_info`,
//! `default_user`, `distro`, `network`, `users-groups`, `write-files`,
//! `ntp_client`, `runcmd`, `scripts-user`, `scripts-per-instance`,
//! `scripts-per-boot`, `scripts-per-once`, `scripts-vendor`,
//! `seed_random`, `set_hostname`, `set_passwords`, `snappy`,
//! `spacewalk`, `ssh_authkey_fingerprints`, `ssh_import_id`,
//! `ssh_authorized_keys`, `timezone`, `update_etc_hosts`,
//! `update_hostname`, `users-groups`, `write-files`, `yum-add-repo`,
//! `zypper-add-repo`, `ca-certs`, `apk`, `apt-configure`,
//! `apt-pipelining`, `bootcmd`, `byobu`, `cc_*` module keys, and
//! `users: [name, ssh_import_id, lock_passwd, sudo, groups, shell,
//! ssh_authorized_keys]` sub-items.
//!
//! ```
//! let b = concat!(
//!     "#cloud-config\n",
//!     "hostname: vm\n",
//!     "users:\n",
//!     "  - default\n",
//!     "  - name: devin\n",
//!     "    sudo: ALL=(ALL) NOPASSWD:ALL\n",
//!     "ssh_authorized_keys:\n",
//!     "  - ssh-rsa AAAA\n",
//!     "packages:\n",
//!     "  - curl\n",
//!     "runcmd:\n",
//!     "  - echo hi\n"
//! ).as_bytes();
//! assert!(izanagi_kit::cloudinit::detect(b));
//! let c = izanagi_kit::cloudinit::Cloudinit::parse(b).unwrap();
//! assert_eq!(c.top_keys, 5);
//! assert_eq!(c.items, 5);
//! ```

use crate::textutil::strip_bom;
/// Parsed cloud-config summary.
#[derive(Debug, Clone)]
pub struct Cloudinit {
    /// Top-level `key:`/`key: value` occurrences.
    pub top_keys: usize,
    /// `- item` list entries.
    pub items: usize,
    /// `users:` list entries.
    pub users: usize,
    /// `packages:` entries.
    pub packages: usize,
    /// `runcmd`/`bootcmd` entries.
    pub commands: usize,
    /// `write_files` entries.
    pub write_files: usize,
    /// `ssh_authorized_keys`/`ssh_import_id`/`ssh_genkeytypes` entries.
    pub ssh: usize,
    /// `#` comment lines (excluding the `#cloud-config` marker).
    pub comments: usize,
}

const TOP_KW: &[&str] = &[
    "hostname",
    "fqdn",
    "prefer_fqdn_over_hostname",
    "preserve_hostname",
    "manage_etc_hosts",
    "users",
    "groups",
    "ssh_authorized_keys",
    "ssh_import_id",
    "ssh_genkeytypes",
    "ssh_deletekeys",
    "ssh_quirk_keygen",
    "ssh_pwauth",
    "disable_root",
    "chpasswd",
    "packages",
    "package_update",
    "package_upgrade",
    "package_reboot_if_required",
    "runcmd",
    "bootcmd",
    "write_files",
    "growpart",
    "resize_rootfs",
    "mounts",
    "mount_default_fields",
    "ntp",
    "timezone",
    "locale",
    "keyboard",
    "final_message",
    "power_state",
    "snap",
    "apt",
    "zypper",
    "yum_repos",
    "ca_certs",
    "resolv_conf",
    "rhn_subscription",
    "spacewalk",
    "ubuntu_advantage",
    "ubuntu_pro",
    "updates",
    "vendor_data",
    "wireguard",
    "fan",
    "grub_dpkg",
    "lxd",
    "disk_setup",
    "fs_setup",
    "landscape",
    "puppet",
    "chef",
    "salt_minion",
    "mcollective",
    "syslog_fix_perms",
    "system_info",
    "default_user",
    "distro",
    "network",
    "seed_random",
    "set_hostname",
    "set_passwords",
    "update_etc_hosts",
    "update_hostname",
    "apk",
    "apt_pipelining",
    "byobu",
    "ssh",
    "ssh_svcname",
    "ssh_publish_hostkeys",
    "ssh_redirect_log",
    "users-groups",
    "write-files",
    "yum-add-repo",
    "zypper-add-repo",
    "ca-certs",
    "apt-configure",
];

/// Whether the buffer looks like cloud-config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.trim_start().starts_with("#cloud-config")
}

impl Cloudinit {
    /// Parses a cloud-config summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            top_keys: 0,
            items: 0,
            users: 0,
            packages: 0,
            commands: 0,
            write_files: 0,
            ssh: 0,
            comments: 0,
        };
        let mut section: Option<&str> = None;
        let mut first = true;
        for l in t.lines() {
            let tr = l.trim_end();
            if tr.trim().is_empty() {
                continue;
            }
            if first {
                first = false;
                if tr.trim_start().starts_with("#cloud-config") {
                    continue;
                }
            }
            if tr.trim_start().starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("- ") || tr == "-" {
                c.items += 1;
                match section {
                    Some("users") => c.users += 1,
                    Some("packages") => c.packages += 1,
                    Some("runcmd") | Some("bootcmd") => c.commands += 1,
                    Some("write_files") => c.write_files += 1,
                    Some("ssh_authorized_keys") | Some("ssh_import_id") => c.ssh += 1,
                    _ => {}
                }
                continue;
            }
            if l.starts_with(' ') || l.starts_with('\t') {
                if tr.trim_start().starts_with("- ") || tr.trim() == "-" {
                    c.items += 1;
                    match section {
                        Some("users") => c.users += 1,
                        Some("packages") => c.packages += 1,
                        Some("runcmd") | Some("bootcmd") => c.commands += 1,
                        Some("write_files") => c.write_files += 1,
                        Some("ssh_authorized_keys") | Some("ssh_import_id") => c.ssh += 1,
                        _ => {}
                    }
                }
                continue;
            }
            if let Some((k, _)) = tr.split_once(':') {
                let k = k.trim();
                c.top_keys += 1;
                section = match k {
                    "users" => Some("users"),
                    "packages" => Some("packages"),
                    "runcmd" | "bootcmd" => Some("runcmd"),
                    "write_files" => Some("write_files"),
                    "ssh_authorized_keys" => Some("ssh_authorized_keys"),
                    "ssh_import_id" => Some("ssh_import_id"),
                    _ => None,
                };
                let _ = TOP_KW.contains(&k);
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cloudinit() {
        let b = concat!(
            "#cloud-config\n",
            "hostname: vm\n",
            "users:\n",
            "  - default\n",
            "  - name: devin\n",
            "    sudo: ALL=(ALL) NOPASSWD:ALL\n",
            "ssh_authorized_keys:\n",
            "  - ssh-rsa AAAA\n",
            "  - ssh-ed25519 BBBB\n",
            "packages:\n",
            "  - curl\n",
            "  - git\n",
            "runcmd:\n",
            "  - echo hi\n",
            "  - touch /x\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Cloudinit::parse(b).unwrap();
        assert_eq!(c.top_keys, 5);
        assert_eq!(c.items, 8);
        assert_eq!(c.users, 2);
        assert_eq!(c.packages, 2);
        assert_eq!(c.commands, 2);
        assert_eq!(c.ssh, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"hostname: vm\n"));
        assert!(Cloudinit::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
