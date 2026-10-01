//! Gentoo Portage `make.conf`/`package.use`/`package.accept_keywords`
//! census.
//!
//! `make.conf`: `VAR="value"`/`VAR='value'`/`VAR=value` shell-style
//! assignments (`CHOST`/`CFLAGS`/`CXXFLAGS`/`FCFLAGS`/`FFLAGS`/`LDFLAGS`/
//! `MAKEOPTS`/`PORTDIR`/`DISTDIR`/`PKGDIR`/`PORTAGE_TMPDIR`/`USE`/`LINGUAS`/
//! `L10N`/`VIDEO_CARDS`/`INPUT_DEVICES`/`ALSA_CARDS`/`APACHE2_MODULES`/
//! `CALLIGRA_FEATURES`/`CAMERAS`/`COLLECTD_PLUGINS`/`CPU_FLAGS_X86`/
//! `CPU_FLAGS_ARM`/`CPU_FLAGS_PPC`/`CURL_SSL`/`DVB_CARDS`/`ELIBC`/`FFTOOLS`/
//! `FOO2ZJS_DEVICES`/`FRITZCAPI_CARDS`/`GPSD_PROTOCOLS`/`GRUB_PLATFORMS`/
//! `INPUT_DEVICES`/`KERNEL`/`LIRC_DEVICES`/`LLVM_TARGETS`/`LUA_TARGETS`/
//! `MONKEYD_PLUGINS`/`NGINX_MODULES_HTTP`/`NGINX_MODULES_MAIL`/
//! `NGINX_MODULES_STREAM`/`OFFICE_IMPLEMENTATION`/`OPENCL_TARGETS`/
//! `OPENMPI_FABRICS`/`OPENMPI_OFED_FEATURES`/`PHP_TARGETS`/`POSTGRES_TARGETS`/
//! `PYTHON_TARGETS`/`PYTHON_SINGLE_TARGET`/`QEMU_SOFTMMU_TARGETS`/
//! `QEMU_USER_TARGETS`/`RUBY_TARGETS`/`SANE_BACKENDS`/`TWOLAME`/
//! `USERLAND`/`UWSGI_PLUGINS`/`VIDEO_CARDS`/`VOICEMAIL_STORAGE`/
//! `XFCE_PLUGINS`/`XTABLES_ADDONS`/`EMERGE_DEFAULT_OPTS`/`FEATURES`/
//! `ACCEPT_KEYWORDS`/`ACCEPT_LICENSE`/`ACCEPT_PROPERTIES`/`ACCEPT_RESTRICT`/
//! `GENTOO_MIRRORS`/`PORTAGE_GPG_KEY`/`PORTAGE_GPG_DIR`/`PORTAGE_RSYNC_RETRIES`/
//! `PORTAGE_RSYNC_EXTRA_OPTS`/`SYNC`/`PORTAGE_ELOG_CLASSES`/`PORTAGE_ELOG_SYSTEM`/
//! `PORTAGE_ELOG_MAILURI`/`PORTAGE_ELOG_MAILFROM`/`PORTAGE_ELOG_MAILSUBJECT`/
//! `PORTAGE_NICENESS`/`PORTAGE_IONICE_COMMAND`/`FETCHCOMMAND`/`RESUMECOMMAND`/
//! `PORTAGE_RO_DISTDIRS`/`PORTAGE_QUIET`/`PORTAGE_VERBOSE`/`PORTAGE_DEBUG`/
//! `PORTAGE_WORKDIR_MODE`/`PORTAGE_BZIP2_COMMAND`/`PORTAGE_BUNZIP2_COMMAND`/
//! `PORTAGE_XZ_COMMAND`/`PORTAGE_LZIP_COMMAND`/`PORTAGE_LZOP_COMMAND`/
//! `PORTAGE_ZSTD_COMMAND`/`BINPKG_COMPRESS`/`BINPKG_COMPRESS_FLAGS`/
//! `BINPKG_FORMAT`/`PORTAGE_BINHOST`/`PORTAGE_BINPKG_HEADER_OFFSET`/
//! `CLEAN_DELAY`/`EMERGE_WARNING_DELAY`/`TERM`/`NOCOLOR`/`COLORFGBG`/
//! `BASH_ENV`/`ENV`/`EXPAND_*`/`USE_EXPAND*`/`USE_ORDER`/`USE_EXPAND_HIDDEN`/
//! `PROFILE_ONLY_VARIABLES`/`CONFIG_PROTECT`/`CONFIG_PROTECT_MASK`/`EBEEP`/
//! `EPAUSE`/`PORTAGE_TMPFS`/`PORTAGE_SSH`/`UNINSTALL_IGNORE`/`INSTALL_MASK`/
//! `RPMDIR`/`PKG_CONFIG_PATH`/`POLICY_TYPES`/`GLSA_*`), plus `source`-like
//! `pkg_*` bash function headers and `#` comments.
//! `package.use`/`package.accept_keywords`/`package.license`/`package.mask`/
//! `package.unmask`/`package.env`/`package.accept_restrict`/`package.properties`/
//! `package.bashrc` lines: `cat/pkg flag1 flag2`, `cat/pkg ~amd64`,
//! `cat/pkg -flag`, `>=cat/pkg-1.0 flag`, `cat/pkg:slot flag`,
//! `cat/pkg:slot/sub flag`, `=cat/pkg-1.0* ~arch`, `~cat/pkg-1.0`.
//!
//! ```rust
//! let p = concat!(
//!     "CHOST=\"x86_64-pc-linux-gnu\"\n",
//!     "CFLAGS=\"-O2 -pipe\"\n",
//!     "MAKEOPTS=\"-j8\"\n",
//!     "USE=\"dbus wayland -kde\"\n",
//!     "ACCEPT_KEYWORDS=\"amd64\"\n",
//! );
//! let c = izanagi_kit::portage::Portage::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.assigns, 5);
//! ```

