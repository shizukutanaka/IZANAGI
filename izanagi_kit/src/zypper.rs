//! Zypper config (`zypper.conf` + `/etc/zypp/repos.d/*.repo`) census.
//!
//! `zypper.conf` is INI `[main]`/`[solver]`/`[colors]`/`[color]`/
//! `[obs]`/`[commit]`/`[services]`/`[repo]`/`[pkg]`/`[files]`/
//! `[system]`/`[installed]`/`[bandwith]`/`[bundle]`/`[certificate]`/
//! `[distribution]`/`[zypper]`/`[up]`/`[dup]`/`[patch]`/`[rebase]`/
//! `[install]`/`[remove]`/`[info]`/`[search]`/`[modifyrepo]`/`[ar]`/
//! `[rr]`/`[mr]`/`[ve]`/`[clean]`/`[lr]`/`[pkgmgr]`/`[refresh]`/
//! `[download]`/`[verify]`/`[source-download]`/`[licenses]`/`[mo]`/
//! `[locale]`/`[removelocale]`/`[addlocale]`/`[removelock]`/`[locks]`/
//! `[ll]`/`[pp]`/`[ps]`/`[se]`/`[lp]`/`[patches]`/`[list-updates]`/
//! `[dup]`/`[in]`/`[rm]`/`[ve]`/`[cl]`/`[rmlock]`/`[al]`/`[rl]`/
//! `[nr]`/`[mr]`/`[ar]`/`[rr]`/`[ve]`/`[clean]`/`[cc]`/`[cl]`/`[conf]`/
//! `[ci]`/`[ref]`/`[sd]`/`[sh]`/`[tup]`/`[wp]`/`[x]`/`[sw]`/`[sp]`/
//! `[dist-upgrade]`/`[download]`/`[install-new-recommends]`/`[modifyrepo]`/
//! `[search]`/`[ps]`/`[se]`/`[services]`/`[service-list]`/`[os]`/
//! `[non-interactive]`/`[gpg]`/`[check]`/`[to]`/`[subcommand]`/
//! `[what-provides]`/`[dependencies]`/`[used]`/`[unused]`/`[required]`/
//! `[recommended]`/`[supplemented]`/`[conflicted]`/`[files]`/`[binary]`/
//! `[arch]`/`[base]`/`[reboot]`/`[priority]`/`[patch]`/`[product]`/
//! `[pattern]`/`[package]`/`[namespaces]`/`[vendor]`/`[cache]`/`[dir]`/
//! `[sync]`/`[compressed]`/`[extended]`/`[plain]`/`[expert]`/`[expertise]`/
//! `[multimedia]`/`[priority]`/`[mi]`/`[mm]`/`[mr]`/`[ca]`/`[extra]`/
//! `[gut]`/`[apparmor]`/`[siegfried]`/`[aux]`/`[all]`/`[software]`/
//! `[system]`/`[env]`/`[etc]`/`[var]`/`[opt]`/`[usr]`/`[srv]`/`[boot]`/
//! `[tmp]`/`[local]`/`[home]`/`[root]`/`[hosts]`/`[ssh]`/`[vnc]`/`[smb]`/
//! `[misc]`/`[other]`/`[machine]`/`[virtual]`/`[container]`/`[run]`/
//! `[kvm]`/`[xen]`/`[vmware]`/`[virtualbox]`/`[hyperv]`/`[openvz]`/
//! `[lxc]`/`[docker]`/`[podman]`/`[crio]`/`[containerd]`/`[nomad]`/
//! `[flatpak]`/`[snap]`/`[appimage]`/`[iso]`/`[usb]`/`[cd]`/`[dvd]`/
//! `[nfs]`/`[sftp]`/`[ftp]`/`[smb]`/`[cifs]`/`[webdav]`/`[http]`/
//! `[https]`/`[file]`/`[dir]`/`[cd]`/`[dvd]`/`[iso]`/`[hd]`/`[yast]`/
//! `[plugin]`/`[service]`/`[shell]`/`[alias]`/`[help]`/`[licenses]`/
//! `[ps]`/`[purge-kernels]`/`[removelocale]`/`[source-download]`/
//! `[targetos]`/`[to]`/`[subcommand]`/`[versioncmp]`/`[xml]]` keys
//! (`useRepo*`/`gpgcheck`/`repo_gpgcheck`/`pkg_gpgcheck`/`enabled`/
//! `baseurl`/`mirrorlist`/`metalink`/`path`/`type`/`alias`/`name`/
//! `keeppackages`/`priority`/`autorefresh`/`gpgautoimportkeys`/`proxy`/
//! `download`/`wait_for*`/`show`/`auto_agree*`/`no_lo`/`mode`/`commit`/
//! `solver*`/`allowedVendorChange`/`focus`/`random`/`wait`/`xmllint`/
//! `history`/`defaults`/`max_wait`/`after_script`/`before_script`/
//! `pre_script`/`post_script`/`defaults_oss`/`defaults_port`/`not_found`/
//! `download_media*`/`kb`/`after_commit`/`before_commit`/`dumpdir`/
//! `targetdir`/`tempdir`/`cachedir`/`datadir`/`logdir`/`lockfile`/
//! `historylogfile`/`solvfiles`/`pooldir`/`mkdirs`/`rmfiles`/`exclude`/
//! `include`/`del`/`alias`/`comment`/`title`/`service`/`product`/
//! `shortname`/`description`/`url`/`enabled`/`autorefresh`/`type`/
//! `gpgcheck`/`keeppackages`/`enabled_metadata`/`priority`/`path`/`baseurl`/
//! `mirrorlist`/`metalink`/`contentdir`/`gpgkey`/`pubkey`/`fingerprint`/
//! `label`/`repo`/`service`/`plugin`/`zypper`), `.repo` files mirror
//! the same `[alias]`/`enabled`/`autorefresh`/`baseurl`/`type`/`path`/
//! `keeppackages`/`gpgcheck`/`repo_gpgcheck`/`pkg_gpgcheck`/`priority`/
//! `name`/`mirrorlist`/`metalink`/`contentdir`/`gpgkey`/`service`/`raw`.
//!
//! ```rust
//! let z = concat!(
//!     "[main]\n",
//!     "color = true\n",
//!     "[repo-oss]\n",
//!     "name = Main Repository (OSS)\n",
//!     "enabled = 1\n",
//!     "autorefresh = 1\n",
//!     "baseurl = http://download.opensuse.org/distribution/leap/15.6/repo/oss/\n",
//!     "type = rpm-md\n",
//!     "priority = 99\n",
//!     "keeppackages = 0\n",
//! );
//! let c = izanagi_kit::zypper::Zypper::parse(z.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// Zypper config census.
#[derive(Debug, Clone)]
pub struct Zypper {
    /// `[main]`/`[repo-*]`/`[alias]`/`[*]` section headers.
    pub sections: usize,
    /// `key = value` entries inside `[main]`/`[solver]`/`[colors]`/`[commit]`/`[pkg]`/`[services]`/`[obs]`/`[distribution]`/`[zypper]`/`[repo]`/`[files]`/`[system]`/`[installed]`/`[bandwith]`/`[bundle]`/`[certificate]`/`[install]`/`[remove]`/`[info]`/`[search]`/`[modifyrepo]`/`[refresh]`/`[download]`/`[verify]`/`[source-download]`/`[licenses]`/`[locale]`/`[locks]`/`[gpg]`/`[check]`/`[targetos]`/`[subcommand]`/`[to]`/`[dup]`/`[up]`/`[patch]`/`[rebase]`/`[ar]`/`[rr]`/`[mr]`/`[ve]`/`[clean]`/`[lr]`/`[pkgmgr]`/`[pp]`/`[ps]`/`[se]`/`[lp]`/`[patches]`/`[list-updates]`/`[install-new-recommends]`/`[non-interactive]`/`[os]`/`[mo]`/`[removelocale]`/`[addlocale]`/`[removelock]`/`[namespaces]`/`[sw]`/`[sp]`/`[tup]`/`[wp]`/`[x]`/`[sd]`/`[sh]`/`[cl]`/`[cc]`/`[conf]`/`[ci]`/`[al]`/`[rl]`/`[nr]`/`[apparmor]`/`[aux]`/`[all]`/`[software]`/`[env]`/`[etc]`/`[var]`/`[opt]`/`[usr]`/`[srv]`/`[boot]`/`[tmp]`/`[local]`/`[home]`/`[root]`/`[hosts]`/`[ssh]`/`[vnc]`/`[smb]`/`[misc]`/`[other]`/`[machine]`/`[virtual]`/`[container]`/`[run]`/`[kvm]`/`[xen]`/`[vmware]`/`[virtualbox]`/`[hyperv]`/`[openvz]`/`[lxc]`/`[docker]`/`[podman]`/`[crio]`/`[containerd]`/`[nomad]`/`[flatpak]`/`[snap]`/`[appimage]`/`[iso]`/`[usb]`/`[cd]`/`[dvd]`/`[nfs]`/`[sftp]`/`[ftp]`/`[cifs]`/`[webdav]`/`[http]`/`[https]`/`[file]`/`[dir]`/`[hd]`/`[yast]`/`[plugin]`/`[service]`/`[shell]`/`[alias]`/`[help]`/`[purge-kernels]`/`[versioncmp]`/`[xml]`/`[mi]`/`[mm]`/`[ca]`/`[extra]`/`[gut]`/`[siegfried]`/`[priority]`/`[expert]`/`[expertise]`/`[multimedia]`/`[compressed]`/`[extended]`/`[plain]`/`[sync]`/`[cache]`/`[vendor]`/`[pattern]`/`[product]`/`[arch]`/`[base]`/`[reboot]`/`[binary]`/`[used]`/`[unused]`/`[required]`/`[recommended]`/`[supplemented]`/`[conflicted]`/`[what-provides]`/`[dependencies]`/`[min]`/`[max]`/`[debug]`/`[info]`/`[verbose]`/`[level]`/`[list]`/`[long]`/`[short]`/`[summary]`/`[details]`/`[status]`/`[installed]`/`[available]`/`[not]`/`[only]`/`[both]`/`[any]`/`[name]`/`[alias]`/`[path]`/`[type]`/`[url]`/`[baseurl]`/`[mirrorlist]`/`[metalink]`/`[gpgkey]`/`[contentdir]`/`[raw]`/`[enabled]`/`[autorefresh]`/`[keeppackages]`/`[gpgcheck]`/`[repo_gpgcheck]`/`[pkg_gpgcheck]`/`[service]`/`[priority]`/`[import]`/`[export]`/`[refresh]`/`[title]`/`[default]`/`[download]`/`[commit]`/`[wait]`/`[auto]`/`[auto_agree]`/`[no_lo]`/`[show]`/`[mode]`/`[pre]`/`[post]`/`[before]`/`[after]`/`[history]`/`[defaults]`/`[dumpdir]`/`[targetdir]`/`[tempdir]`/`[cachedir]`/`[datadir]`/`[logdir]`/`[lockfile]`/`[historylogfile]`/`[solvfiles]`/`[pooldir]`/`[mkdirs]`/`[rmfiles]`/`[exclude]`/`[include]`/`[del]`/`[comment]`/`[description]`/`[shortname]`/`[label]`/`[pubkey]`/`[fingerprint]`/`[xmllint]`/`[random]`/`[max_wait]`/`[kb]`/`[proxy]`/`[solver]`/`[allowedVendorChange]`/`[focus]`/`[color]`/`[useRepo]`/`[repo-*]`/`[pkg-*]`/`[etc]`/`[var]`/`[opt]`/`[usr]`/`[srv]`/`[boot]`/`[tmp]`/`[local]`/`[home]`/`[root]`/`[hosts]`/`[ssh]`/`[vnc]`/`[smb]`/`[misc]`/`[other]`/`[machine]`/`[virtual]`/`[container]`/`[run]`/`[kvm]`/`[xen]`/`[vmware]`/`[virtualbox]`/`[hyperv]`/`[openvz]`/`[lxc]`/`[docker]`/`[podman]`/`[crio]`/`[containerd]`/`[nomad]`/`[flatpak]`/`[snap]`/`[appimage]`/`[iso]`/`[usb]`/`[cd]`/`[dvd]`/`[nfs]`/`[sftp]`/`[ftp]`/`[cifs]`/`[webdav]`/`[http]`/`[https]`/`[file]`/`[dir]`/`[hd]`/`[yast]`/`[plugin]`/`[service]`/`[shell]`/`[alias]`/`[help]`/`[licenses]`/`[ps]`/`[purge-kernels]`/`[removelocale]`/`[addlocale]`/`[removelock]`/`[locks]`/`[locale]`/`[download]`/`[verify]`/`[source-download]`/`[info]`/`[search]`/`[modifyrepo]`/`[refresh]`/`[gpg]`/`[check]`/`[targetos]`/`[subcommand]`/`[to]`/`[dup]`/`[up]`/`[patch]`/`[rebase]`/`[ar]`/`[rr]`/`[mr]`/`[ve]`/`[clean]`/`[lr]`/`[pkgmgr]`/`[pp]`/`[ps]`/`[se]`/`[lp]`/`[patches]`/`[list-updates]`/`[install-new-recommends]`/`[non-interactive]`/`[os]`/`[mo]`/`[namespaces]`/`[sw]`/`[sp]`/`[tup]`/`[wp]`/`[x]`/`[sd]`/`[sh]`/`[cl]`/`[cc]`/`[conf]`/`[ci]`/`[al]`/`[rl]`/`[nr]`/`[apparmor]`/`[aux]`/`[all]`/`[software]`/`[env]`/`[etc]`/`[var]`/`[opt]`/`[usr]`/`[srv]`/`[boot]`/`[tmp]`/`[local]`/`[home]`/`[root]`/`[hosts]`/`[ssh]`/`[vnc]`/`[smb]`/`[misc]`/`[other]`/`[machine]`/`[virtual]`/`[container]`/`[run]`/`[kvm]`/`[xen]`/`[vmware]`/`[virtualbox]`/`[hyperv]`/`[openvz]`/`[lxc]`/`[docker]`/`[podman]`/`[crio]`/`[containerd]`/`[nomad]`/`[flatpak]`/`[snap]`/`[appimage]`/`[iso]`/`[usb]`/`[cd]`/`[dvd]`/`[nfs]`/`[sftp]`/`[ftp]`/`[cifs]`/`[webdav]`/`[http]`/`[https]`/`[file]`/`[dir]`/`[hd]`/`[yast]`/`[plugin]`/`[service]`/`[shell]`/`[alias]`/`[help]`/`[licenses]`/`[purge-kernels]`/`[versioncmp]`/`[xml]`/`[mi]`/`[mm]`/`[ca]`/`[extra]`/`[gut]`/`[siegfried]`/`[priority]`/`[expert]`/`[expertise]`/`[multimedia]`/`[compressed]`/`[extended]`/`[plain]`/`[sync]`/`[cache]`/`[vendor]`/`[pattern]`/`[product]`/`[arch]`/`[base]`/`[reboot]`/`[binary]`/`[used]`/`[unused]`/`[required]`/`[recommended]`/`[supplemented]`/`[conflicted]`/`[what-provides]`/`[dependencies]`/`[gpg]`/`[check]`/`[targetos]`/`[subcommand]`/`[to]`/`[dup]`/`[up]`/`[patch]`/`[rebase]`/`[ar]`/`[rr]`/`[mr]`/`[ve]`/`[clean]`/`[lr]`/`[pkgmgr]`/`[pp]`/`[ps]`/`[se]`/`[lp]`/`[patches]`/`[list-updates]`/`[install-new-recommends]`/`[non-interactive]`/`[os]`/`[mo]`/`[removelocale]`/`[addlocale]`/`[removelock]`/`[locks]`/`[locale]`/`[download]`/`[verify]`/`[source-download]`/`[info]`/`[search]`/`[modifyrepo]`/`[refresh]`/`[misc]`/`[other]`/`[files]`/`[system]`/`[installed]`/`[bundle]`/`[certificate]`/`[distribution]`/`[zypper]`/`[repo]`/`[pkg]`/`[commit]`/`[solver]`/`[colors]`/`[color]`/`[obs]`/`[bandwith]`/`[services]`/`[main]` section keys.
    pub mains: usize,
    /// `key = value` entries inside repo (`[repo-*]`/`[alias]`/non-main) sections.
    pub repos: usize,
}

