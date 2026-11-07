//! Stake-locking — Phase 3a: staked coins ab chain par LOCK hote hain.
//!
//! Phase 2 tak slashing/stake sirf `ValidatorSet` me bookkeeping tha —
//! staked coins abhi bhi kharch kiye ja sakte the. Ab nahi:
//! - **Stake karte hi coins lock ho jate hain** — wo `balances` (spendable)
//!   se nikal kar `StakingState.locks` me chale jate hain. Transaction ka
//!   balance check spendable balance par hota hai, isliye locked coins
//!   kharch hi nahi ho sakte.
//! - **Lock tiers — FINAL (Sanjay, tokenomics §6):** 90 din → 180 din →
//!   1 saal → 2 saal → 3 saal → 5 saal. Lock khatm hone se pehle unstake
//!   nahi hoga — tier ka matlab hi hai "itne din ke liye band".
//! - **Unstake par release 72 ghante me, dheere-dheere** — ek saath nahi.
//!   Release linear hoti hai: 72 ghante me har guzarte pal ke hisaab se.
//!   `claim` karne par us waqt tak ka due hissa spendable balance me
//!   wapas aata hai.
//! - **Ek staker ka ek hi lock hota hai** — dobara stake karne par amount
//!   usi lock me jud jata hai (tier aur `locked_at` pehle wale hi rehte
//!   hain — naya lock shuru nahi hota, warna purana lock kabhi khatm na hota).
//!
//! **Ambiguity note (tokenomics §6 me flagged):** tiers lock hain ya
//! unbonding — is code me dono ko alag rakha gaya hai: **tier = lock
//! period** (stake karne ke baad kitne din coins band rahenge), aur
//! **unbonding = 72 ghante gradual release** (unstake ke baad paise
//! wapas aane me lagne wala time). Agar Sanjay ka matlab kuch aur tha
//! to ye badlega.
//!
//! **StakeOp — block me jaane wala signed operation:** staking ke faisle
//! block me darj hote hain (taaki chain verify karte waqt staking state
//! bhi rebuild ho sake — sirf stored state par bharosa nahi). StakeOp
//! transaction jaisa hi signed hota hai (Ed25519, nonce ke saath) —
//! bina sahi signature ke reject, replay bhi nonce se rokta hai.
//! StakeOp aur transfer transaction ek hi nonce sequence share karte
//! hain (har address ka ek hi nonce) — isliye ek hi nonce se do kaam
//! nahi ho sakte.

use crate::error::{ButError, Result};
use crate::hash::sha256_hex;
use crate::wallet::validate_address;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unbonding release kitne time me poori hoti hai — 72 ghante (FINAL, §6).
/// Release linear hai: har guzarte second ke hisaab se hissa milta hai.
pub const UNBONDING_RELEASE_SECS: u64 = 72 * 3600;

/// Ek din, seconds me.
pub const DAY_SECS: u64 = 24 * 3600;

/// Staking lock tier — FINAL (Sanjay, tokenomics §6).
/// Tier = lock period: stake karne ke baad itne din coins band rahenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockTier {
    /// 90 din lock.
    Days90,
    /// 180 din lock.
    Days180,
    /// 1 saal lock.
    Year1,
    /// 2 saal lock.
    Year2,
    /// 3 saal lock.
    Year3,
    /// 5 saal lock.
    Year5,
}

impl LockTier {
    /// Tier ka lock period, seconds me.
    pub fn lock_seconds(&self) -> u64 {
        match self {
            LockTier::Days90 => 90 * DAY_SECS,
            LockTier::Days180 => 180 * DAY_SECS,
            LockTier::Year1 => 365 * DAY_SECS,
            LockTier::Year2 => 2 * 365 * DAY_SECS,
            LockTier::Year3 => 3 * 365 * DAY_SECS,
            LockTier::Year5 => 5 * 365 * DAY_SECS,
        }
    }

    /// Tier ka naam, display ke liye.
    pub fn label(&self) -> &'static str {
        match self {
            LockTier::Days90 => "90 din",
            LockTier::Days180 => "180 din",
            LockTier::Year1 => "1 saal",
            LockTier::Year2 => "2 saal",
            LockTier::Year3 => "3 saal",
            LockTier::Year5 => "5 saal",
        }
    }

    /// Dinon se tier banao — sirf ye 6 tiers allowed hain.
    pub fn from_days(days: u64) -> Result<Self> {
        match days {
            90 => Ok(LockTier::Days90),
            180 => Ok(LockTier::Days180),
            365 => Ok(LockTier::Year1),
            730 => Ok(LockTier::Year2),
            1095 => Ok(LockTier::Year3),
            1825 => Ok(LockTier::Year5),
            _ => Err(ButError::InvalidTransaction(format!(
                "lock tier sirf ye ho sakte hain: 90, 180, 365, 730, 1095, 1825 din — mila {days}"
            ))),
        }
    }
}

