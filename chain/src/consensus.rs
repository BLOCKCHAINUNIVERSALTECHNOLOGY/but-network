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
//! - **Stake-locking Phase 3a me jud gaya hai** (`staking.rs`): staked
//!   coins ab chain par lock hote hain — `Chain` stake karte hi coins ko
//!   spendable balance se nikaal kar lock me dalta hai, aur slashing lock
//!   se bhi kaat ta hai. `ValidatorSet.stakes` aur lock hamesha ek saath
//!   update hote hain (`Chain` ke andar).
//! - **BFT-only chain Phase 3a me hai**: purana PoA `add_block` hata diya
//!   gaya — har non-genesis block `add_block_bft` se hi judta hai (sahi
//!   proposer + 2/3 quorum). Votes P2P par gossip hote hain (`p2p.rs`).
//! - **Bootstrap rule**: jab tak kisi ne stake nahi kiya (kul active
//!   stake 0 — sirf dev/legacy genesis), ek hi validator ki chain me
//!   us validator ka khud ka approve vote quorum mana jata hai. Jaise hi
//!   koi stake karta hai, ye rule band ho jata hai aur sakht 2/3 quorum
//!   lagta hai. Ye dev chains ko chalane ke liye hai, production rule nahi.
//! - **Block reward PLACEHOLDER schedule hai**: Sanjay ka FINAL faisla
//!   demand-based reward hai, par uska exact formula abhi pending hai.
//!   Tab tak `block_reward_for` ek simple fixed schedule deta hai (5 BUT
//!   se shuru, har ~4 saal aadha) — mechanism ke roop me. Demand-based
//!   formula aate hi ye badlega; FINAL faisla overwrite nahi hua hai.
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

/// Minimum stake — **released supply ka 1%** (FINAL — Sanjay, 2026-11-07).
/// Sanjay ka faisla "1% se 5% tak, released supply ka" hai. Code me range
/// ka **neeche wala hissa (1%)** minimum ke roop me lagaya gaya hai, taaki
/// zyada log validator ban sakein (5% lagane par sirf ameer log hi ban
/// payenge — research me yehi dikkat likhi hai). `MIN_STAKE_PERCENT_MAX`
/// upar wala hissa record karta hai, enforce nahi hota.
/// Released supply `Chain` track karti hai (genesis allocations + ab tak
/// ke block rewards). Supply 0 ho to minimum 1 bit hota hai.
pub const MIN_STAKE_PERCENT: u64 = 1;
/// Minimum stake range ka upar wala hissa — sirf record ke liye (5%).
pub const MIN_STAKE_PERCENT_MAX: u64 = 5;

/// Released supply ke hisaab se minimum stake (bits me).
/// Jaise released 100,000,000 bits (100,000 BUT) ho to minimum
/// 1,000,000 bits (1,000 BUT) hoga.
pub fn min_stake_for(released_supply: u64) -> u64 {
    (released_supply / 100 * MIN_STAKE_PERCENT).max(1)
}

/// Slashing: galat kaam par stake ka kitna hissa katega (10%).
pub const SLASH_PERCENT: u64 = 10;

/// Block reward — PLACEHOLDER schedule (mechanism ke liye).
///
/// **DHYAAN DO — ye FINAL nahi hai.** Sanjay ka FINAL faisla hai ki block
/// reward **demand ke hisaab se** hoga (fixed number nahi), par uska exact
/// formula abhi pending hai (demand kaise naapi jayegi — Sanjay ka faisla
/// baaki hai). Tab tak chain chalane ke liye ek simple fixed schedule hai:
/// shuru me 5 BUT per block, har ~4 saal aadha (Bitcoin-jaisa halving).
/// Jaise hi demand-based formula aayega, `block_reward_for` badlega —
/// FINAL faisla overwrite nahi hua hai, sirf mechanism placeholder hai.
///
/// - Shuruati reward: **5 BUT** (5,000 bits) per block.
/// - Halving: har 25,228,800 blocks (~4 saal, 5 sec/block ke hisaab se)
///   reward aadha.
/// - Cap: reward kabhi max supply (1 billion BUT) se zyada mint nahi
///   karega — supply poori release hote hi reward 0 ho jata hai (endgame,
///   tokenomics §9 — uske baad sirf fees).
pub const INITIAL_BLOCK_REWARD_BITS: u64 = 5 * BITS_PER_BUT;
/// Halving interval — blocks me (~4 saal, 5 sec/block).
pub const REWARD_HALVING_INTERVAL_BLOCKS: u64 = 25_228_800;

