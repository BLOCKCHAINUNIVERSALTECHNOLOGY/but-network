//! Consensus — Phase 2: Proof-of-Stake + BFT vote (simple, honest version).
//!
//! Phase 1 me sirf Proof-of-Authority (PoA) tha — ek authorized validator.
//! Phase 2 me PoS + BFT ki machinery ban gayi hai:
//! - **Staking**: validator banne ke liye stake lock karna padta hai
//!   (`MIN_STAKE` ya zyada). Stake bina validator nahi ban sakta.
//! - **Proposer chunav**: har block-height (round) ke liye ek proposer
//!   stake-weighted deterministic tareeke se chuna jata hai — zyada stake,
//!   zyada chance. Koi bhi node khud calculate kar sakta hai ki is round
//!   ka proposer kaun hona chahiye tha.
//! - **BFT vote**: block tabhi final mana jata hai jab kul active stake
//!   ke **2/3 se zyada** hisse ne us block par `approve` vote diya ho.
//!   Vote Ed25519 signature se sign hota hai — nakli vote pakda jayega.
//! - **Slashing**: galat kaam (jaise ek hi height par do alag blocks ko
//!   vote dena — equivocation) pakda jaye to stake ka 10% kat jata hai.
//!   Stake `MIN_STAKE` se neeche gir jaye to validator jail ho jata hai
//!   (block propose / vote nahi kar sakta) jab tak dobara stake na kare.
//!
//! SACH BATA RAHE HAIN — kya abhi bhi simplified hai:
//! - **Slashing yahan bookkeeping hai**: `ValidatorSet` ke andar stake
//!   kaatna aur jail karna real hai, par ye abhi `Chain` ke balances se
//!   juda nahi hai — matlab stake ke coins abhi chain-state me lock nahi
//!   hote, sirf validator set me darj hote hain. Asli stake-locking
//!   (coins chain par lock hona, unstake par unbonding period) Phase 3 ka
//!   kaam hai.
//! - **BFT path opt-in hai**: `Chain::add_block_bft` block tabhi leta hai
//!   jab 2/3 quorum ho aur proposer sahi ho. Purana `Chain::add_block`
//!   abhi bhi PoA tareeke se chalta hai (sirf authorized validator check).
//!   Poori tarah BFT-only chain (har block ke liye vote zaroori) abhi
//!   baaki hai — uske liye pehle P2P par vote gossip chahiye.
//! - **Single-machine test**: vote aur quorum yahan ek hi machine par
//!   test hote hain. Alag-alag machines par baithe validators ke beech
//!   asli BFT (network delay, byzantine node) abhi test nahi hua.
//! - **No fork choice**: do alag chains me se lambi/sahi chain chunne ka
//!   rule (longest-chain / fork-choice) abhi nahi hai — sync sirf apni
//!   chain ko aage badhata hai, reorg nahi karta.
//!
//! Constants tokenomics se jude hain (docs/TOKENOMICS-DRAFT.md).
//! Sanjay ke FINAL faisle (2026-11-07):
//! - Sabse chhoti unit **"bits"** (Sanjay ne B-I-T-S spell kiya) —
//!   1 BUT = 1,000 bits.
//! - Max supply **1 billion BUT** — ek saath unlock nahi, demand ke
//!   hisaab se dheere-dheere release hogi.
//! - Block reward demand ke hisaab se hoga (fixed number nahi) —
//!   uska formula abhi design hona baaki hai.
//! - Minimum stake **1% se 5% tak** — kis cheez ka %, ye confirm
//!   hone par `MIN_STAKE` badlega. Tab tak 100 BUT placeholder hai.
//! - Fees Solana jaisi bilkul kam, bits me kategi.
//! - Team allocation sirf **1%** hai (2026-11-07, pehle 10% tha).
//!   Ongoing team funding: har block reward ka **0.1%** team account me
//!   jayega — implement hoga jab block rewards Phase 3 me judenge.
//! - Liquidity rule: swap sirf available liquidity tak, partial fill
//!   allowed — implement hoga jab swap feature banega.