/// Ek staker ka lock — kitne coins, kis tier par, kab se band hain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakeLock {
    /// Staker ka address.
    pub staker: String,
    /// Locked amount (bits).
    pub amount: u64,
    /// Lock tier (kitne din ke liye band).
    pub tier: LockTier,
    /// Kab lock hua (block timestamp, seconds since epoch).
    pub locked_at: u64,
}

impl StakeLock {
    /// Lock kab khatm hoga (timestamp).
    pub fn unlock_at(&self) -> u64 {
        self.locked_at.saturating_add(self.tier.lock_seconds())
    }

    /// Kya is waqt (`now`) lock khatm ho chuka hai — unstake kar sakte hain?
    pub fn is_unlocked_at(&self, now: u64) -> bool {
        now >= self.unlock_at()
    }
}

/// Unbonding entry — unstake ke baad coins yahan aate hain aur 72 ghante
/// me dheere-dheere release hote hain. Ek staker ki ek se zyada entries
/// ho sakti hain (alag-alag waqt par unstake kiya ho to).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unbonding {
    /// Kiske coins hain.
    pub staker: String,
    /// Kul kitne coins unbonding me gaye.
    pub total_amount: u64,
    /// Ab tak kitne release ho chuke (claim ho chuke).
    pub released_amount: u64,
    /// Unbonding kab shuru hui (block timestamp).
    pub started_at: u64,
}

impl Unbonding {
    /// Is waqt (`now`) tak kul kitne release hone chahiye (linear, 72h me).
    pub fn due_amount(&self, now: u64) -> u64 {
        let elapsed = now.saturating_sub(self.started_at);
        if elapsed >= UNBONDING_RELEASE_SECS {
            return self.total_amount;
        }
        // Linear: total * elapsed / 72h. Overflow se bachne ke liye u128.
        ((self.total_amount as u128 * elapsed as u128) / UNBONDING_RELEASE_SECS as u128) as u64
    }

    /// Abhi claim karne layak kitne hain (due - pehle se released).
    pub fn claimable(&self, now: u64) -> u64 {
        self.due_amount(now).saturating_sub(self.released_amount)
    }

    /// Claim karo — due hissa nikalo, entry update karo.
    /// Returns: kitne coins claim hue.
    pub fn claim(&mut self, now: u64) -> u64 {
        let amount = self.claimable(now);
        self.released_amount += amount;
        amount
    }

    /// Kya poori entry release ho chuki hai (delete kar sakte hain)?
    pub fn is_fully_released(&self) -> bool {
        self.released_amount >= self.total_amount
    }
}

/// Poori staking state — chain me save hoti hai, verify ke waqt rebuild hoti hai.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StakingState {
    /// Har staker ka lock (ek staker = ek lock).
    #[serde(default)]
    pub locks: HashMap<String, StakeLock>,
    /// Chalti hui unbonding entries.
    #[serde(default)]
    pub unbonding: Vec<Unbonding>,
}

impl StakingState {
    /// Khaali staking state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Kisi address ke locked coins (0 ho to lock nahi hai).
    pub fn locked_of(&self, address: &str) -> u64 {
        self.locks.get(address).map(|l| l.amount).unwrap_or(0)
    }

    /// Kisi address ke unbonding me pade coins (abhi release nahi hue).
    pub fn unbonding_of(&self, address: &str) -> u64 {
        self.unbonding
            .iter()
            .filter(|u| u.staker == address)
            .map(|u| u.total_amount - u.released_amount)
            .sum()
    }

    /// Kisi address ke abhi claim karne layak coins (72h schedule ke hisaab se).
    pub fn claimable_of(&self, address: &str, now: u64) -> u64 {
        self.unbonding
            .iter()
            .filter(|u| u.staker == address)
            .map(|u| u.claimable(now))
            .sum()
    }