/// Portage config census.
#[derive(Debug, Clone)]
pub struct Portage {
    /// `VAR="…"`/`VAR='…'`/`VAR=…` assignments (make.conf).
    pub assigns: usize,
    /// package.use-style atom lines (`cat/pkg flags`, `>=cat/pkg ~arch`, `=cat/pkg-1.0*`, `~cat/pkg`, `cat/pkg:slot` forms).
    pub atoms: usize,
    /// `USE`/`ACCEPT_*`/`FEATURES`/`EMERGE_DEFAULT_OPTS`/`PORTAGE_*`/`SYNC`/`GENTOO_MIRRORS`/`CONFIG_PROTECT*`/`MAKEOPTS`/`CFLAGS`/`CXXFLAGS`/`FCFLAGS`/`FFLAGS`/`LDFLAGS`/`CHOST`/`RUBY_TARGETS`/`PYTHON_*`/`VIDEO_CARDS`/`INPUT_DEVICES`/`LINGUAS`/`L10N`/`CPU_FLAGS*`/`*_TARGETS`/`*_MODULES`/`GRUB_PLATFORMS`/`CURL_SSL`/`APACHE2_*`/`CALLIGRA_*`/`NGINX_*`/`QEMU_*`/`SANE_*`/`DVB_*`/`CAMERAS`/`COLLECTD_*`/`ELIBC`/`FFTOOLS`/`FOO2ZJS_*`/`FRITZCAPI_*`/`GPSD_*`/`KERNEL`/`LIRC_*`/`LLVM_*`/`MONKEYD_*`/`OFFICE_*`/`OPENCL_*`/`OPENMPI_*`/`PHP_*`/`POSTGRES_*`/`TWOLAME`/`USERLAND`/`UWSGI_*`/`VOICEMAIL_*`/`XFCE_*`/`XTABLES_*`/`FETCHCOMMAND`/`RESUMECOMMAND`/`PORTDIR`/`DISTDIR`/`PKGDIR`/`PORTAGE_TMPDIR`/`RPMDIR`/`PKG_CONFIG_PATH`/`POLICY_TYPES`/`CLEAN_DELAY`/`EMERGE_WARNING_DELAY`/`TERM`/`NOCOLOR`/`COLORFGBG`/`BASH_ENV`/`ENV`/`EBEEP`/`EPAUSE`/`PORTAGE_TMPFS`/`PORTAGE_SSH`/`UNINSTALL_IGNORE`/`INSTALL_MASK`/`GLSA_*`/`USE_ORDER`/`USE_EXPAND*`/`USE_EXPAND_HIDDEN`/`EXPAND_*`/`PROFILE_ONLY_*`/`PORTAGE_RO_DISTDIRS`/`PORTAGE_QUIET`/`PORTAGE_VERBOSE`/`PORTAGE_DEBUG`/`PORTAGE_WORKDIR_MODE`/`PORTAGE_BZIP2_COMMAND`/`PORTAGE_BUNZIP2_COMMAND`/`PORTAGE_XZ_COMMAND`/`PORTAGE_LZIP_COMMAND`/`PORTAGE_LZOP_COMMAND`/`PORTAGE_ZSTD_COMMAND`/`BINPKG_*`/`PORTAGE_BINHOST`/`PORTAGE_BINPKG_HEADER_OFFSET`/`PORTAGE_NICENESS`/`PORTAGE_IONICE_COMMAND`/`PORTAGE_ELOG_*`/`PORTAGE_GPG_*`/`PORTAGE_RSYNC_*`/`PORTAGE_RSYNC_RETRIES`/`PORTAGE_RSYNC_EXTRA_OPTS`/`PORTAGE_IONICE`/`PORTAGE_RO`/`PORTAGE_RO_DISTDIRS`-named assignments.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like a Portage config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("CHOST")
        || t.contains("MAKEOPTS")
        || t.contains("PORTDIR")
        || t.contains("USE=\"")
        || t.contains("GENTOO_MIRRORS")
        || t.contains("ACCEPT_KEYWORDS")
        || t.contains("PORTAGE_")
        || t.lines().any(|l| {
            let s = l.trim();
            s.contains('/')
                && !s.contains("://")
                && (s.contains("~amd64") || s.contains("~x86") || s.contains("atom"))
        })
}