use crate::error::{ButError, Result};
use crate::hash::sha256_hex;
use crate::wallet::validate_address;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 1 BUT = kitne bits (smallest units). FINAL — Sanjay, 2026-11-07.
pub const BITS_PER_BUT: u64 = 1_000;

/// Max supply — isse zyada BUT kabhi nahi banenge. FINAL — Sanjay, 2026-11-07.
/// (Release dheere-dheere, demand ke hisaab se — ek saath unlock nahi.)
pub const MAX_SUPPLY_BUT: u64 = 1_000_000_000;

/// Validator banne ke liye minimum stake — bits me.
/// TODO: Sanjay ka FINAL faisla minimum stake **1% se 5% tak, RELEASED
/// supply ka** hai (2026-11-07) — jitni supply release hoti jayegi, uska
/// 1-5%. Ye constant tab badlega jab chain me released-supply tracking
/// jud jayegi (Phase 3). Tab tak 100 BUT placeholder hai.
pub const MIN_STAKE: u64 = 100 * BITS_PER_BUT;

/// Slashing: galat kaam par stake ka kitna hissa katega (10%).
pub const SLASH_PERCENT: u64 = 10;

/// BFT quorum: approvals ka stake kul active stake ke 2/3 se ZYADA hona chahiye.
/// Matlab 4 barabar validators me se 3 ki haan chahiye (2 ki nahi).
pub const QUORUM_NUMERATOR: u64 = 2;
pub const QUORUM_DENOMINATOR: u64 = 3;

/// Authorized validators ki list + unka stake.
/// Phase 1 me sirf `validators` list thi. Phase 2 me stake, slashing aur
/// jail ki jaankari bhi yahin rehti hai. Naye fields par `#[serde(default)]`
/// hai, isliye Phase 1 ki purani chain.json file bhi load ho jayegi
/// (stake 0 mana jayega — PoA legacy chain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSet {
    /// Authorized validator addresses ("but1...", 56 chars).
    pub validators: Vec<String>,
    /// Har validator ka locked stake (smallest units). Na ho to 0.
    #[serde(default)]
    pub stakes: HashMap<String, u64>,
    /// Har validator ka ab tak kul kata hua stake (slashing history).
    #[serde(default)]
    pub slashed_total: HashMap<String, u64>,
    /// Jail hue validators — block propose / vote nahi kar sakte.
    #[serde(default)]
    pub jailed: Vec<String>,
}

impl ValidatorSet {
    /// Ek validator se set banao (Phase 1 compat).
    /// Is validator ka stake 0 darj hota hai — ye PoA legacy hai.
    /// PoS me aane ke liye `restake` se `MIN_STAKE` ya zyada stake karo.
    pub fn single(validator_address: String) -> Result<Self> {
        validate_address(&validator_address)?;
        Ok(Self {
            validators: vec![validator_address],
            stakes: HashMap::new(),
            slashed_total: HashMap::new(),
            jailed: Vec::new(),
        })
    }

    /// Kya ye address authorized validator hai?
    pub fn is_validator(&self, address: &str) -> bool {
        self.validators.iter().any(|v| v == address)
    }

    /// Kya ye validator jail me hai?
    pub fn is_jailed(&self, address: &str) -> bool {
        self.jailed.iter().any(|v| v == address)
    }

    /// Active validators — authorized aur jail me nahi.
    pub fn active_validators(&self) -> Vec<String> {
        self.validators
            .iter()
            .filter(|v| !self.is_jailed(v))
            .cloned()
            .collect()
    }

    /// Kisi validator ka stake (na ho to 0).
    pub fn stake_of(&self, address: &str) -> u64 {
        self.stakes.get(address).copied().unwrap_or(0)
    }

    /// Kul active stake — sirf active validators ka stake joda jata hai.
    pub fn total_active_stake(&self) -> u64 {
        self.active_validators()
            .iter()
            .map(|v| self.stake_of(v))
            .sum()
    }