    /// Stake karo — coins lock karo. Caller ne pehle se check kiya hoga ki
    /// staker ke paas spendable balance kaafi hai (Chain karta hai).
    /// Pehle se lock ho to amount jud jata hai (tier/locked_at pehle wale).
    pub fn stake(&mut self, staker: &str, amount: u64, tier: LockTier, now: u64) -> Result<()> {
        validate_address(staker)?;
        if amount == 0 {
            return Err(ButError::InvalidTransaction(
                "stake amount 0 se zyada hona chahiye".to_string(),
            ));
        }
        match self.locks.get_mut(staker) {
            Some(lock) => {
                lock.amount = lock
                    .amount
                    .checked_add(amount)
                    .ok_or_else(|| ButError::InvalidTransaction("stake amount overflow".to_string()))?;
            }
            None => {
                self.locks.insert(
                    staker.to_string(),
                    StakeLock {
                        staker: staker.to_string(),
                        amount,
                        tier,
                        locked_at: now,
                    },
                );
            }
        }
        Ok(())
    }

    /// Unstake shuru karo — lock se coins nikaal kar unbonding me dalo.
    /// Lock khatm hone se pehle unstake nahi hoga. Poora lock ya uska hissa
    /// unstake kar sakte ho (amount <= locked).
    pub fn start_unbonding(&mut self, staker: &str, amount: u64, now: u64) -> Result<()> {
        validate_address(staker)?;
        if amount == 0 {
            return Err(ButError::InvalidTransaction(
                "unstake amount 0 se zyada hona chahiye".to_string(),
            ));
        }
        let lock = self.locks.get_mut(staker).ok_or_else(|| {
            ButError::InvalidTransaction(format!("{staker} ka koi stake lock nahi hai"))
        })?;
        if !lock.is_unlocked_at(now) {
            return Err(ButError::InvalidTransaction(format!(
                "lock abhi khatm nahi hua — {} ke baad hi unstake kar sakte ho (tier: {}, unlock at: {})",
                lock.unlock_at(),
                lock.tier.label(),
                lock.unlock_at()
            )));
        }
        if amount > lock.amount {
            return Err(ButError::InvalidTransaction(format!(
                "unstake amount locked se zyada hai: locked {}, manga {amount}",
                lock.amount
            )));
        }
        lock.amount -= amount;
        let staker_owned = lock.staker.clone();
        if lock.amount == 0 {
            self.locks.remove(staker);
        }
        self.unbonding.push(Unbonding {
            staker: staker_owned,
            total_amount: amount,
            released_amount: 0,
            started_at: now,
        });
        Ok(())
    }

    /// Claim karo — due hissa nikalo. Returns: kitne coins claim hue.
    /// Poori release hui entries delete ho jati hain.
    pub fn claim(&mut self, staker: &str, now: u64) -> u64 {
        let mut total = 0u64;
        for entry in self.unbonding.iter_mut().filter(|u| u.staker == staker) {
            total += entry.claim(now);
        }
        self.unbonding
            .retain(|u| !(u.staker == staker && u.is_fully_released()));
        total
    }

    /// Slashing — lock se coins kato (ValidatorSet::slash ke saath chalta hai).
    /// Lock me kam coins hon to jitne hain utne kat te hain.
    /// Returns: kitne coins kate.
    pub fn slash_lock(&mut self, staker: &str, cut: u64) -> u64 {
        match self.locks.get_mut(staker) {
            Some(lock) => {
                let actual = cut.min(lock.amount);
                lock.amount -= actual;
                if lock.amount == 0 {
                    self.locks.remove(staker);
                }
                actual
            }
            None => 0,
        }
    }
}

/// StakeOp ka type — block me kya karna hai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StakeOpKind {
    /// Coins lock karo (stake).
    Stake,
    /// Lock se coins nikaal kar unbonding me dalo (unstake).
    Unstake,
    /// Unbonding se due hissa claim karo (spendable me wapas).
    Claim,
    /// Locked stake ke hisaab se validator register karo. Lock
    /// minimum stake (`min_stake_for`) ya zyada hona chahiye.
    /// Registration block me darj hoti hai — taaki chain verify karte
    /// waqt validator set bhi rebuild ho sake.
    Register,
}

