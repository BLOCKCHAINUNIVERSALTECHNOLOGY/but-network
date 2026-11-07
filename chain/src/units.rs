//! Units — bits aur BUT ke beech conversion (display ke liye).
//!
//! Sanjay ka FINAL faisla (tokenomics §1): sabse chhoti unit **"bits"**,
//! **1 BUT = 1,000 bits**. Chain par amount hamesha bits (u64) me rehta
//! hai — ye module sirf dikhane/parsing ke liye hai, chain ka hisaab
//! kabhi BUT (float) me nahi hota. Float use nahi hota — galti se bhi
//! rounding na ho, isliye sab integer + string se hota hai.
//!
//! Ye Rust module TS wallet ke `shared/but-tx.ts` (butToBits / bitsToBut /
//! formatBits) ka mirror hai — dono taraf wahi format, wahi 3 decimals.
//! Koi ek taraf badle to doosri taraf bhi badalna.

use crate::consensus::BITS_PER_BUT;
use crate::error::{ButError, Result};

/// Bits ko BUT me dikhao — jaise 1500 bits → "1.5", 1000 → "1",
/// 1 → "0.001", 0 → "0". Hamesha 3 decimal tak, peeche ke zero hatakar.
pub fn format_but(bits: u64) -> String {
    let whole = bits / BITS_PER_BUT;
    let frac = bits % BITS_PER_BUT;
    if frac == 0 {
        return whole.to_string();
    }
    let frac_str = format!("{frac:03}").trim_end_matches('0').to_string();
    format!("{whole}.{frac_str}")
}

/// Bits ko "1.5 BUT" jaise dikhao.
pub fn format_but_with_unit(bits: u64) -> String {
    format!("{} BUT", format_but(bits))
}

/// User ka likha BUT amount (jaise "12.5" ya "0.001") bits me badlo.
/// Sirf 3 decimal tak chalenge (1 bit = 0.001 BUT). Galat input par
/// Hindi error — kabhi silently galat number nahi banega.
pub fn parse_but(input: &str) -> Result<u64> {
    let s = input.trim();
    if s.is_empty() {
        return Err(ButError::InvalidTransaction(
            "Rakam likho.".to_string(),
        ));
    }
    let mut parts = s.split('.');
    let whole = parts.next().unwrap_or("");
    let frac = parts.next().unwrap_or("");
    if parts.next().is_some() || whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(ButError::InvalidTransaction(
            "Rakam sirf number me likho (jaise 12.5).".to_string(),
        ));
    }
    if frac.len() > 3 {
        return Err(ButError::InvalidTransaction(
            "Sirf 3 decimal tak likh sakte ho (1 bit = 0.001 BUT).".to_string(),
        ));
    }
    let mut frac_padded = frac.to_string();
    while frac_padded.len() < 3 {
        frac_padded.push('0');
    }
    let whole_bits = whole
        .parse::<u64>()
        .map_err(|_| ButError::InvalidTransaction("Rakam bahut badi hai.".to_string()))?
        .checked_mul(BITS_PER_BUT)
        .ok_or_else(|| ButError::InvalidTransaction("Rakam bahut badi hai.".to_string()))?;
    let frac_bits: u64 = if frac.is_empty() {
        0
    } else {
        frac_padded
            .parse()
            .map_err(|_| ButError::InvalidTransaction("Rakam samajh nahi aayi.".to_string()))?
    };
    let bits = whole_bits
        .checked_add(frac_bits)
        .ok_or_else(|| ButError::InvalidTransaction("Rakam bahut badi hai.".to_string()))?;
    if bits == 0 {
        return Err(ButError::InvalidTransaction(
            "Rakam 0 se zyada honi chahiye.".to_string(),
        ));
    }
    Ok(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_but_sahi_hai() {
        assert_eq!(format_but(0), "0");
        assert_eq!(format_but(1), "0.001");
        assert_eq!(format_but(999), "0.999");
        assert_eq!(format_but(1000), "1");
        assert_eq!(format_but(1500), "1.5");
        assert_eq!(format_but(1010), "1.01");
        assert_eq!(format_but(1_000_000), "1000");
        assert_eq!(format_but(1_234_500), "1234.5");
        assert_eq!(format_but_with_unit(1500), "1.5 BUT");
    }

    #[test]
    fn parse_but_sahi_hai() {
        assert_eq!(parse_but("1").unwrap(), 1000);
        assert_eq!(parse_but("1.5").unwrap(), 1500);
        assert_eq!(parse_but("0.001").unwrap(), 1);
        assert_eq!(parse_but("  12.25  ").unwrap(), 12250);
        assert_eq!(parse_but("1000").unwrap(), 1_000_000);
    }

    #[test]
    fn parse_but_galat_input_reject() {
        assert!(parse_but("").is_err());
        assert!(parse_but("abc").is_err());
        assert!(parse_but("1.0001").is_err()); // 4 decimal — 1 bit se chhota
        assert!(parse_but("0").is_err());
        assert!(parse_but("0.000").is_err());
        assert!(parse_but("1.2.3").is_err());
        assert!(parse_but("-5").is_err());
    }

    #[test]
    fn roundtrip_format_parse() {
        for bits in [0u64, 1, 999, 1000, 1500, 12345, 1_000_000] {
            if bits == 0 {
                continue;
            }
            assert_eq!(parse_but(&format_but(bits)).unwrap(), bits);
        }
    }
}