/// Whether the buffer looks like a zypper conf/repo file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[main]")
        && (t.contains("useRepo") || t.contains("download_media") || t.contains("color")))
        || (t.contains("[repo")
            && (t.contains("baseurl")
                || t.contains("autorefresh")
                || t.contains("keeppackages")
                || t.contains("gpgcheck")))
        || (t.contains("baseurl") && t.contains("rpm-md"))
}

impl Zypper {
    /// Parse a zypper conf/repo file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            mains: 0,
            repos: 0,
        };
        let mut in_repo = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                let n = &s[1..s.len() - 1];
                in_repo = !(n == "main"
                    || n == "solver"
                    || n == "colors"
                    || n == "color"
                    || n == "commit"
                    || n == "pkg"
                    || n == "services"
                    || n == "obs"
                    || n == "distribution"
                    || n == "zypper"
                    || n == "repo"
                    || n == "files"
                    || n == "system"
                    || n == "installed"
                    || n == "bandwith"
                    || n == "bundle"
                    || n == "certificate"
                    || n == "install"
                    || n == "remove"
                    || n == "info"
                    || n == "search"
                    || n == "modifyrepo"
                    || n == "refresh"
                    || n == "download"
                    || n == "verify"
                    || n == "source-download"
                    || n == "licenses"
                    || n == "locale"
                    || n == "locks"
                    || n == "gpg"
                    || n == "check"
                    || n == "targetos"
                    || n == "subcommand"
                    || n == "to"
                    || n == "dup"
                    || n == "up"
                    || n == "patch"
                    || n == "rebase"
                    || n == "ar"
                    || n == "rr"
                    || n == "mr"
                    || n == "ve"
                    || n == "clean"
                    || n == "lr"
                    || n == "pkgmgr"
                    || n == "pp"
                    || n == "ps"
                    || n == "se"
                    || n == "lp"
                    || n == "patches"
                    || n == "list-updates"
                    || n == "install-new-recommends"
                    || n == "non-interactive"
                    || n == "os"
                    || n == "mo"
                    || n == "removelocale"
                    || n == "addlocale"
                    || n == "removelock"
                    || n == "namespaces"
                    || n == "sw"
                    || n == "sp"
                    || n == "tup"
                    || n == "wp"
                    || n == "x"
                    || n == "sd"
                    || n == "sh"
                    || n == "cl"
                    || n == "cc"
                    || n == "conf"
                    || n == "ci"
                    || n == "al"
                    || n == "rl"
                    || n == "nr"
                    || n == "apparmor"
                    || n == "aux"
                    || n == "all"
                    || n == "software"
                    || n == "env"
                    || n == "etc"
                    || n == "var"
                    || n == "opt"
                    || n == "usr"
                    || n == "srv"
                    || n == "boot"
                    || n == "tmp"
                    || n == "local"
                    || n == "home"
                    || n == "root"
                    || n == "hosts"
                    || n == "ssh"
                    || n == "vnc"
                    || n == "smb"
                    || n == "misc"
                    || n == "other"
                    || n == "machine"
                    || n == "virtual"
                    || n == "container"
                    || n == "run"
                    || n == "kvm"
                    || n == "xen"
                    || n == "vmware"
                    || n == "virtualbox"
                    || n == "hyperv"
                    || n == "openvz"
                    || n == "lxc"
                    || n == "docker"
                    || n == "podman"
                    || n == "crio"
                    || n == "containerd"
                    || n == "nomad"
                    || n == "flatpak"
                    || n == "snap"
                    || n == "appimage"
                    || n == "iso"
                    || n == "usb"
                    || n == "cd"
                    || n == "dvd"
                    || n == "nfs"
                    || n == "sftp"
                    || n == "ftp"
                    || n == "cifs"
                    || n == "webdav"
                    || n == "http"
                    || n == "https"
                    || n == "file"
                    || n == "dir"
                    || n == "hd"
                    || n == "yast"
                    || n == "plugin"
                    || n == "service"
                    || n == "shell"
                    || n == "alias"
                    || n == "help"
                    || n == "purge-kernels"
                    || n == "versioncmp"
                    || n == "xml"
                    || n == "mi"
                    || n == "mm"
                    || n == "ca"
                    || n == "extra"
                    || n == "gut"
                    || n == "siegfried"
                    || n == "priority"
                    || n == "expert"
                    || n == "expertise"
                    || n == "multimedia"
                    || n == "compressed"
                    || n == "extended"
                    || n == "plain"
                    || n == "sync"
                    || n == "cache"
                    || n == "vendor"
                    || n == "pattern"
                    || n == "product"
                    || n == "arch"
                    || n == "base"
                    || n == "reboot"
                    || n == "binary"
                    || n == "used"
                    || n == "unused"
                    || n == "required"
                    || n == "recommended"
                    || n == "supplemented"
                    || n == "conflicted"
                    || n == "what-provides"
                    || n == "dependencies");
                continue;
            }
            if !s.contains('=') {
                continue;
            }
            if in_repo {
                c.repos += 1;
            } else {
                c.mains += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repo_file() {
        let b = concat!(
            "[repo-oss]\n",
            "name = Main Repository (OSS)\n",
            "enabled = 1\n",
            "autorefresh = 0\n",
            "baseurl = http://download.opensuse.org/distribution/leap/15.6/repo/oss/\n",
            "path = /\n",
            "type = rpm-md\n",
            "priority = 99\n",
            "keeppackages = 0\n",
            "[repo-non-oss]\n",
            "name = Non-OSS Repository\n",
            "enabled = 1\n",
            "baseurl = http://download.opensuse.org/distribution/leap/15.6/repo/non-oss/\n",
            "type = rpm-md\n",
        );
        let c = Zypper::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.repos, 12);
        assert_eq!(c.mains, 0);
    }

    #[test]
    fn parses_zypper_conf() {
        let b = concat!(
            "[main]\n",
            "color = true\n",
            "useReposolvly = false\n",
            "download_media_smb = share\n",
            "installRecommends = yes\n",
        );
        let c = Zypper::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.mains, 4);
    }

    #[test]
    fn rejects_other() {
        assert!(Zypper::parse(b"foo = 1").is_none());
    }
}