/// Har block reward ka kitna hissa team ko jayega — **0.1%** (FINAL,
/// Sanjay, 2026-11-07). Per-mille me: 1/1000.
pub const TEAM_REWARD_SHARE_PER_MILLE: u64 = 1;

/// Is height ke block ka reward (bits me) — placeholder schedule.
/// `released_supply` ab tak release hui supply hai (genesis + rewards).
/// Supply cap tak pahunch jaye to reward 0.
pub fn block_reward_for(height: u64, released_supply: u64) -> u64 {
    let max_supply_bits = MAX_SUPPLY_BUT * BITS_PER_BUT;
    if released_supply >= max_supply_bits {
        return 0;
    }
    let halvings = height / REWARD_HALVING_INTERVAL_BLOCKS;
    let reward = if halvings >= 64 {
        0
    } else {
        INITIAL_BLOCK_REWARD_BITS >> halvings
    };
    reward.min(max_supply_bits - released_supply)
}

/// Block reward me se team ka hissa (0.1% — FINAL, Sanjay).
/// Jaise reward 5,000 bits ho to team ko 5 bits milenge.
pub fn team_share_of(reward: u64) -> u64 {
    reward * TEAM_REWARD_SHARE_PER_MILLE / 1000
}

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
    /// PoS me aane ke liye stake karke `min_stake_for` ya zyada lock karo.
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
    /// Rules: address sahi ho, stake `min_stake_for(released_supply)` ya
    /// zyada ho (FINAL: released supply ka 1%), pehle se validator na ho,
    /// jail me na ho.
    pub fn add_staked_validator(
        &mut self,
        address: String,
        stake: u64,
        released_supply: u64,
    ) -> Result<()> {
        validate_address(&address)?;
        let min_stake = min_stake_for(released_supply);
        if stake < min_stake {
            return Err(ButError::InvalidTransaction(format!(
                "stake kam hai: chahiye kam se kam {min_stake} bits (released supply {released_supply} bits ka 1% — FINAL, Sanjay), mila {stake}"
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

    /// Stake badhao. Jail validator ka stake `min_stake_for` par pahunche
    /// to wo apne aap unjail ho jata hai.
    pub fn restake(&mut self, address: &str, additional: u64, released_supply: u64) -> Result<()> {
        if !self.is_validator(address) {
            return Err(ButError::InvalidTransaction(format!(
                "restake: ye validator nahi hai: {address}"
            )));
        }
        let new_stake = self.stake_of(address).saturating_add(additional);
        self.stakes.insert(address.to_string(), new_stake);
        if new_stake >= min_stake_for(released_supply) {
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
    /// Stake ka `SLASH_PERCENT`% (10%) kat jata hai. Stake
    /// `min_stake_for` se neeche gir jaye to validator jail ho jata hai.
    /// Stake 0 ho (PoA legacy) to kaatne ko kuch nahi — seedha jail.
    /// Returns: kitna stake kata (smallest units).
    /// Note: ye sirf validator-set ka record kaat ta hai — locked coins
    /// kaatne ke liye caller (`Chain::apply_slash_op`, block-level SlashOp)
    /// ko `StakingState::slash_lock` bhi chalana chahiye (wo lock se bhi
    /// utna hi kaat ta hai).
    pub fn slash(&mut self, address: &str, reason: &str, released_supply: u64) -> Result<u64> {
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
        if remaining < min_stake_for(released_supply) && !self.is_jailed(address) {
            self.jailed.push(address.to_string());
        }
        let _ = reason; // reason caller log karega; yahan sirf record hai.
        Ok(cut)
    }

    /// Validator ko jail karo agar uska stake minimum se neeche gir gaya ho
    /// (jaise unstake ke baad). Jail hua validator propose/vote nahi kar sakta.
    pub fn jail_if_below_min(&mut self, address: &str, released_supply: u64) {
        if self.is_validator(address)
            && !self.is_jailed(address)
            && self.stake_of(address) < min_stake_for(released_supply)
        {
            self.jailed.push(address.to_string());
        }
    }

    /// Validator ka stake record locked amount ke barabar rakho.
    /// Unstake ke baad lock kam hota hai to record bhi kam hota hai;
    /// record minimum se neeche gira to validator jail ho jata hai.
    /// (Stake badhane ke liye `restake` use karo — ye sirf sync ke liye hai.)
    pub fn sync_stake(
        &mut self,
        address: &str,
        locked: u64,
        released_supply: u64,
    ) -> Result<()> {
        if !self.is_validator(address) {
            return Err(ButError::InvalidTransaction(format!(
                "sync_stake: ye validator nahi hai: {address}"
            )));
        }
        self.stakes.insert(address.to_string(), locked);
        self.jail_if_below_min(address, released_supply);
        Ok(())
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
    ///
    /// **Bootstrap rule:** kul active stake 0 ho (kisi ne stake nahi kiya
    /// — sirf dev/legacy genesis chain) to kisi ek active validator ka
    /// approve vote hi quorum mana jata hai. Aisi chain me sirf genesis
    /// validator hota hai, isliye uska khud ka vote hi kaafi hai. Jaise
    /// hi koi stake karta hai (total > 0), ye rule band ho jata hai aur
    /// sakht 2/3 quorum lagta hai.
    pub fn has_quorum_for(&self, block_hash: &str, set: &ValidatorSet) -> bool {
        let total = set.total_active_stake();
        if total == 0 {
            // Bootstrap: koi stake nahi hai — ek active validator ka
            // approve vote hi kaafi hai.
            return self.votes.iter().any(|v| {
                v.block_hash == block_hash
                    && v.approve
                    && set.active_validators().contains(&v.voter)
            });
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

    /// Tests me released supply 1,000,000 bits (1000 BUT) mani gayi hai —
    /// minimum stake uska 1% = 10,000 bits (10 BUT) hai.
    const TEST_RELEASED: u64 = 1_000_000;
    /// Har test validator ka stake — 100 BUT (minimum se zyada).
    const STAKE: u64 = 100 * BITS_PER_BUT;

    fn staked_set() -> (ValidatorSet, Vec<Wallet>) {
        // 4 validators, har ek ka stake 100 BUT = kul 400 BUT.
        let wallets: Vec<Wallet> = (1..=4).map(|i| wallet(i)).collect();
        let mut set = ValidatorSet::single(wallets[0].address.clone()).unwrap();
        set.restake(&wallets[0].address, STAKE, TEST_RELEASED)
            .unwrap();
        for w in &wallets[1..] {
            set.add_staked_validator(w.address.clone(), STAKE, TEST_RELEASED)
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
        let min = min_stake_for(TEST_RELEASED);
        // Minimum se 1 kam — reject hona chahiye.
        assert!(set
            .add_staked_validator(newcomer.address.clone(), min - 1, TEST_RELEASED)
            .is_err());
        // Bilkul minimum — ok.
        assert!(set
            .add_staked_validator(newcomer.address.clone(), min, TEST_RELEASED)
            .is_ok());
        let _ = wallets;
    }

    #[test]
    fn min_stake_released_supply_ka_ek_percent_hai() {
        // Released 1,000,000 bits (1000 BUT) → minimum 10,000 bits (10 BUT).
        assert_eq!(min_stake_for(1_000_000), 10_000);
        // Released 100,000,000 bits (100,000 BUT) → minimum 1,000,000 bits.
        assert_eq!(min_stake_for(100_000_000), 1_000_000);
        // Released 500,000,000,000 bits (500M BUT) → minimum 5,000,000,000 bits.
        assert_eq!(min_stake_for(500_000_000_000), 5_000_000_000);
        // Supply 0 ho to bhi minimum 1 bit (0-stake validator nahi).
        assert_eq!(min_stake_for(0), 1);
    }

    #[test]
    fn duplicate_validator_reject() {
        let (mut set, wallets) = staked_set();
        assert!(set
            .add_staked_validator(wallets[1].address.clone(), STAKE, TEST_RELEASED)
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
        weighted
            .restake(&wallets[0].address, STAKE * 3, TEST_RELEASED)
            .unwrap();
        for w in &wallets[1..] {
            weighted
                .add_staked_validator(w.address.clone(), STAKE, TEST_RELEASED)
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

        // Slash: 10% kata — stake ka 10%.
        let cut = set
            .slash(&wallets[0].address, "equivocation", TEST_RELEASED)
            .unwrap();
        assert_eq!(cut, STAKE / 10);
        assert_eq!(set.stake_of(&wallets[0].address), STAKE - cut);
        assert_eq!(set.slashed_total[&wallets[0].address], STAKE / 10);
        // 90,000 abhi bhi minimum (10,000) se zyada hai — jail nahi hoga.
        assert!(!set.is_jailed(&wallets[0].address));
        // Jail tab hota hai jab stake minimum se neeche gire: alag set me
        // minimum jitna hi stake karke slash karo.
        let min = min_stake_for(TEST_RELEASED);
        let mut small = ValidatorSet::single(wallets[1].address.clone()).unwrap();
        small
            .restake(&wallets[1].address, min, TEST_RELEASED)
            .unwrap();
        let cut2 = small
            .slash(&wallets[1].address, "equivocation", TEST_RELEASED)
            .unwrap();
        assert_eq!(cut2, min / 10);
        assert!(small.is_jailed(&wallets[1].address));
        assert!(small.authorize_block(&wallets[1].address).is_err());
        // Restake karke unjail ho sakta hai.
        small
            .restake(&wallets[1].address, min, TEST_RELEASED)
            .unwrap();
        assert!(!small.is_jailed(&wallets[1].address));
        assert!(small.authorize_block(&wallets[1].address).is_ok());
    }

    #[test]
    fn slash_bina_stake_wale_poa_validator_ko_jail_karta_hai() {
        let v = wallet(9);
        let mut set = ValidatorSet::single(v.address.clone()).unwrap();
        let cut = set.slash(&v.address, "galat block", TEST_RELEASED).unwrap();
        assert_eq!(cut, 0); // kaatne ko stake hi nahi tha
        assert!(set.is_jailed(&v.address));
    }

    #[test]
    fn block_reward_placeholder_schedule() {
        // Shuru me 5 BUT (5,000 bits) per block.
        assert_eq!(block_reward_for(0, 1_000_000), 5_000);
        assert_eq!(block_reward_for(1, 1_000_000), 5_000);
        // Halving interval ke baad aadha — 2.5 BUT.
        assert_eq!(
            block_reward_for(REWARD_HALVING_INTERVAL_BLOCKS, 1_000_000),
            2_500
        );
        // Do halvings ke baad 1.25 BUT.
        assert_eq!(
            block_reward_for(2 * REWARD_HALVING_INTERVAL_BLOCKS, 1_000_000),
            1_250
        );
        // Supply cap tak pahunch jaye to reward 0 (endgame — sirf fees).
        assert_eq!(
            block_reward_for(0, MAX_SUPPLY_BUT * BITS_PER_BUT),
            0
        );
        // Cap se thoda kam release ho to sirf baaki hissa mint hoga.
        let almost = MAX_SUPPLY_BUT * BITS_PER_BUT - 100;
        assert_eq!(block_reward_for(0, almost), 100);
    }

    #[test]
    fn team_share_reward_ka_zero_point_one_percent_hai() {
        // 5,000 bits reward → team ko 5 bits (0.1%).
        assert_eq!(team_share_of(5_000), 5);
        assert_eq!(team_share_of(1_000_000), 1_000);
        assert_eq!(team_share_of(999), 0); // 1 bit se kam — 0 (integer division)
    }

    #[test]
    fn bootstrap_quorum_stake_zero_par_self_vote_se() {
        // Kisi ne stake nahi kiya — kul stake 0. Ek hi validator hai.
        let v = wallet(9);
        let set = ValidatorSet::single(v.address.clone()).unwrap();
        let hash = "a".repeat(64);
        // Bina vote ke quorum nahi.
        let empty = VoteTally::new(1);
        assert!(!empty.has_quorum_for(&hash, &set));
        // Validator ka khud ka approve vote — bootstrap quorum HAAN.
        let mut tally = VoteTally::new(1);
        let vote = Vote::new(
            hash.clone(),
            1,
            true,
            v.address.clone(),
            &v.signing_key(),
        )
        .unwrap();
        tally.add_vote(vote, &set).unwrap();
        assert!(tally.has_quorum_for(&hash, &set));
        // Reject vote se quorum nahi.
        let mut tally2 = VoteTally::new(1);
        let vote2 = Vote::new(
            hash.clone(),
            1,
            false,
            v.address.clone(),
            &v.signing_key(),
        )
        .unwrap();
        tally2.add_vote(vote2, &set).unwrap();
        assert!(!tally2.has_quorum_for(&hash, &set));
    }
}
