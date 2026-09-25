//! Check-digit identifiers — Luhn, ISBN-10/13, EAN-13, UPC-A, IBAN,
//! VIN, and MRZ (ICAO 9303) check digits.
//!
//! The human-facing cousin of [`bech32`](crate::bech32) and
//! [`base58`](crate::base58): these are the checksum schemes printed on
//! cards, books, barcodes, bank accounts, vehicles, and passports.
//! Every validator is a total `&str -> bool`; separators (spaces,
//! hyphens) are stripped where the spec tolerates them.
//!
//! ```
//! assert!(izanagi_kit::checkcode::luhn("79927398713"));
//! assert!(izanagi_kit::checkcode::iban("GB29 NWBK 6016 1331 9268 19"));
//! assert!(izanagi_kit::checkcode::isbn10("0-306-40615-2"));
//! ```

fn digits(s: &str) -> Option<Vec<u32>> {
    s.bytes().map(|b| (b as char).to_digit(10)).collect()
}

fn strip_sep(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '\t'))
        .collect()
}

/// Luhn (ISO/IEC 7812-1): payment cards, IMEI, many national ids.
pub fn luhn(s: &str) -> bool {
    let Some(d) = digits(&strip_sep(s)) else {
        return false;
    };
    if d.len() < 2 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &v) in d.iter().rev().enumerate() {
        let mut x = v;
        if i % 2 == 1 {
            x *= 2;
            if x > 9 {
                x -= 9;
            }
        }
        sum += x;
    }
    sum % 10 == 0
}

/// The digit that makes `payload` pass [`luhn`].
pub fn luhn_check_digit(payload: &str) -> Option<u8> {
    for c in b'0'..=b'9' {
        if luhn(&format!("{payload}{}", c as char)) {
            return Some(c - b'0');
        }
    }
    None
}

/// ISBN-10: nine digits + a check character (digit or `X`).
pub fn isbn10(s: &str) -> bool {
    let t = strip_sep(s).to_uppercase();
    let b: Vec<char> = t.chars().collect();
    if b.len() != 10 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &c) in b.iter().enumerate() {
        let v = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'X' if i == 9 => 10,
            _ => return false,
        };
        sum += (10 - i as u32) * v;
    }
    sum % 11 == 0
}

/// GS1 check shared by ISBN-13, EAN-13, and UPC-A (weights 1,3,1,3...).
fn gs1(s: &str, len: usize) -> bool {
    let Some(d) = digits(&strip_sep(s)) else {
        return false;
    };
    if d.len() != len {
        return false;
    }
    let sum: u32 = d
        .iter()
        .enumerate()
        .map(|(i, &v)| if i % 2 == 0 { v } else { 3 * v })
        .sum();
    sum % 10 == 0
}

/// ISBN-13: GS1 check + a Bookland prefix (`978` or `979`).
pub fn isbn13(s: &str) -> bool {
    gs1(s, 13) && (strip_sep(s).starts_with("978") || strip_sep(s).starts_with("979"))
}

/// EAN-13 (JAN included): GS1 check, any prefix.
pub fn ean13(s: &str) -> bool {
    gs1(s, 12 + 1)
}

/// UPC-A: 12 digits, GS1 check.
pub fn upca(s: &str) -> bool {
    gs1(s, 11 + 1)
}

/// UPC-A to EAN-13 (prefix `0`), recomputing nothing — the check digit
/// is shared by construction.
pub fn upca_to_ean13(s: &str) -> Option<String> {
    if !upca(s) {
        return None;
    }
    Some(format!("0{}", strip_sep(s)))
}

/// Convert a valid ISBN-10 to its `978`-Bookland ISBN-13.
pub fn isbn10_to_13(s: &str) -> Option<String> {
    if !isbn10(s) {
        return None;
    }
    let t = strip_sep(s).to_uppercase();
    let mut body: String = format!("978{}", &t[..9]);
    let check = (0..10u32)
        .find(|c| gs1(&format!("{body}{c}"), 13))
        .unwrap_or(0);
    body.push(char::from_digit(check, 10).unwrap_or('0'));
    Some(body)
}

/// ISO 13616 IBAN: `CC` + two check digits + BBAN; mod-97 == 1.
/// Length is verified against the official registry.
pub fn iban(s: &str) -> bool {
    const LEN: &[(&str, usize)] = &[
        ("AD", 24),
        ("AE", 23),
        ("AL", 28),
        ("AT", 20),
        ("AZ", 28),
        ("BA", 20),
        ("BE", 16),
        ("BG", 22),
        ("BH", 22),
        ("BR", 29),
        ("CH", 21),
        ("CR", 22),
        ("CY", 28),
        ("CZ", 24),
        ("DE", 22),
        ("DK", 18),
        ("DO", 28),
        ("EE", 20),
        ("ES", 24),
        ("FI", 18),
        ("FO", 18),
        ("FR", 27),
        ("GB", 22),
        ("GE", 22),
        ("GI", 23),
        ("GL", 18),
        ("GR", 27),
        ("GT", 28),
        ("HR", 21),
        ("HU", 28),
        ("IE", 22),
        ("IL", 23),
        ("IQ", 23),
        ("IS", 26),
        ("IT", 27),
        ("JO", 30),
        ("KW", 30),
        ("KZ", 20),
        ("LB", 28),
        ("LC", 32),
        ("LI", 21),
        ("LT", 20),
        ("LU", 20),
        ("LV", 21),
        ("MC", 27),
        ("MD", 24),
        ("ME", 22),
        ("MK", 19),
        ("MR", 27),
        ("MT", 31),
        ("MU", 30),
        ("NL", 18),
        ("NO", 15),
        ("PK", 24),
        ("PL", 28),
        ("PS", 29),
        ("PT", 25),
        ("QA", 29),
        ("RO", 24),
        ("RS", 22),
        ("SA", 24),
        ("SC", 31),
        ("SE", 24),
        ("SI", 19),
        ("SK", 31),
        ("SM", 27),
        ("ST", 25),
        ("SV", 28),
        ("TL", 23),
        ("TN", 24),
        ("TR", 26),
        ("UA", 29),
        ("VA", 22),
        ("VG", 24),
        ("XK", 20),
    ];
    let t = strip_sep(s).to_uppercase();
    if t.len() < 15 || t.len() > 34 {
        return false;
    }
    let cc = &t[..2];
    if !cc.bytes().all(|b| b.is_ascii_uppercase()) {
        return false;
    }
    if !t[2..4].bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if !t.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    match LEN.iter().find(|(c, _)| *c == cc) {
        Some((_, n)) if *n != t.len() => return false,
        None => return false,
        _ => {}
    }
    // Rearrange: move CC+check to the end, expand letters, mod 97.
    let mut rem = 0u32;
    for b in t[4..].bytes().chain(t[..4].bytes()) {
        if b.is_ascii_digit() {
            rem = (rem * 10 + u32::from(b - b'0')) % 97;
        } else {
            let v = u32::from(b - b'A') + 10;
            rem = (rem * 100 + v) % 97;
        }
    }
    rem == 1
}

