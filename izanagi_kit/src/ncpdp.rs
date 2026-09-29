//! NCPDP SCRIPT (`*.ncpdp`/XML) message census.
//!
//! `<Message>` root with `<Header>` transport fields and `<Body>`
//! transaction payloads (`NewRx`, `RefillRequest`, `RxFill`, `Status`,
//! `CancelRx`, `RxHistoryResponse`, ...).
//!
//! ```
//! let s = br#"<Message version="010" release="006"><Header><To>ph</To><From>dr</From><MessageID>1</MessageID><SentTime>2020</SentTime></Header><Body><NewRx><Prescriber><Name>DR</Name></Prescriber><Patient><Name>PT</Name></Patient><MedicationPrescribed><DrugDescription>RX</DrugDescription></MedicationPrescribed></NewRx></Body></Message>"#;
//! assert!(izanagi_kit::ncpdp::detect(s));
//! let n = izanagi_kit::ncpdp::Ncpdp::parse(s).unwrap();
//! assert_eq!(n.messages, 1);
//! assert_eq!(n.new_rx, 1);
//! assert_eq!(n.headers, 1);
//! assert_eq!(n.prescribers, 1);
//! assert_eq!(n.patients, 1);
//! ```

/// Parsed census of an NCPDP SCRIPT message.
#[derive(Debug, Clone)]
pub struct Ncpdp {
    /// `<Message` elements.
    pub messages: usize,
    /// `<Header` elements.
    pub headers: usize,
    /// `<Body` elements.
    pub bodies: usize,
    /// `<NewRx` transactions.
    pub new_rx: usize,
    /// `<RefillRequest`/`<RefillResponse` transactions.
    pub refills: usize,
    /// `<RxFill` transactions.
    pub rx_fills: usize,
    /// `<Status`/`<Error` responses.
    pub statuses: usize,
    /// `<CancelRx`/`<CancelRxResponse` transactions.
    pub cancels: usize,
    /// `<RxHistory`/`<RxHistoryResponse` transactions.
    pub rx_history: usize,
    /// `<Prescriber` elements.
    pub prescribers: usize,
    /// `<Pharmacy` elements.
    pub pharmacies: usize,
    /// `<Patient` elements.
    pub patients: usize,
    /// `<Medication`/`<DrugDescription` elements.
    pub medications: usize,
    /// `<Sig`/`<SigText` elements.
    pub sigs: usize,
    /// `<Quantity` elements.
    pub quantities: usize,
    /// `<DaysSupply` elements.
    pub days_supply: usize,
    /// `<Refill` count fields.
    pub refills_allowed: usize,
    /// `<WrittenDate` elements.
    pub written_dates: usize,
    /// `<Substitution`/`<Substitutions` elements.
    pub substitutions: usize,
    /// `<Diagnosis`/`<PrimaryDiagnosis` elements.
    pub diagnoses: usize,
    /// `<PriorAuthorization` elements.
    pub prior_auth: usize,
    /// `<To>`/`<From>` transport fields.
    pub addresses: usize,
    /// `<MessageID` elements.
    pub message_ids: usize,
}

/// Reports whether `b` looks like an NCPDP SCRIPT message.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("<Message")
        && (t.contains("<NewRx")
            || t.contains("<RxFill")
            || t.contains("<RefillRequest")
            || t.contains("<Header>"))
}

fn count(t: &str, key: &str) -> usize {
    t.matches(key).count()
}

impl Ncpdp {
    /// Parses `b` as an NCPDP message, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Ncpdp {
            messages: count(t, "<Message "),
            headers: count(t, "<Header>") + count(t, "<Header "),
            bodies: count(t, "<Body>"),
            new_rx: count(t, "<NewRx"),
            refills: count(t, "<RefillRequest") + count(t, "<RefillResponse"),
            rx_fills: count(t, "<RxFill"),
            statuses: count(t, "<Status ") + count(t, "<Error "),
            cancels: count(t, "<CancelRx"),
            rx_history: count(t, "<RxHistory"),
            prescribers: count(t, "<Prescriber"),
            pharmacies: count(t, "<Pharmacy"),
            patients: count(t, "<Patient"),
            medications: count(t, "<Medication") + count(t, "<DrugDescription"),
            sigs: count(t, "<Sig>") + count(t, "<SigText"),
            quantities: count(t, "<Quantity"),
            days_supply: count(t, "<DaysSupply"),
            refills_allowed: count(t, "<Refills>") + count(t, "<NumberOfRefills"),
            written_dates: count(t, "<WrittenDate"),
            substitutions: count(t, "<Substitution"),
            diagnoses: count(t, "<Diagnosis"),
            prior_auth: count(t, "<PriorAuthorization"),
            addresses: count(t, "<To>") + count(t, "<From>"),
            message_ids: count(t, "<MessageID"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = br#"<Message version="010" release="006"><Header><To>ph</To><From>dr</From><MessageID>1</MessageID><SentTime>2020</SentTime></Header><Body><NewRx><Prescriber><Name>DR</Name></Prescriber><Patient><Name>PT</Name></Patient><MedicationPrescribed><DrugDescription>RX</DrugDescription></MedicationPrescribed></NewRx></Body></Message>"#;

    #[test]
    fn parses_ncpdp() {
        assert!(detect(S));
        let n = Ncpdp::parse(S).unwrap();
        assert_eq!(n.messages, 1);
        assert_eq!(n.new_rx, 1);
        assert_eq!(n.headers, 1);
        assert_eq!(n.prescribers, 1);
        assert_eq!(n.patients, 1);
    }

    #[test]
    fn rejects_non_ncpdp() {
        assert!(!detect(b"<xml/>"));
        assert!(Ncpdp::parse(b"{}").is_none());
    }
}
