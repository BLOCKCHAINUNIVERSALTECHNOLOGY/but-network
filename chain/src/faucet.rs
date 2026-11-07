//! Testnet faucet — free test coins, SIRF testnet par.
//!
//! Testnet par naye user ko coins chahiye hote hain taaki wo wallet,
//! BUT Pay, swap aur staking try kar sake — bina asli paisa lagaye.
//! Faucet wahi kaam karta hai: maangne par chhoti fixed raqam ke
//! **test coins** deta hai.
//!
//! SACH BATA RAHE HAIN:
//! - **Ye test coins hain — inki koi real value nahi hai.** Inhe becha,
//!   khareeda ya asli paisa mana nahi ja sakta. Mainnet par faucet
//!   KABHI nahi chalega — faucet sirf testnet mode me chalta hai
//!   (`but-node testnet` / `ApiState::load_or_create_testnet`).
//! - Faucet seedha balance nahi badalta (wo verify tod dega). Faucet ek
//!   wallet hai jo testnet genesis me fund hota hai; wo asli signed
//!   transaction bhejta hai, aur coins agle block mine hone par milte hain.
//! - Faucet wallet ki key testnet-only dev key hai (entropy `[7u8; 32]`,
//!   jaise dev validator `[9u8; 32]`) — iski key public code me hai,
//!   isliye ye wallet sirf testnet ke liye hai. Mainnet par aisi key
//!   kabhi nahi hogi.
//! - Rate limit: ek address din me sirf 1 baar; ek IP se din me zyada
//!   se zyada 10 alag addresses. Localhost dev me saare requests ek hi
//!   IP (127.0.0.1) se aate hain — isliye IP limit wahan spam rokne ke
//!   liye hai, per-person limit nahi. Asli testnet par node apne proxy
//!   ke peeche `X-Forwarded-For` se real IP lega (aur us header par
//!   sirf tab bharosa karo jab proxy tumhara ho — koi bhi client use
//!   jhooth bhi likh sakta hai).
//! - Claims `faucet.json` me save hote hain (chain file ke saath) —
//!   node restart hone par bhi rate limit reset nahi hoti.

use crate::error::{ButError, Result};
use crate::wallet::{validate_address, Wallet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Faucet wallet — testnet-only dev key (entropy `[7u8; 32]`).
/// Sirf testnet genesis me fund hota hai. Mainnet par kabhi use nahi hoga.
pub const FAUCET_ENTROPY: [u8; 32] = [7u8; 32];

/// Ek drip me kitne test coins milte hain — 1,000 BUT = 1,000,000 bits
/// (1 BUT = 1,000 bits). Configurable hai (`FaucetConfig.drip_amount_bits`),
/// par default yahi hai.
pub const DEFAULT_DRIP_BITS: u64 = 1_000_000;

/// Ek address dobara kab claim kar sakta hai — 24 ghante (86,400 seconds).
pub const DEFAULT_COOLDOWN_SECS: u64 = 86_400;

/// Ek IP se ek din me kitne alag addresses claim kar sakte hain.
pub const DEFAULT_MAX_PER_IP_PER_DAY: u32 = 10;

/// Testnet genesis me faucet wallet ko kitne test coins milte hain —
/// 100,000,000 BUT (100,000,000,000 bits). 1,000 BUT/drip se 100,000
/// drips. Ye sirf test coins hain — inki koi real value nahi.
pub const TESTNET_FAUCET_FUND_BITS: u64 = 100_000_000_000;

/// Faucet wallet banao — testnet-only dev key.
pub fn faucet_wallet() -> Wallet {
    Wallet::from_entropy(&FAUCET_ENTROPY).expect("faucet wallet hamesha banta hai")
}

/// Faucet ki settings — config se badli ja sakti hain, par default
/// safe hain. `enabled` false ho to faucet kuch nahi dega.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetConfig {
    /// Faucet chal raha hai ya nahi. Sirf testnet mode me true hota hai.
    pub enabled: bool,
    /// Ek drip me kitne bits milte hain (default 1,000,000 bits = 1,000 BUT).
    pub drip_amount_bits: u64,
    /// Ek address dobara kitne seconds baad claim kar sakta hai (default 86,400 = 1 din).
    pub cooldown_secs: u64,
    /// Ek IP se ek din me kitne alag addresses claim kar sakte hain (default 10).
    pub max_claims_per_ip_per_day: u32,
}