    /// Block banane ki permission check — galat ya jail validator reject.
    pub fn authorize_block(&self, validator_address: &str) -> Result<()> {
        if !self.is_validator(validator_address) {
            return Err(ButError::InvalidBlock(format!(
                "ye validator authorized nahi hai: {validator_address}"
            )));
        }
        if self.is_jailed(validator_address) {
            return Err(ButError::InvalidBlock(format!(
                "ye validator jail me hai (slashed): {validator_address}"
            )));
        }
        Ok(())
    }

    /// Stake ke saath naya validator jodo.
    /// Rules: address sahi ho, stake `MIN_STAKE` ya zyada ho, pehle se
    /// validator na ho, jail me na ho.
    pub fn add_staked_validator(&mut self, address: String, stake: u64) -> Result<()> {
        validate_address(&address)?;
        if stake < MIN_STAKE {
            return Err(ButError::InvalidTransaction(format!(
                "stake kam hai: chahiye kam se kam {MIN_STAKE} bits (abhi placeholder: 100 BUT; Sanjay ka faisla 1-5% hai, base confirm hone par badlega), mila {stake}"
            )));
        }
        if self.is_validator(&address) {
            return Err(ButError::InvalidTransaction(format!(
                "ye address pehle se validator hai: {address}"
            )));
        }
        if self.is_jailed(&address) {
            return Err(ButError::InvalidTransaction(format!(
                "ye validator jail me hai — pehle restake karke unjail karo: {address}"
            )));
        }
        self.validators.push(address.clone());
        self.stakes.insert(address, stake);
        Ok(())
    }

    /// Validator hatao (stake record bhi hatao). Jail list se bhi hatao.
    pub fn remove_validator(&mut self, address: &str) {
        self.validators.retain(|v| v != address);
        self.stakes.remove(address);
        self.jailed.retain(|v| v != address);
    }

    /// Stake badhao. Jail validator ka stake `MIN_STAKE` par pahunche to
    /// wo apne aap unjail ho jata hai.
    pub fn restake(&mut self, address: &str, additional: u64) -> Result<()> {
        if !self.is_validator(address) {
            return Err(ButError::InvalidTransaction(format!(
                "restake: ye validator nahi hai: {address}"
            )));
        }
        let new_stake = self.stake_of(address).saturating_add(additional);
        self.stakes.insert(address.to_string(), new_stake);
        if new_stake >= MIN_STAKE {
            self.jailed.retain(|v| v != address);
        }
        Ok(())
    }

    /// Is round (block height) ka proposer chuno — stake-weighted, deterministic.
    /// Koi bhi node khud calculate kar sakta hai: round ka hash nikalo,
    /// kul stake me se ek point chuno, stake ke hisaab se chalo.
    /// Kul stake 0 ho (PoA legacy chain) to round-robin fallback hai.
    /// Koi active validator na ho to None.
    pub fn proposer_for_round(&self, round: u64) -> Option<String> {
        let mut active = self.active_validators();
        if active.is_empty() {
            return None;
        }
        active.sort();
        let total = self.total_active_stake();
        if total == 0 {
            // PoA legacy: stake darj nahi hai — round-robin.
            return Some(active[(round as usize) % active.len()].clone());
        }
        let hash = sha256_hex(format!("BUT-PROPOSER|{round}").as_bytes());
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hex::decode(&hash[..16]).unwrap_or(vec![0; 8]));
        let point = u64::from_be_bytes(bytes) % total;
        let mut cumulative: u64 = 0;
        for v in &active {
            cumulative += self.stake_of(v);
            if point < cumulative {
                return Some(v.clone());
            }
        }
        Some(active[active.len() - 1].clone())
    }

    /// Validator ko slash karo — galat kaam ki saza.
    /// Stake ka `SLASH_PERCENT`% (10%) kat jata hai. Stake `MIN_STAKE` se
    /// neeche gir jaye to validator jail ho jata hai. Stake 0 ho (PoA
    /// legacy) to kaatne ko kuch nahi — seedha jail.
    /// Returns: kitna stake kata (smallest units).
    pub fn slash(&mut self, address: &str, reason: &str) -> Result<u64> {
        if !self.is_validator(address) {
            return Err(ButError::InvalidTransaction(format!(
                "slash: ye validator nahi hai: {address}"
            )));
        }
        let stake = self.stake_of(address);
        let cut = stake * SLASH_PERCENT / 100;
        let remaining = stake.saturating_sub(cut);
        self.stakes.insert(address.to_string(), remaining);
        *self.slashed_total.entry(address.to_string()).or_insert(0) += cut;
        if remaining < MIN_STAKE && !self.is_jailed(address) {
            self.jailed.push(address.to_string());
        }
        let _ = reason; // reason caller log karega; yahan sirf record hai.
        Ok(cut)
    }
}