/// US-market VIN (ISO 3779): 17 chars, no I/O/Q, position-9 check.
pub fn vin(s: &str) -> bool {
    const W: [u32; 17] = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];
    let t = s.to_uppercase();
    let b: Vec<u8> = t.bytes().collect();
    if b.len() != 17 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &c) in b.iter().enumerate() {
        let v = match c {
            b'0'..=b'9' => u32::from(c - b'0'),
            b'A'..=b'Z' => {
                if matches!(c, b'I' | b'O' | b'Q') {
                    return false;
                }
                // Transliteration: A=1..H=8, J=1..R=9, S=2..Z=9.
                match c {
                    b'A'..=b'H' => u32::from(c - b'A') + 1,
                    b'J'..=b'R' => u32::from(c - b'J') + 1,
                    _ => u32::from(c - b'S') + 2,
                }
            }
            _ => return false,
        };
        sum += v * W[i];
    }
    let want = sum % 11;
    let check = b[8];
    (want == 10 && check == b'X') || (check.is_ascii_digit() && u32::from(check - b'0') == want)
}

/// ICAO 9303 MRZ check digit over a field (`0-9A-Z<`, weights 7-3-1).
/// Returns the digit to append, or `None` on bad characters.
pub fn mrz_check_digit(field: &str) -> Option<u8> {
    let mut sum = 0u32;
    for (i, b) in field.bytes().enumerate() {
        let v = match b {
            b'0'..=b'9' => u32::from(b - b'0'),
            b'A'..=b'Z' => u32::from(b - b'A') + 10,
            b'<' => 0,
            _ => return None,
        };
        sum += v * [7u32, 3, 1][i % 3];
    }
    Some((sum % 10) as u8)
}

/// Verify a `field+check` pair against [`mrz_check_digit`].
pub fn mrz_valid(field_with_check: &str) -> bool {
    let (field, check) = field_with_check.split_at(field_with_check.len().saturating_sub(1));
    match (mrz_check_digit(field), check.bytes().next()) {
        (Some(c), Some(b)) if b.is_ascii_digit() => c == b - b'0',
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_vectors() {
        assert!(luhn("79927398713"));
        assert!(luhn("4242 4242 4242 4242")); // classic test card
        assert!(!luhn("79927398714"));
        assert!(!luhn("79927a98713"));
        assert_eq!(luhn_check_digit("7992739871"), Some(3));
    }

    #[test]
    fn isbn_and_ean() {
        assert!(isbn10("0-306-40615-2"));
        assert!(isbn10("0306406152"));
        assert!(!isbn10("0-306-40615-3"));
        assert!(isbn13("978-0-306-40615-7"));
        assert!(!isbn13("968-0-306-40615-0")); // wrong prefix
        assert_eq!(
            isbn10_to_13("0-306-40615-2").as_deref(),
            Some("9780306406157")
        );
        assert!(ean13("4006381333931")); // EAN test vector
        assert!(upca("012345678905"));
        assert_eq!(
            upca_to_ean13("012345678905").as_deref(),
            Some("0012345678905")
        );
    }

    #[test]
    fn iban_vectors() {
        for good in [
            "GB29 NWBK 6016 1331 9268 19",
            "DE89370400440532013000",
            "FR14 2004 1010 0505 0001 3M02 606",
            "NL91ABNA0417164300",
            "ES9121000418450200051332",
        ] {
            assert!(iban(good), "{good}");
        }
        for bad in [
            "GB29 NWBK 6016 1331 9268 20", // bad check
            "DE8937040044053201300",       // wrong length
            "XX29NWBK60161331926819",      // unknown country
            "GB29NWBK6016133192681!",      // bad char
            "GB29NWBK60161331926819X",     // too long
        ] {
            assert!(!iban(bad), "{bad}");
        }
    }

    #[test]
    fn vin_and_mrz() {
        assert!(vin("1M8GDM9AXKP042788")); // Wikipedia's canonical example
        assert!(!vin("1M8GDM9AXKP042789"));
        assert!(!vin("1M8GDM9AIKP042788")); // contains I
        assert!(!vin("1M8GDM9AXKP04278"));
        // MRZ: "D23145890" + check '7' (ICAO example field).
        assert_eq!(mrz_check_digit("D23145890"), Some(7));
        assert!(mrz_valid("D231458907"));
        assert!(!mrz_valid("D231458908"));
        assert!(mrz_valid("520727<<<<<3"));
    }
}
