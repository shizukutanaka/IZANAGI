//! OVF (Open Virtualization Format) XML.
//!
//! OVF envelopes carry `<Envelope>` with `<References>`/`<File>`,
//! `<DiskSection>`/`<NetworkSection>`/`<AnnotationSection>`/
//! `<ProductSection>`/`<EulaSection>` sections, `<VirtualSystem>`/
//! `<VirtualSystemCollection>` payloads, and `<Item>` hardware entries
//! (`ResourceType` RASD codes, `AllocationUnits`, `VirtualQuantity`).
//!
//! ```
//! let b = concat!(
//!     "<Envelope>\n",
//!     "  <References><File href=\"disk.vmdk\"/></References>\n",
//!     "  <DiskSection><Info>i</Info><Disk diskId=\"d\"/></DiskSection>\n",
//!     "  <VirtualSystem>\n",
//!     "    <VirtualHardwareSection>\n",
//!     "      <Item><ResourceType>3</ResourceType></Item>\n",
//!     "      <Item><ResourceType>4</ResourceType></Item>\n",
//!     "    </VirtualHardwareSection>\n",
//!     "  </VirtualSystem>\n",
//!     "</Envelope>\n"
//! ).as_bytes();
//! assert!(izanagi_kit::ovf::detect(b));
//! let c = izanagi_kit::ovf::Ovf::parse(b).unwrap();
//! assert_eq!(c.items, 2);
//! ```

/// Parsed OVF summary.
#[derive(Debug, Clone)]
pub struct Ovf {
    /// `<File>` references.
    pub files: usize,
    /// `<Disk>` entries.
    pub disks: usize,
    /// `<Network>` entries.
    pub networks: usize,
    /// `<VirtualSystem>`/`<VirtualSystemCollection>` entries.
    pub systems: usize,
    /// `<Item>` hardware entries.
    pub items: usize,
    /// `<Info>`/`<Annotation>`/`<Name>`/`<Description>` info elements.
    pub info: usize,
    /// `<Section>`-typed elements (`AnnotationSection`, `ProductSection`, `EulaSection`, `StartupSection`, `DeploymentOptionSection`, `OperatingSystemSection`, `InstallSection`).
    pub sections: usize,
    /// `<Property>` entries.
    pub properties: usize,
}

fn tag_count(t: &str, n: &str) -> usize {
    t.matches(&format!("<{n} ")).count()
        + t.matches(&format!("<{n}/")).count()
        + t.matches(&format!("<{n}>")).count()
}

/// Whether the buffer looks like OVF.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<Envelope")
        && (t.contains("<VirtualSystem") || t.contains("<DiskSection") || t.contains("<References"))
}

impl Ovf {
    /// Parses an OVF summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            files: tag_count(t, "File"),
            disks: tag_count(t, "Disk"),
            networks: tag_count(t, "Network"),
            systems: tag_count(t, "VirtualSystem") + tag_count(t, "VirtualSystemCollection"),
            items: tag_count(t, "Item"),
            info: tag_count(t, "Info")
                + tag_count(t, "Annotation")
                + tag_count(t, "Name")
                + tag_count(t, "Description"),
            sections: [
                "AnnotationSection",
                "ProductSection",
                "EulaSection",
                "StartupSection",
                "DeploymentOptionSection",
                "OperatingSystemSection",
                "InstallSection",
                "DiskSection",
                "NetworkSection",
                "References",
                "VirtualHardwareSection",
                "Property",
            ]
            .iter()
            .map(|k| tag_count(t, k))
            .sum(),
            properties: tag_count(t, "Property"),
        };
        c.sections -= c.properties;
        c.info += tag_count(t, "Label") + tag_count(t, "Category");
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ovf() {
        let b = concat!(
            "<Envelope>\n",
            "  <References><File href=\"disk.vmdk\"/><File href=\"iso.iso\"/></References>\n",
            "  <DiskSection><Info>i</Info><Disk diskId=\"d\" capacity=\"1\"/></DiskSection>\n",
            "  <NetworkSection><Network name=\"n\"/></NetworkSection>\n",
            "  <VirtualSystem>\n",
            "    <Name>vm</Name>\n",
            "    <VirtualHardwareSection>\n",
            "      <Item><ResourceType>3</ResourceType></Item>\n",
            "      <Item><ResourceType>4</ResourceType></Item>\n",
            "      <Item><ResourceType>10</ResourceType></Item>\n",
            "    </VirtualHardwareSection>\n",
            "    <ProductSection><Property key=\"k\"/></ProductSection>\n",
            "  </VirtualSystem>\n",
            "</Envelope>\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Ovf::parse(b).unwrap();
        assert_eq!(c.files, 2);
        assert_eq!(c.disks, 1);
        assert_eq!(c.networks, 1);
        assert_eq!(c.systems, 1);
        assert_eq!(c.items, 3);
        assert_eq!(c.properties, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"<html/>\n"));
        assert!(Ovf::parse(b"x").is_none());
    }
}