/// Block me jaane wala signed staking operation.
///
/// Transaction jaisa hi signed hota hai — Ed25519 signature + nonce.
/// Payload: `BUT-STAKE|<kind>|<staker>|<amount>|<tier>|<nonce>`.
/// - Stake: amount > 0, tier zaroori.
/// - Unstake: amount > 0, tier nahi.
/// - Claim: amount 0, tier nahi — due hissa claim hota hai.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakeOp {
    /// Stake / Unstake / Claim.
    pub kind: StakeOpKind,
    /// Staker ka address ("but1...", 56 characters).
    pub staker: String,
    /// Kitne bits (Stake/Unstake ke liye; Claim ke liye 0).
    pub amount: u64,
    /// Lock tier — sirf Stake ke liye.
    #[serde(default)]
    pub tier: Option<LockTier>,
    /// Staker ka nonce — transfer transaction ke saath ek hi sequence.
    /// Pehli op ka nonce bhi 0 se shuru hota hai (chain ke nonce jaisa).
    pub nonce: u64,
    /// Staker ka Ed25519 public key (hex, 64 characters).
    pub staker_public_key_hex: String,
    /// Ed25519 signature (hex, 128 characters) — op ID par sign.
    pub signature_hex: String,
}

impl StakeOp {
    /// Bina signature ke op banao (sign karne se pehle).
    pub fn new(kind: StakeOpKind, staker: String, amount: u64, tier: Option<LockTier>, nonce: u64) -> Self {
        Self {
            kind,
            staker,
            amount,
            tier,
            nonce,
            staker_public_key_hex: String::new(),
            signature_hex: String::new(),
        }
    }

    /// Stake ke liye shortcut.
    pub fn stake(staker: String, amount: u64, tier: LockTier, nonce: u64) -> Self {
        Self::new(StakeOpKind::Stake, staker, amount, Some(tier), nonce)
    }

    /// Unstake ke liye shortcut.
    pub fn unstake(staker: String, amount: u64, nonce: u64) -> Self {
        Self::new(StakeOpKind::Unstake, staker, amount, None, nonce)
    }

    /// Claim ke liye shortcut.
    pub fn claim(staker: String, nonce: u64) -> Self {
        Self::new(StakeOpKind::Claim, staker, 0, None, nonce)
    }

    /// Validator register karne ke liye shortcut.
    pub fn register(staker: String, nonce: u64) -> Self {
        Self::new(StakeOpKind::Register, staker, 0, None, nonce)
    }

    /// Op ka ID — fields ka SHA-256 hash (signature ke bina).
    pub fn id(&self) -> String {
        sha256_hex(self.canonical_payload().as_bytes())
    }

    fn tier_str(&self) -> String {
        match self.tier {
            Some(LockTier::Days90) => "90".to_string(),
            Some(LockTier::Days180) => "180".to_string(),
            Some(LockTier::Year1) => "365".to_string(),
            Some(LockTier::Year2) => "730".to_string(),
            Some(LockTier::Year3) => "1095".to_string(),
            Some(LockTier::Year5) => "1825".to_string(),
            None => "-".to_string(),
        }
    }

