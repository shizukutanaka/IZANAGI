//! Windows driver/setup `.inf` file census.
//!
//! `.inf` files are INI with `[Version]`
//! (`Signature="$CHICAGO$"`), `[Manufacturer]`,
//! `[SourceDisksFiles]`/`[SourceDisksNames]`,
//! `[DefaultInstall]`/`[DefaultInstall.Services]`/
//! `[DefaultUnInstall]`/`[DestinationDirs]`/`[Strings]`/
//! `[Service]`/`[HW]`/`[D3]`/`[DefaultInstall.NT]`/
//! `[Parameters]`/`[Addreg]`/`[AddService]`/`[CopyFiles]`/
//! `[DelFiles]`/`[RenFiles]`/`[AddInterface]`/`[Security]`/
//! `[IME]`/`[ClassInstall32]`/`[Uninstall]`/`[LnFiles]`/
//! `[UninstallSection]`/`[InfFile]` sections and
//! `DriverVer=`/`Class=`/`Provider=`/`CatalogFile=`/
//! `ProviderVersion=`/`DriverPackageType=`/`AddService=`/
//! `CopyFiles=`/`DelFiles=`/`RenFiles=`/`AddReg=`/`DelReg=`/
//! `HKR=`/`HKLM=`/`HKCU=`/`HKCR=`/`ServiceBinary=`/`ServiceType=`/
//! `StartType=`/`ErrorControl=`/`Reboot=`/`RegisterDlls=` directives.
//!
//! ```rust
//! let c = izanagi_kit::inffile::Inffile::parse(b"[Version]\nSignature=\"$CHICAGO$\"\nClass=Net\n[Strings]\nMfg=\"Acme\"\n").unwrap();
//! assert_eq!(c.entries, 3);
//! ```

/// `.inf` file census.
#[derive(Debug, Clone)]
pub struct Inffile {
    /// `[section]` headers.
    pub sections: usize,
    /// `key=value` entries.
    pub entries: usize,
    /// registry-path directives (`HKR`/`HKLM`/`HKCU`/`HKCR`).
    pub reg_dirs: usize,
    /// `;` comments.
    pub comments: usize,
}

const HEADS: &[&str] = &[
    "Version",
    "Manufacturer",
    "SourceDisksFiles",
    "SourceDisksNames",
    "DefaultInstall",
    "DefaultUnInstall",
    "DestinationDirs",
    "Strings",
    "Service",
    "HW",
    "Parameters",
    "Addreg",
    "AddService",
    "CopyFiles",
    "DelFiles",
    "RenFiles",
    "AddInterface",
    "Security",
    "ClassInstall32",
    "Uninstall",
    "LnFiles",
    "InfFile",
    "NTx86",
    "NTamd64",
    "NTARM64",
];

/// Whether the buffer looks like a `.inf` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let known_sections = HEADS
        .iter()
        .filter(|s| t.contains(&format!("[{s}]")))
        .count();
    t.contains("Signature=\"$CHICAGO$\"")
        || t.contains("Signature = \"$CHICAGO$\"")
        || (known_sections >= 2
            && (t.contains("DriverVer") || t.contains("CatalogFile") || t.contains("ClassGuid")))
        || t.contains("[SourceDisksFiles]")
        || t.contains("AddService=")
}

impl Inffile {
    /// Parse a `.inf` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            reg_dirs: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                continue;
            }
            let head = s.split(['=', ',']).next().unwrap_or("").trim();
            if head == "HKR" || head == "HKLM" || head == "HKCU" || head == "HKCR" {
                c.reg_dirs += 1;
            }
            if s.contains('=') {
                c.entries += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inf() {
        let b = concat!(
            "; Sample driver INF\n",
            "[Version]\n",
            "Signature=\"$CHICAGO$\"\n",
            "Class=Net\n",
            "ClassGUID={4d36e972-e325-11ce-bfc1-08002be10318}\n",
            "Provider=%Mfg%\n",
            "CatalogFile=netdrv.cat\n",
            "DriverVer=01/01/2024,1.0\n",
            "[Manufacturer]\n",
            "%Mfg%=Models,NTamd64\n",
            "[Models.NTamd64]\n",
            "%DeviceDesc%=Install,PCI\\VEN_0001&DEV_0001\n",
            "[DefaultInstall]\n",
            "CopyFiles=@netdrv.sys\n",
            "AddReg=Reg\n",
            "[Reg]\n",
            "HKR,,DriverDesc,,%DeviceDesc%\n",
            "HKLM,SYSTEM\\CurrentControlSet\\Services,Test,,1\n",
            "[Strings]\n",
            "Mfg=\"Acme\"\n",
            "DeviceDesc=\"Acme NIC\"\n",
        );
        let c = Inffile::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.entries, 12);
        assert_eq!(c.reg_dirs, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Inffile::parse(b"[foo]\nx=1\n").is_none());
    }
}