impl Default for FaucetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            drip_amount_bits: DEFAULT_DRIP_BITS,
            cooldown_secs: DEFAULT_COOLDOWN_SECS,
            max_claims_per_ip_per_day: DEFAULT_MAX_PER_IP_PER_DAY,
        }
    }
}

/// Faucet ki state — kaun kab claim kar chuka hai, kul kitna diya gaya.
/// Ye `faucet.json` me save hoti hai (chain file ke saath).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faucet {
    /// Faucet ki settings.
    pub config: FaucetConfig,
    /// Address → aakhri claim ka time (seconds since epoch).
    pub claims: HashMap<String, u64>,
    /// IP → us IP se claims ke times (seconds since epoch).
    /// Sirf pichle `cooldown_secs` ke andar ke times gine jate hain.
    pub ip_claims: HashMap<String, Vec<u64>>,
    /// Ab tak kul kitne test coins (bits) diye gaye.
    pub total_dripped_bits: u64,
    /// Ab tak kul kitne claims hue.
    pub total_claims: u64,
}

impl Faucet {
    /// Settings ke saath faucet banao.
    pub fn new(config: FaucetConfig) -> Self {
        Self {
            config,
            claims: HashMap::new(),
            ip_claims: HashMap::new(),
            total_dripped_bits: 0,
            total_claims: 0,
        }
    }

    /// Testnet faucet — enabled + defaults. Sirf testnet genesis par use hota hai.
    pub fn new_testnet() -> Self {
        Self::new(FaucetConfig {
            enabled: true,
            ..FaucetConfig::default()
        })
    }

    /// Band faucet — kuch nahi dega. Safe default (normal node par yahi hota hai).
    pub fn disabled() -> Self {
        Self::new(FaucetConfig::default())
    }

    /// Kya ye address + IP abhi claim kar sakta hai?
    /// Galat ho to saaf error — dobara kab kar sakta hai, wo bhi batata hai.
    pub fn can_claim(&self, address: &str, ip: &str, now: u64) -> Result<()> {
        if !self.config.enabled {
            return Err(ButError::InvalidTransaction(
                "faucet sirf testnet mode me chalta hai — mainnet par faucet kabhi nahi chalega. Testnet node chalao: `but-node testnet [port] [chain-file]`"
                    .to_string(),
            ));
        }
        validate_address(address)?;
        // Ek address din me sirf 1 baar.
        if let Some(last) = self.claims.get(address) {
            let next = last.saturating_add(self.config.cooldown_secs);
            if now < next {
                let wait = next.saturating_sub(now);
                return Err(ButError::InvalidTransaction(format!(
                    "is address ne aaj test coins le liye hain — dobara {wait} seconds baad (lagbhag {} ghante baad) claim kar sakte ho",
                    wait / 3600
                )));
            }
        }
        // Ek IP se ek din me zyada se zyada N alag addresses.
        let recent = self.ip_claims.get(ip).map(|times| {
            times
                .iter()
                .filter(|t| now.saturating_sub(**t) < self.config.cooldown_secs)
                .count() as u32
        });
        if let Some(count) = recent {
            if count >= self.config.max_claims_per_ip_per_day {
                return Err(ButError::InvalidTransaction(format!(
                    "is IP se aaj {} claims ho chuke hain (limit {}) — kal dobara try karo",
                    count, self.config.max_claims_per_ip_per_day
                )));
            }
        }
        Ok(())
    }

    /// Claim darj karo — `can_claim` ke baad hi bulana. Totals badhata hai
    /// aur IP ke purane (cooldown se bahar ke) times saaf karta hai.
    pub fn record_claim(&mut self, address: &str, ip: &str, now: u64, amount_bits: u64) {
        self.claims.insert(address.to_string(), now);
        let times = self.ip_claims.entry(ip.to_string()).or_default();
        times.retain(|t| now.saturating_sub(*t) < self.config.cooldown_secs);
        times.push(now);
        self.total_dripped_bits = self.total_dripped_bits.saturating_add(amount_bits);
        self.total_claims = self.total_claims.saturating_add(1);
    }