/// Ek validator ka BFT vote — block par haan/na, signature ke saath.
///
/// Vote sign hota hai payload `BUT-VOTE|<block_hash>|<height>|<approve>` par.
/// Public key vote me hi hoti hai, taaki koi bhi node bina registry ke
/// verify kar sake (transaction jaisa hi pattern).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    /// Jis block par vote hai, uska hash.
    pub block_hash: String,
    /// Block ki height (index).
    pub height: u64,
    /// true = approve, false = reject.
    pub approve: bool,
    /// Vote dene wale validator ka address.
    pub voter: String,
    /// Voter ka Ed25519 public key (hex, 64 chars).
    pub voter_public_key_hex: String,
    /// Ed25519 signature (hex, 128 chars).
    pub signature_hex: String,
}

impl Vote {
    /// Naya vote banao aur voter ki key se sign karo.
    pub fn new(
        block_hash: String,
        height: u64,
        approve: bool,
        voter: String,
        voter_key: &SigningKey,
    ) -> Result<Self> {
        validate_address(&voter)?;
        let public_key = voter_key.verifying_key().to_bytes();
        let expected = crate::wallet::address_from_public_key(&public_key);
        if expected != voter {
            return Err(ButError::InvalidSignature(
                "vote karne wali key voter address se match nahi karti".to_string(),
            ));
        }
        let mut vote = Self {
            block_hash,
            height,
            approve,
            voter,
            voter_public_key_hex: hex::encode(public_key),
            signature_hex: String::new(),
        };
        let payload = vote.canonical_payload();
        vote.signature_hex = hex::encode(voter_key.sign(payload.as_bytes()).to_bytes());
        Ok(vote)
    }

    fn canonical_payload(&self) -> String {
        format!(
            "BUT-VOTE|{}|{}|{}",
            self.block_hash, self.height, self.approve
        )
    }

    /// Vote verify karo — address, public key aur signature teenon check.
    pub fn verify(&self) -> Result<()> {
        validate_address(&self.voter)?;
        let pk_bytes = hex::decode(&self.voter_public_key_hex).map_err(|_| {
            ButError::InvalidSignature("voter public key hex format sahi nahi hai".to_string())
        })?;
        if pk_bytes.len() != 32 {
            return Err(ButError::InvalidSignature(
                "voter public key 32 bytes ki honi chahiye".to_string(),
            ));
        }
        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&pk_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| {
            ButError::InvalidSignature("voter public key valid Ed25519 key nahi hai".to_string())
        })?;
        if crate::wallet::address_from_public_key(&pk_array) != self.voter {
            return Err(ButError::InvalidSignature(
                "public key voter address se match nahi karti".to_string(),
            ));
        }
        let sig_bytes = hex::decode(&self.signature_hex).map_err(|_| {
            ButError::InvalidSignature("vote signature hex format sahi nahi hai".to_string())
        })?;
        if sig_bytes.len() != 64 {
            return Err(ButError::InvalidSignature(
                "vote signature 64 bytes (128 hex chars) ki honi chahiye".to_string(),
            ));
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_array);
        public_key
            .verify(self.canonical_payload().as_bytes(), &signature)
            .map_err(|_| {
                ButError::InvalidSignature(
                    "vote signature verify nahi hui — vote badla gaya ya galat key se sign hua"
                        .to_string(),
                )
            })?;
        Ok(())
    }
}

