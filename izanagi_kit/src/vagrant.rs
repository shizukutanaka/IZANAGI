//! Parser for Vagrantfiles (Ruby DSL).
//!
//! Extracts the `Vagrant.configure` version and first `config.vm.box`,
//! counts `vm.define`/`provider`/`network`/`forwarded_port`/`synced_folder`/
//! `provision`/`required_plugins` lines, and `#` comments.
//!
//! ```
//! let b = b"Vagrant.configure(\"2\") do |config|\n  config.vm.box = \"ubuntu/jammy64\"\n  config.vm.network \"forwarded_port\", guest: 80, host: 8080\nend\n";
//! assert!(izanagi_kit::vagrant::detect(b));
//! let v = izanagi_kit::vagrant::Vagrant::parse(b).unwrap();
//! assert_eq!(v.config_version, "2");
//! assert_eq!(v.box_name, "ubuntu/jammy64");
//! assert_eq!(v.networks, 1);
//! ```

/// Parsed Vagrantfile summary.
#[derive(Debug, Clone)]
pub struct Vagrant {
    /// `Vagrant.configure("…")` version argument.
    pub config_version: String,
    /// First `config.vm.box` value.
    pub box_name: String,
    /// `config.vm.box` assignments.
    pub boxes: usize,
    /// `config.vm.define` blocks.
    pub machines: usize,
    /// `config.vm.provider` blocks.
    pub providers: usize,
    /// `config.vm.network` calls.
    pub networks: usize,
    /// `forwarded_port` usages.
    pub forwarded_ports: usize,
    /// `config.vm.synced_folder` calls.
    pub synced_folders: usize,
    /// `config.vm.provision` calls.
    pub provisions: usize,
    /// `config.required_plugins` entries.
    pub required_plugins: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Returns `true` when `b` looks like a Vagrantfile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.contains("Vagrant.configure") || t.contains("config.vm.box")
}

impl Vagrant {
    /// Parses `b` as a Vagrantfile.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let config_version = t
            .lines()
            .find(|l| l.contains("Vagrant.configure"))
            .and_then(|l| l.split('"').nth(1))
            .unwrap_or("")
            .to_string();
        let box_name = t
            .lines()
            .find(|l| l.contains("config.vm.box") && l.contains('='))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
            .unwrap_or_default();
        let count = |needle: &str| t.lines().filter(|l| l.contains(needle)).count();
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            config_version,
            box_name,
            boxes: count("config.vm.box"),
            machines: count("config.vm.define"),
            providers: count("config.vm.provider"),
            networks: count("config.vm.network"),
            forwarded_ports: count("forwarded_port"),
            synced_folders: count("config.vm.synced_folder"),
            provisions: count("config.vm.provision"),
            required_plugins: count("required_plugins"),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# -*- mode: ruby -*-
Vagrant.configure(\"2\") do |config|
  config.vm.box = \"ubuntu/jammy64\"
  config.vm.hostname = \"dev\"
  config.vm.define :db do |db|
  end
  config.vm.network \"forwarded_port\", guest: 80, host: 8080
  config.vm.network \"private_network\", ip: \"192\x2e168\x2e33\x2e10\"
  config.vm.synced_folder \".\", \"/vagrant\"
  config.vm.provider \"virtualbox\" do |vb|
  end
  config.vm.provision \"shell\", inline: \"echo hi\"
end
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"puts 'hi'"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let v = Vagrant::parse(SRC).unwrap();
        assert_eq!(v.config_version, "2");
        assert_eq!(v.box_name, "ubuntu/jammy64");
        assert_eq!(v.boxes, 1);
        assert_eq!(v.machines, 1);
        assert_eq!(v.providers, 1);
        assert_eq!(v.networks, 2);
        assert_eq!(v.forwarded_ports, 1);
        assert_eq!(v.synced_folders, 1);
        assert_eq!(v.provisions, 1);
        assert_eq!(v.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Vagrant::parse(b"\x01\x02").is_none());
    }
}
