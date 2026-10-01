//! PMML (Predictive Model Markup Language) XML parser.
//!
//! Detects `<PMML>` documents and counts `<DataField>`/`<MiningField>`
//! schema entries, model elements, `<Segment>` partitions and
//! `<OutputField>`/`<Target>` outputs.
//!
//! ```
//! let b = concat!(
//!     "<PMML version=\"4\" xmlns=\"http://www.dmg.org/PMML-4_4\"><Header/>",
//!     "<DataDictionary><DataField name=\"x\" optype=\"continuous\"/>",
//!     "<DataField name=\"y\"/></DataDictionary>",
//!     "<MiningModel><MiningSchema><MiningField name=\"x\"/></MiningSchema>",
//!     "<Segmentation><Segment/></Segmentation></MiningModel></PMML>"
//! ).as_bytes();
//! assert!(izanagi_kit::pmml::detect(b));
//! let c = izanagi_kit::pmml::Pmml::parse(b).unwrap();
//! assert_eq!(c.data_fields, 2);
//! assert_eq!(c.models, 1);
//! ```

/// Parsed PMML document summary.
#[derive(Debug, Clone)]
pub struct Pmml {
    /// `<DataField>` entries.
    pub data_fields: usize,
    /// `<MiningField>` entries.
    pub mining_fields: usize,
    /// Model elements (`RegressionModel`, `TreeModel`, `NeuralNetwork`,
    /// `SupportVectorMachineModel`, `ClusteringModel`, `Scorecard`,
    /// `AssociationModel`, `NaiveBayesModel`, `GeneralRegressionModel`,
    /// `MiningModel`, `TimeSeriesModel`, `BaselineModel`,
    /// `SequenceModel`, `RuleSetModel`, `BayesianNetworkModel`).
    pub models: usize,
    /// `<Segment>`/`<Segmentation>` entries.
    pub segments: usize,
    /// `<OutputField>`/`<Target>`/`<Apply>` entries.
    pub outputs: usize,
    /// `<TransformationDictionary>`/`<LocalTransformations>` blocks.
    pub transformations: usize,
    /// `<Verification>` blocks.
    pub verifications: usize,
}

const MODELS: &[&str] = &[
    "<RegressionModel",
    "<TreeModel",
    "<NeuralNetwork",
    "<SupportVectorMachineModel",
    "<ClusteringModel",
    "<Scorecard",
    "<AssociationModel",
    "<NaiveBayesModel",
    "<GeneralRegressionModel",
    "<MiningModel",
    "<TimeSeriesModel",
    "<BaselineModel",
    "<SequenceModel",
    "<RuleSetModel",
    "<BayesianNetworkModel",
    "<AnomalyDetectionModel",
];

fn count_key(t: &str, key: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        n += 1;
        off += i + key.len();
    }
    n
}

/// Whether the buffer looks like a PMML document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<PMML") || (t.contains("<DataDictionary") && t.contains("<MiningSchema"))
}

impl Pmml {
    /// Parses a PMML document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let models = MODELS.iter().map(|k| count_key(t, k)).sum();
        Some(Self {
            data_fields: count_key(t, "<DataField"),
            mining_fields: count_key(t, "<MiningField"),
            models,
            segments: count_key(t, "<Segment ")
                + count_key(t, "<Segment>")
                + count_key(t, "<Segment/"),
            outputs: count_key(t, "<OutputField")
                + count_key(t, "<Target")
                + count_key(t, "<Apply"),
            transformations: count_key(t, "<TransformationDictionary")
                + count_key(t, "<LocalTransformations"),
            verifications: count_key(t, "<Verification"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "<PMML version=\"4\" xmlns=\"http://www.dmg.org/PMML-4_4\"><Header/>",
            "<DataDictionary><DataField name=\"x\" optype=\"continuous\"/>",
            "<DataField name=\"y\"/><DataField name=\"z\"/></DataDictionary>",
            "<MiningModel><MiningSchema>",
            "<MiningField name=\"x\"/><MiningField name=\"y\"/></MiningSchema>",
            "<Output><OutputField name=\"p\"/></Output>",
            "<Targets><Target field=\"p\"/></Targets>",
            "<Segmentation><Segment id=\"1\"/><Segment id=\"2\"/></Segmentation>",
            "<LocalTransformations/>",
            "<Verification/></MiningModel>",
            "<TreeModel/><RegressionModel/></PMML>"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Pmml::parse(b).unwrap();
        assert_eq!(c.data_fields, 3);
        assert_eq!(c.mining_fields, 2);
        assert_eq!(c.models, 3);
        assert_eq!(c.segments, 2);
        assert_eq!(c.outputs, 3);
        assert_eq!(c.transformations, 1);
        assert_eq!(c.verifications, 1);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(b"<root/>"));
        assert!(Pmml::parse(b"<x/>").is_none());
    }
}