    fn kind_str(&self) -> &'static str {
        match self.kind {
            StakeOpKind::Stake => "stake",
            StakeOpKind::Unstake => "unstake",
            StakeOpKind::Claim => "claim",
            StakeOpKind::Register => "register",
        }
    }

    /// Sign karne wala payload — hamesha ek hi format.
    /// Format badla to purani signature toot jayegi.
    fn canonical_payload(&self) -> String {
        format!(
            "BUT-STAKE|{}|{}|{}|{}|{}",
            self.kind_str(),
            self.staker,
            self.amount,
            self.tier_str(),
            self.nonce
        )
    }

    /// Staker ki key se op par sign karo.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        let public_key = signing_key.verifying_key().to_bytes();
        let expected = crate::wallet::address_from_public_key(&public_key);
        if expected != self.staker {
            return Err(ButError::InvalidSignature(
                "sign karne wali key staker address se match nahi karti".to_string(),
            ));
        }
        self.staker_public_key_hex = hex::encode(public_key);
        let op_id = self.id();
        let signature = signing_key.sign(op_id.as_bytes());
        self.signature_hex = hex::encode(signature.to_bytes());
        Ok(())
    }

    /// Op verify karo — format + signature. Nonce aur balance ki checking
    /// `Chain` karta hai (state janta hai).
    pub fn verify(&self) -> Result<()> {
        validate_address(&self.staker)?;
        match self.kind {
            StakeOpKind::Stake => {
                if self.amount == 0 {
                    return Err(ButError::InvalidTransaction(
                        "stake amount 0 se zyada hona chahiye".to_string(),
                    ));
                }
                if self.tier.is_none() {
                    return Err(ButError::InvalidTransaction(
                        "stake ke liye lock tier zaroori hai (90/180/365/730/1095/1825 din)".to_string(),
                    ));
                }
            }
            StakeOpKind::Unstake => {
                if self.amount == 0 {
                    return Err(ButError::InvalidTransaction(
                        "unstake amount 0 se zyada hona chahiye".to_string(),
                    ));
                }
                if self.tier.is_some() {
                    return Err(ButError::InvalidTransaction(
                        "unstake ke liye tier nahi chahiye".to_string(),
                    ));
                }
            }
            StakeOpKind::Claim => {
                if self.amount != 0 || self.tier.is_some() {
                    return Err(ButError::InvalidTransaction(
                        "claim me amount 0 aur tier nahi hota — due hissa claim hota hai".to_string(),
                    ));
                }
            }
            StakeOpKind::Register => {
                if self.amount != 0 || self.tier.is_some() {
                    return Err(ButError::InvalidTransaction(
                        "register me amount 0 aur tier nahi hota — locked stake se register hota hai".to_string(),
                    ));
                }
            }
        }
        self.verify_signature()
    }

    fn verify_signature(&self) -> Result<()> {
        let pk_bytes = hex::decode(&self.staker_public_key_hex).map_err(|_| {
            ButError::InvalidSignature("staker public key hex format sahi nahi hai".to_string())
        })?;
        if pk_bytes.len() != 32 {
            return Err(ButError::InvalidSignature(
                "staker public key 32 bytes ki honi chahiye".to_string(),
            ));
        }
        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&pk_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| {
            ButError::InvalidSignature("staker public key valid Ed25519 key nahi hai".to_string())
        })?;
        if crate::wallet::address_from_public_key(&pk_array) != self.staker {
            return Err(ButError::InvalidSignature(
                "public key staker address se match nahi karti".to_string(),
            ));
        }
        let sig_bytes = hex::decode(&self.signature_hex).map_err(|_| {
            ButError::InvalidSignature("signature hex format sahi nahi hai".to_string())
        })?;
        if sig_bytes.len() != 64 {
            return Err(ButError::InvalidSignature(
                "signature 64 bytes (128 hex chars) ki honi chahiye".to_string(),
            ));
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_array);
        public_key
            .verify(self.id().as_bytes(), &signature)
            .map_err(|_| {
                ButError::InvalidSignature(
                    "stake op signature verify nahi hui — op badla gaya ya galat key se sign hua"
                        .to_string(),
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn test_wallet(seed: u8) -> Wallet {
        Wallet::from_entropy(&[seed; 32]).unwrap()
    }

    #[test]
    fn tier_lock_seconds_sahi_hain() {
        assert_eq!(LockTier::Days90.lock_seconds(), 90 * DAY_SECS);
        assert_eq!(LockTier::Year1.lock_seconds(), 365 * DAY_SECS);
        assert_eq!(LockTier::Year5.lock_seconds(), 5 * 365 * DAY_SECS);
        assert_eq!(LockTier::from_days(90).unwrap(), LockTier::Days90);
        assert_eq!(LockTier::from_days(1825).unwrap(), LockTier::Year5);
        assert!(LockTier::from_days(30).is_err());
        assert!(LockTier::from_days(100).is_err());
    }

    #[test]
    fn stake_lock_karta_hai_aur_dobara_jodta_hai() {
        let alice = test_wallet(1);
        let mut state = StakingState::new();
        state.stake(&alice.address, 100_000, LockTier::Days90, 1_700_000_000).unwrap();
        assert_eq!(state.locked_of(&alice.address), 100_000);
        // Dobara stake — amount jud jata hai, tier/locked_at pehle wale.
        state.stake(&alice.address, 50_000, LockTier::Year5, 1_700_000_100).unwrap();
        let lock = &state.locks[&alice.address];
        assert_eq!(lock.amount, 150_000);
        assert_eq!(lock.tier, LockTier::Days90);
        assert_eq!(lock.locked_at, 1_700_000_000);
    }

    #[test]
    fn unstake_lock_khatm_hone_se_pehle_reject() {
        let alice = test_wallet(1);
        let mut state = StakingState::new();
        state.stake(&alice.address, 100_000, LockTier::Days90, 1_700_000_000).unwrap();
        // 89 din baad bhi lock hai.
        let err = state.start_unbonding(&alice.address, 50_000, 1_700_000_000 + 89 * DAY_SECS);
        assert!(err.is_err());
        // 90 din baad unstake chalega.
        state.start_unbonding(&alice.address, 50_000, 1_700_000_000 + 90 * DAY_SECS).unwrap();
        assert_eq!(state.locked_of(&alice.address), 50_000);
        assert_eq!(state.unbonding_of(&alice.address), 50_000);
    }

    #[test]
    fn unbonding_72h_me_dheere_dheere_release() {
        let alice = test_wallet(1);
        let mut state = StakingState::new();
        state.stake(&alice.address, 72_000, LockTier::Days90, 1_700_000_000).unwrap();
        let t_unstake = 1_700_000_000 + 90 * DAY_SECS;
        state.start_unbonding(&alice.address, 72_000, t_unstake).unwrap();
        // Shuru me 0 claimable.
        assert_eq!(state.claimable_of(&alice.address, t_unstake), 0);
        // 24 ghante baad 1/3 (24,000).
        assert_eq!(state.claimable_of(&alice.address, t_unstake + 24 * 3600), 24_000);
        // 36 ghante baad aadha (36,000).
        assert_eq!(state.claimable_of(&alice.address, t_unstake + 36 * 3600), 36_000);
        // Claim karo — 36,000 mile, baaki unbonding me.
        assert_eq!(state.claim(&alice.address, t_unstake + 36 * 3600), 36_000);
        assert_eq!(state.unbonding_of(&alice.address), 36_000);
        // 72 ghante baad baaki 36,000 bhi claimable.
        assert_eq!(state.claim(&alice.address, t_unstake + 72 * 3600), 36_000);
        assert_eq!(state.unbonding_of(&alice.address), 0);
        // Dobara claim — 0.
        assert_eq!(state.claim(&alice.address, t_unstake + 73 * 3600), 0);
    }

    #[test]
    fn unstake_locked_se_zyada_reject() {
        let alice = test_wallet(1);
        let mut state = StakingState::new();
        state.stake(&alice.address, 10_000, LockTier::Days90, 1_700_000_000).unwrap();
        let t = 1_700_000_000 + 90 * DAY_SECS;
        assert!(state.start_unbonding(&alice.address, 10_001, t).is_err());
        // Bina lock ke unstake bhi reject.
        let bob = test_wallet(2);
        assert!(state.start_unbonding(&bob.address, 1, t).is_err());
    }

    #[test]
    fn slash_lock_kam_karta_hai() {
        let alice = test_wallet(1);
        let mut state = StakingState::new();
        state.stake(&alice.address, 100_000, LockTier::Days90, 1_700_000_000).unwrap();
        assert_eq!(state.slash_lock(&alice.address, 10_000), 10_000);
        assert_eq!(state.locked_of(&alice.address), 90_000);
        // Lock se zyada kaatne ki koshish — jitna hai utna kata.
        assert_eq!(state.slash_lock(&alice.address, 200_000), 90_000);
        assert_eq!(state.locked_of(&alice.address), 0);
        assert!(!state.locks.contains_key(&alice.address));
    }

    #[test]
    fn stake_op_sign_verify_aur_tamper() {
        let alice = test_wallet(1);
        let mut op = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        op.verify().unwrap();
        // Tamper: amount badla — signature toot gayi.
        let mut bad = op.clone();
        bad.amount = 999;
        assert!(bad.verify().is_err());
        // Bina signature reject.
        let unsigned = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        assert!(unsigned.verify().is_err());
        // Galat key se sign reject (sign() khud rokega).
        let bob = test_wallet(2);
        let mut op2 = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        assert!(op2.sign(&bob.signing_key()).is_err());
    }

    #[test]
    fn stake_op_format_rules() {
        let alice = test_wallet(1);
        // Stake bina tier reject.
        let mut op = StakeOp::new(StakeOpKind::Stake, alice.address.clone(), 100, None, 0);
        op.sign(&alice.signing_key()).unwrap();
        assert!(op.verify().is_err());
        // Unstake 0 amount reject.
        let mut op2 = StakeOp::unstake(alice.address.clone(), 0, 0);
        op2.sign(&alice.signing_key()).unwrap();
        assert!(op2.verify().is_err());
        // Claim sahi hai.
        let mut op3 = StakeOp::claim(alice.address.clone(), 0);
        op3.sign(&alice.signing_key()).unwrap();
        op3.verify().unwrap();
        // Register sahi hai.
        let mut op4 = StakeOp::register(alice.address.clone(), 0);
        op4.sign(&alice.signing_key()).unwrap();
        op4.verify().unwrap();
    }
}