/// Ek height ke liye votes ka hisaab — quorum check aur equivocation pakadna.
///
/// Equivocation = ek hi validator ne ek hi height par do ALAG blocks ko
/// approve vote diya. Ye BFT me sabse bada jurm hai — isse fork banaya
/// ja sakta hai. `equivocators()` aise validators ko pakadta hai.
#[derive(Debug, Default, Clone)]
pub struct VoteTally {
    /// Kis height ke votes hain.
    pub height: u64,
    /// Saare votes (approve aur reject, dono).
    pub votes: Vec<Vote>,
}

impl VoteTally {
    /// Nayi khaali tally, is height ke liye.
    pub fn new(height: u64) -> Self {
        Self {
            height,
            votes: Vec::new(),
        }
    }

    /// Vote jodo. Check: height match, signature sahi, voter active
    /// validator ho, bilkul wahi vote pehle se na ho.
    /// Conflicting vote (wahi voter, wahi height, alag block) reject nahi
    /// hota — wo store hota hai taaki `equivocators()` use pakad sake.
    pub fn add_vote(&mut self, vote: Vote, set: &ValidatorSet) -> Result<()> {
        if vote.height != self.height {
            return Err(ButError::InvalidTransaction(format!(
                "vote ki height galat hai: mila {}, tally ki height {}",
                vote.height, self.height
            )));
        }
        vote.verify()?;
        if !set.active_validators().contains(&vote.voter) {
            return Err(ButError::InvalidTransaction(format!(
                "vote dene wala active validator nahi hai: {}",
                vote.voter
            )));
        }
        if self
            .votes
            .iter()
            .any(|v| v.voter == vote.voter && v.block_hash == vote.block_hash)
        {
            return Err(ButError::InvalidTransaction(format!(
                "ye voter is block par pehle se vote de chuka hai: {}",
                vote.voter
            )));
        }
        self.votes.push(vote);
        Ok(())
    }

    /// Is block hash ko approve karne walon ka kul stake.
    pub fn approval_stake_for(&self, block_hash: &str, set: &ValidatorSet) -> u64 {
        let mut voters: HashSet<&str> = HashSet::new();
        let mut stake: u64 = 0;
        for v in &self.votes {
            if v.block_hash == block_hash && v.approve && voters.insert(v.voter.as_str()) {
                stake += set.stake_of(&v.voter);
            }
        }
        stake
    }

    /// Kya is block ko 2/3 quorum mila? Approvals ka stake kul active
    /// stake ke 2/3 se ZYADA hona chahiye (barabar nahi — zyada).
    pub fn has_quorum_for(&self, block_hash: &str, set: &ValidatorSet) -> bool {
        let total = set.total_active_stake();
        if total == 0 {
            return false; // stake hi nahi hai to BFT quorum ka matlab nahi
        }
        let approvals = self.approval_stake_for(block_hash, set);
        approvals * QUORUM_DENOMINATOR > total * QUORUM_NUMERATOR
    }