    /// Is address ko dobara claim karne me kitne seconds baki hain (0 = abhi kar sakta hai).
    pub fn seconds_until_next_claim(&self, address: &str, now: u64) -> u64 {
        match self.claims.get(address) {
            Some(last) => last
                .saturating_add(self.config.cooldown_secs)
                .saturating_sub(now),
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_but;

    fn testnet_faucet() -> Faucet {
        Faucet::new_testnet()
    }

    fn addr(n: u8) -> String {
        Wallet::from_entropy(&[n; 32]).unwrap().address
    }

    #[test]
    fn drip_amount_1000_but_hai() {
        // 1,000 BUT = 1,000,000 bits (1 BUT = 1,000 bits).
        assert_eq!(DEFAULT_DRIP_BITS, 1_000_000);
        assert_eq!(format_but(DEFAULT_DRIP_BITS), "1000");
    }

    #[test]
    fn pehli_baar_claim_milta_hai() {
        let mut f = testnet_faucet();
        let a = addr(2);
        assert!(f.can_claim(&a, "1.2.3.4", 1_700_000_000).is_ok());
        f.record_claim(&a, "1.2.3.4", 1_700_000_000, DEFAULT_DRIP_BITS);
        assert_eq!(f.total_claims, 1);
        assert_eq!(f.total_dripped_bits, DEFAULT_DRIP_BITS);
    }

    #[test]
    fn ek_hi_din_dobara_claim_reject() {
        let mut f = testnet_faucet();
        let a = addr(2);
        f.record_claim(&a, "1.2.3.4", 1_700_000_000, DEFAULT_DRIP_BITS);
        // 1 ghante baad dobara — reject, wait time milega.
        let err = f.can_claim(&a, "1.2.3.4", 1_700_003_600).unwrap_err();
        assert!(err.to_string().contains("dobara"), "error: {err}");
        assert!(f.seconds_until_next_claim(&a, 1_700_003_600) > 0);
        // 24 ghante + 1 second baad — phir se claim kar sakta hai.
        assert!(f
            .can_claim(&a, "1.2.3.4", 1_700_000_000 + DEFAULT_COOLDOWN_SECS + 1)
            .is_ok());
    }

    #[test]
    fn alag_address_claim_kar_sakta_hai() {
        let mut f = testnet_faucet();
        let a = addr(2);
        let b = addr(3);
        f.record_claim(&a, "1.2.3.4", 1_700_000_000, DEFAULT_DRIP_BITS);
        // Alag address, wahi din, wahi IP — allowed (IP limit 10 hai).
        assert!(f.can_claim(&b, "1.2.3.4", 1_700_000_100).is_ok());
    }

    #[test]
    fn galat_address_reject() {
        let f = testnet_faucet();
        assert!(f.can_claim("but1abc", "1.2.3.4", 1_700_000_000).is_err());
        assert!(f.can_claim("", "1.2.3.4", 1_700_000_000).is_err());
    }

    #[test]
    fn band_faucet_sab_reject() {
        let f = Faucet::disabled();
        assert!(!f.config.enabled);
        let err = f.can_claim(&addr(2), "1.2.3.4", 1_700_000_000).unwrap_err();
        assert!(
            err.to_string().contains("testnet"),
            "error me testnet likha hona chahiye: {err}"
        );
    }

    #[test]
    fn ek_ip_se_limit() {
        let mut f = testnet_faucet();
        let now = 1_700_000_000u64;
        // 10 alag addresses, ek hi IP — sab allowed.
        for n in 2u8..12 {
            let a = addr(n);
            assert!(f.can_claim(&a, "9.9.9.9", now).is_ok(), "address {n} fail");
            f.record_claim(&a, "9.9.9.9", now, DEFAULT_DRIP_BITS);
        }
        assert_eq!(f.total_claims, 10);
        // 11th address, wahi IP, wahi din — reject.
        let err = f.can_claim(&addr(20), "9.9.9.9", now).unwrap_err();
        assert!(err.to_string().contains("limit"), "error: {err}");
        // Alag IP se allowed.
        assert!(f.can_claim(&addr(20), "8.8.8.8", now).is_ok());
        // Agle din wahi IP phir se allowed.
        assert!(f
            .can_claim(&addr(20), "9.9.9.9", now + DEFAULT_COOLDOWN_SECS + 1)
            .is_ok());
    }
}