impl Portage {
    /// Parse a Portage config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assigns: 0,
            atoms: 0,
            named: 0,
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
            if s.contains('=') {
                let key = s.split('=').next().unwrap_or("").trim();
                if !key.is_empty()
                    && key
                        .chars()
                        .all(|ch| ch.is_ascii_uppercase() || ch == '_' || ch.is_ascii_digit())
                {
                    c.assigns += 1;
                    if [
                        "USE",
                        "FEATURES",
                        "EMERGE_DEFAULT_OPTS",
                        "MAKEOPTS",
                        "CFLAGS",
                        "CXXFLAGS",
                        "FCFLAGS",
                        "FFLAGS",
                        "LDFLAGS",
                        "CHOST",
                        "PORTDIR",
                        "DISTDIR",
                        "PKGDIR",
                        "PORTAGE_TMPDIR",
                        "GENTOO_MIRRORS",
                        "SYNC",
                        "ACCEPT_KEYWORDS",
                        "ACCEPT_LICENSE",
                        "ACCEPT_PROPERTIES",
                        "ACCEPT_RESTRICT",
                        "LINGUAS",
                        "L10N",
                        "RUBY_TARGETS",
                        "PYTHON_TARGETS",
                        "PYTHON_SINGLE_TARGET",
                        "VIDEO_CARDS",
                        "INPUT_DEVICES",
                        "GRUB_PLATFORMS",
                        "CURL_SSL",
                        "FETCHCOMMAND",
                        "RESUMECOMMAND",
                        "RPMDIR",
                        "PKG_CONFIG_PATH",
                        "POLICY_TYPES",
                        "CLEAN_DELAY",
                        "EMERGE_WARNING_DELAY",
                        "TERM",
                        "NOCOLOR",
                        "COLORFGBG",
                        "BASH_ENV",
                        "ENV",
                        "EBEEP",
                        "EPAUSE",
                        "USE_ORDER",
                        "CONFIG_PROTECT",
                        "CONFIG_PROTECT_MASK",
                        "UNINSTALL_IGNORE",
                        "INSTALL_MASK",
                        "KERNEL",
                        "USERLAND",
                        "ELIBC",
                        "FFTOOLS",
                        "TWOLAME",
                        "OPENCL_TARGETS",
                        "LLVM_TARGETS",
                        "CPU_FLAGS_X86",
                        "CPU_FLAGS_ARM",
                        "CPU_FLAGS_PPC",
                        "DVB_CARDS",
                        "CAMERAS",
                        "SANE_BACKENDS",
                        "LIRC_DEVICES",
                        "ALSA_CARDS",
                        "APACHE2_MODULES",
                        "CALLIGRA_FEATURES",
                        "COLLECTD_PLUGINS",
                        "GPSD_PROTOCOLS",
                        "FRITZCAPI_CARDS",
                        "FOO2ZJS_DEVICES",
                        "MONKEYD_PLUGINS",
                        "OFFICE_IMPLEMENTATION",
                        "OPENMPI_FABRICS",
                        "OPENMPI_OFED_FEATURES",
                        "PHP_TARGETS",
                        "POSTGRES_TARGETS",
                        "QEMU_SOFTMMU_TARGETS",
                        "QEMU_USER_TARGETS",
                        "XFCE_PLUGINS",
                        "XTABLES_ADDONS",
                        "UWSGI_PLUGINS",
                        "VOICEMAIL_STORAGE",
                        "LUA_TARGETS",
                    ]
                    .contains(&key)
                        || key.starts_with("ACCEPT_")
                        || key.starts_with("PORTAGE_")
                        || key.starts_with("CPU_FLAGS_")
                        || key.starts_with("NGINX_MODULES_")
                        || key.starts_with("EXPAND_")
                        || key.starts_with("USE_EXPAND")
                        || key.starts_with("PROFILE_ONLY_")
                        || key.starts_with("GLSA_")
                        || key.ends_with("_TARGETS")
                        || key.ends_with("_MODULES")
                        || key.ends_with("_PLUGINS")
                        || key.ends_with("_FEATURES")
                        || key.ends_with("_CARDS")
                        || key.ends_with("_DEVICES")
                        || key.ends_with("_FLAGS")
                    {
                        c.named += 1;
                    }
                    continue;
                }
            }
            let first = s.split_whitespace().next().unwrap_or("");
            if first.contains('/')
                || first.starts_with('>')
                || first.starts_with('=')
                || first.starts_with('<')
                || first.starts_with('~')
                || first.starts_with('!')
            {
                c.atoms += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_make_conf() {
        let b = concat!(
            "# make.conf\n",
            "CHOST=\"x86_64-pc-linux-gnu\"\n",
            "CFLAGS=\"-O2 -pipe\"\n",
            "CXXFLAGS=\"${CFLAGS}\"\n",
            "MAKEOPTS=\"-j8\"\n",
            "PORTDIR=\"/var/db/repos/gentoo\"\n",
            "DISTDIR=\"/var/cache/distfiles\"\n",
            "PKGDIR=\"/var/cache/binpkgs\"\n",
            "USE=\"dbus wayland -kde -gnome X\"\n",
            "ACCEPT_KEYWORDS=\"amd64\"\n",
            "ACCEPT_LICENSE=\"*\"\n",
            "FEATURES=\"parallel-fetch\"\n",
            "EMERGE_DEFAULT_OPTS=\"--ask\"\n",
            "VIDEO_CARDS=\"intel\"\n",
            "INPUT_DEVICES=\"libinput\"\n",
            "LINGUAS=\"en ja\"\n",
            "GENTOO_MIRRORS=\"https://mirror.example/gentoo\"\n",
            "PORTAGE_ELOG_CLASSES=\"log warn error\"\n",
            "PYTHON_TARGETS=\"python3_12\"\n",
            "CONFIG_PROTECT=\"/etc\"\n",
        );
        let c = Portage::parse(b.as_bytes()).unwrap();
        assert_eq!(c.assigns, 19);
        assert_eq!(c.comments, 1);
        assert!(c.named >= 15);
    }

    #[test]
    fn parses_package_use() {
        let b = concat!(
            "# package.use\n",
            "app-editors/vim X python\n",
            ">=dev-libs/foo-1.0 ~amd64\n",
            "dev-lang/python:3.12 sqlite\n",
            "=sys-kernel/gentoo-sources-6.8* ~amd64\n",
            "~dev-util/bar-2.0 ~x86\n",
        );
        let c = Portage::parse(b.as_bytes()).unwrap();
        assert_eq!(c.atoms, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Portage::parse(b"foo = 1").is_none());
    }
}