    /// Equivocators — ek hi height par do alag blocks ko approve karne wale.
    pub fn equivocators(&self) -> Vec<String> {
        let mut by_voter: HashMap<&str, HashSet<&str>> = HashMap::new();
        for v in &self.votes {
            if v.approve {
                by_voter
                    .entry(v.voter.as_str())
                    .or_default()
                    .insert(v.block_hash.as_str());
            }
        }
        by_voter
            .into_iter()
            .filter(|(_, hashes)| hashes.len() > 1)
            .map(|(voter, _)| voter.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn wallet(seed: u8) -> Wallet {
        Wallet::from_entropy(&[seed; 32]).unwrap()
    }

    fn staked_set() -> (ValidatorSet, Vec<Wallet>) {
        // 4 validators, har ek ka stake 100 BUT (MIN_STAKE) = kul 400 BUT.
        let wallets: Vec<Wallet> = (1..=4).map(|i| wallet(i)).collect();
        let mut set = ValidatorSet::single(wallets[0].address.clone()).unwrap();
        set.restake(&wallets[0].address, MIN_STAKE).unwrap();
        for w in &wallets[1..] {
            set.add_staked_validator(w.address.clone(), MIN_STAKE)
                .unwrap();
        }
        (set, wallets)
    }

    #[test]
    fn authorized_validator_ok_galat_reject() {
        let v = wallet(9);
        let other = wallet(8);
        let set = ValidatorSet::single(v.address.clone()).unwrap();
        assert!(set.authorize_block(&v.address).is_ok());
        assert!(set.authorize_block(&other.address).is_err());
    }

    #[test]
    fn kam_stake_validator_reject() {
        let (mut set, wallets) = staked_set();
        let newcomer = wallet(7);
        // MIN_STAKE se 1 kam — reject hona chahiye.
        assert!(set
            .add_staked_validator(newcomer.address.clone(), MIN_STAKE - 1)
            .is_err());
        // Bilkul MIN_STAKE — ok.
        assert!(set
            .add_staked_validator(newcomer.address.clone(), MIN_STAKE)
            .is_ok());
        let _ = wallets;
    }

    #[test]
    fn duplicate_validator_reject() {
        let (mut set, wallets) = staked_set();
        assert!(set
            .add_staked_validator(wallets[1].address.clone(), MIN_STAKE)
            .is_err());
    }

    #[test]
    fn proposer_deterministic_aur_stake_weighted() {
        let (set, wallets) = staked_set();
        // Wahi round hamesha wahi proposer de.
        let p1 = set.proposer_for_round(5).unwrap();
        assert_eq!(p1, set.proposer_for_round(5).unwrap());
        assert!(wallets.iter().any(|w| w.address == p1));

        // Stake-weighted: ek validator ko 3x stake do — 1000 rounds me
        // uske proposer banne ki ginti baakiyon se clearly zyada honi chahiye.
        let mut weighted = ValidatorSet::single(wallets[0].address.clone()).unwrap();
        weighted.restake(&wallets[0].address, MIN_STAKE * 3).unwrap();
        for w in &wallets[1..] {
            weighted
                .add_staked_validator(w.address.clone(), MIN_STAKE)
                .unwrap();
        }
        let mut counts: HashMap<String, usize> = HashMap::new();
        for round in 0..1000u64 {
            let p = weighted.proposer_for_round(round).unwrap();
            *counts.entry(p).or_insert(0) += 1;
        }
        let big = counts[&wallets[0].address];
        let small = counts[&wallets[1].address];
        // Big ka stake 50% hai (3/6) — 1000 rounds me ~500 baar aana chahiye,
        // small (~167) se clearly zyada. Range loose rakhi hai taaki flaky na ho.
        assert!(big > 400 && big < 600, "big proposer count: {big}");
        assert!(small > 100 && small < 250, "small proposer count: {small}");
        assert!(big > small * 2);
    }

    #[test]
    fn quorum_teen_chaar_haan_chahiye() {
        let (set, wallets) = staked_set();
        let mut tally = VoteTally::new(1);
        let hash = "a".repeat(64);
        // 2 approvals (200/400 = 1/2) — quorum NAHI.
        for w in &wallets[..2] {
            let v = Vote::new(hash.clone(), 1, true, w.address.clone(), &w.signing_key())
                .unwrap();
            tally.add_vote(v, &set).unwrap();
        }
        assert!(!tally.has_quorum_for(&hash, &set));
        // 3rd approval (300/400 = 3/4 > 2/3) — quorum HAAN.
        let v = Vote::new(
            hash.clone(),
            1,
            true,
            wallets[2].address.clone(),
            &wallets[2].signing_key(),
        )
        .unwrap();
        tally.add_vote(v, &set).unwrap();
        assert!(tally.has_quorum_for(&hash, &set));
        // Reject vote quorum me nahi gina jata.
        let mut tally2 = VoteTally::new(1);
        for w in &wallets[..3] {
            let approve = w.address != wallets[2].address;
            let v = Vote::new(
                hash.clone(),
                1,
                approve,
                w.address.clone(),
                &w.signing_key(),
            )
            .unwrap();
            tally2.add_vote(v, &set).unwrap();
        }
        assert!(!tally2.has_quorum_for(&hash, &set));
    }

    #[test]
    fn galat_signature_wala_vote_reject() {
        let (set, wallets) = staked_set();
        let mut tally = VoteTally::new(1);
        let mut v = Vote::new(
            "b".repeat(64),
            1,
            true,
            wallets[0].address.clone(),
            &wallets[0].signing_key(),
        )
        .unwrap();
        v.approve = false; // sign ke baad badla — signature toot gayi
        assert!(tally.add_vote(v, &set).is_err());
    }

    #[test]
    fn non_validator_ka_vote_reject_aur_duplicate_reject() {
        let (set, wallets) = staked_set();
        let mut tally = VoteTally::new(1);
        let outsider = wallet(7);
        let v = Vote::new(
            "c".repeat(64),
            1,
            true,
            outsider.address.clone(),
            &outsider.signing_key(),
        )
        .unwrap();
        assert!(tally.add_vote(v, &set).is_err());
        // Duplicate vote reject.
        let hash = "d".repeat(64);
        let v1 = Vote::new(
            hash.clone(),
            1,
            true,
            wallets[0].address.clone(),
            &wallets[0].signing_key(),
        )
        .unwrap();
        tally.add_vote(v1.clone(), &set).unwrap();
        assert!(tally.add_vote(v1, &set).is_err());
    }

    #[test]
    fn equivocation_pakda_jata_hai_aur_slash_jail_karta_hai() {
        let (mut set, wallets) = staked_set();
        let mut tally = VoteTally::new(1);
        // wallets[0] ek hi height par do alag blocks ko approve karta hai.
        let v1 = Vote::new(
            "e".repeat(64),
            1,
            true,
            wallets[0].address.clone(),
            &wallets[0].signing_key(),
        )
        .unwrap();
        let v2 = Vote::new(
            "f".repeat(64),
            1,
            true,
            wallets[0].address.clone(),
            &wallets[0].signing_key(),
        )
        .unwrap();
        tally.add_vote(v1, &set).unwrap();
        tally.add_vote(v2, &set).unwrap();
        let bad = tally.equivocators();
        assert_eq!(bad, vec![wallets[0].address.clone()]);

        // Slash: 10% kata — MIN_STAKE ka 10%.
        let cut = set.slash(&wallets[0].address, "equivocation").unwrap();
        assert_eq!(cut, MIN_STAKE / 10);
        assert_eq!(set.stake_of(&wallets[0].address), MIN_STAKE - cut);
        assert_eq!(
            set.slashed_total[&wallets[0].address],
            MIN_STAKE / 10
        );
        // Stake ab MIN_STAKE se neeche hai — jail hona chahiye.
        assert!(set.is_jailed(&wallets[0].address));
        assert!(set.authorize_block(&wallets[0].address).is_err());
        // Restake karke unjail ho sakta hai.
        set.restake(&wallets[0].address, MIN_STAKE).unwrap();
        assert!(!set.is_jailed(&wallets[0].address));
        assert!(set.authorize_block(&wallets[0].address).is_ok());
    }

    #[test]
    fn slash_bina_stake_wale_poa_validator_ko_jail_karta_hai() {
        let v = wallet(9);
        let mut set = ValidatorSet::single(v.address.clone()).unwrap();
        let cut = set.slash(&v.address, "galat block").unwrap();
        assert_eq!(cut, 0); // kaatne ko stake hi nahi tha
        assert!(set.is_jailed(&v.address));
    }
}
