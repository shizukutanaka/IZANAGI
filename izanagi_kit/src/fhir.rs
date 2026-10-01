//! HL7 FHIR resource (`*.fhir`/JSON) census.
//!
//! `"resourceType"` root plus common members: `id`/`meta`/`identifier`/
//! `name`/`telecom`/`address`/`coding`/`system`/`code`/`reference`/
//! `status`/`category`/`valueQuantity`/`entry` (Bundle).
//!
//! ```
//! let s = br#"{"resourceType":"Patient","id":"p1","meta":{"versionId":"1"},"identifier":[{"system":"sys","value":"v"}],"name":[{"family":"DOE"}],"gender":"male","active":true}"#;
//! assert!(izanagi_kit::fhir::detect(s));
//! let f = izanagi_kit::fhir::Fhir::parse(s).unwrap();
//! assert_eq!(f.resource_types, 1);
//! assert_eq!(f.patients, 1);
//! assert_eq!(f.identifiers, 1);
//! assert_eq!(f.names, 1);
//! ```

/// Parsed census of a FHIR JSON resource.
#[derive(Debug, Clone)]
pub struct Fhir {
    /// `"resourceType"` keys.
    pub resource_types: usize,
    /// Patient resources.
    pub patients: usize,
    /// Observation resources.
    pub observations: usize,
    /// Bundle resources.
    pub bundles: usize,
    /// Encounter resources.
    pub encounters: usize,
    /// MedicationRequest resources.
    pub medication_requests: usize,
    /// `"id"` keys.
    pub ids: usize,
    /// `"meta"` keys.
    pub metas: usize,
    /// `"identifier"` keys.
    pub identifiers: usize,
    /// `"name"` keys.
    pub names: usize,
    /// `"telecom"` keys.
    pub telecoms: usize,
    /// `"address"` keys.
    pub addresses: usize,
    /// `"gender"` keys.
    pub genders: usize,
    /// `"birthDate"` keys.
    pub birth_dates: usize,
    /// `"coding"` keys.
    pub codings: usize,
    /// `"system"` keys.
    pub systems: usize,
    /// `"code"` keys.
    pub codes: usize,
    /// `"display"` keys.
    pub displays: usize,
    /// `"reference"` keys.
    pub references: usize,
    /// `"status"` keys.
    pub statuses: usize,
    /// `"category"` keys.
    pub categories: usize,
    /// `"effectiveDateTime"`/`"issued"`/`"performed"` keys.
    pub times: usize,
    /// `"valueQuantity"`/`"valueCodeableConcept"` keys.
    pub values: usize,
    /// `"unit"` keys.
    pub units: usize,
    /// `"entry"` keys.
    pub entries: usize,
    /// `"narrative"`/`"text"` keys.
    pub narratives: usize,
    /// `"extension"` keys.
    pub extensions: usize,
    /// `"url"` keys.
    pub urls: usize,
    /// `"subject"` keys.
    pub subjects: usize,
}

const RES_TYPES: &[&str] = &[
    "Patient",
    "Observation",
    "Bundle",
    "Encounter",
    "MedicationRequest",
    "Condition",
    "Procedure",
    "AllergyIntolerance",
    "Immunization",
    "DiagnosticReport",
    "Organization",
    "Practitioner",
    "Medication",
    "CarePlan",
    "ServiceRequest",
    "Coverage",
    "Claim",
    "ExplanationOfBenefit",
    "DocumentReference",
    "Composition",
    "Questionnaire",
    "QuestionnaireResponse",
    "Specimen",
    "Appointment",
    "Schedule",
    "Slot",
    "Location",
    "Device",
    "ImagingStudy",
    "Media",
    "Flag",
    "Consent",
    "Provenance",
    "AuditEvent",
    "OperationOutcome",
    "CapabilityStatement",
];

/// Reports whether `b` looks like a FHIR resource.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"resourceType\"")
        && RES_TYPES.iter().any(|r| {
            t.contains(&format!("\"resourceType\":\"{r}\""))
                || t.contains(&format!("\"resourceType\": \"{r}\""))
        })
}

fn count(t: &str, key: &str) -> usize {
    t.matches(key).count()
}

impl Fhir {
    /// Parses `b` as a FHIR document, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Fhir {
            resource_types: count(t, "\"resourceType\""),
            patients: count(t, "\"Patient\""),
            observations: count(t, "\"Observation\""),
            bundles: count(t, "\"Bundle\""),
            encounters: count(t, "\"Encounter\""),
            medication_requests: count(t, "\"MedicationRequest\""),
            ids: count(t, "\"id\""),
            metas: count(t, "\"meta\""),
            identifiers: count(t, "\"identifier\""),
            names: count(t, "\"name\""),
            telecoms: count(t, "\"telecom\""),
            addresses: count(t, "\"address\""),
            genders: count(t, "\"gender\""),
            birth_dates: count(t, "\"birthDate\""),
            codings: count(t, "\"coding\""),
            systems: count(t, "\"system\""),
            codes: count(t, "\"code\""),
            displays: count(t, "\"display\""),
            references: count(t, "\"reference\""),
            statuses: count(t, "\"status\""),
            categories: count(t, "\"category\""),
            times: count(t, "\"effectiveDateTime\"")
                + count(t, "\"issued\"")
                + count(t, "\"performed\""),
            values: count(t, "\"valueQuantity\"") + count(t, "\"valueCodeableConcept\""),
            units: count(t, "\"unit\""),
            entries: count(t, "\"entry\""),
            narratives: count(t, "\"narrative\"") + count(t, "\"text\""),
            extensions: count(t, "\"extension\""),
            urls: count(t, "\"url\""),
            subjects: count(t, "\"subject\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = br#"{"resourceType":"Patient","id":"p1","meta":{"versionId":"1"},"identifier":[{"system":"sys","value":"v"}],"name":[{"family":"DOE"}],"gender":"male","active":true}"#;

    #[test]
    fn parses_fhir() {
        assert!(detect(S));
        let f = Fhir::parse(S).unwrap();
        assert_eq!(f.resource_types, 1);
        assert_eq!(f.patients, 1);
        assert_eq!(f.identifiers, 1);
        assert_eq!(f.names, 1);
    }

    #[test]
    fn rejects_non_fhir() {
        assert!(!detect(b"{}"));
        assert!(Fhir::parse(b"[]").is_none());
    }
}
