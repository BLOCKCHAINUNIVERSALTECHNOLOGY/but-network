//! Real chain — blocks ki list, balances, aur poori verification.
//!
//! Model (simple aur sahi, account-based — Bitcoin ke UTXO jaisa complex nahi):
//! - Har address ka balance aur nonce chain state me rehta hai.
//! - Genesis block me koi transaction nahi hoti; shuru ka balance
//!   `genesis_allocations` se aata hai (jaise Bitcoin ke genesis reward).
//!   Ye allocations chain me save hoti hain, taaki koi bhi node chain ko
//!   scratch se rebuild karke verify kar sake.
//! - Har block add hone se pehle POORI tarah validate hota hai — pehle
//!   state ki copy par, galat block hua to asli state nahi badalti.
//! - Amounts "smallest unit" me hain (u64). 1 BUT = kitne smallest unit,
//!   ye tokenomics ka faisla abhi baaki hai — isliye yahan sirf raw units
//!   use ho rahe hain, koi BUT-amount ka daava nahi.
//!
//! Block accept karne ke rules (koi ek bhi toota to block reject):
//! 1. index pichle block se 1 zyada hona chahiye.
//! 2. prev_hash pichle block ka hash hona chahiye.
//! 3. timestamp pichle block se pehle ka nahi hona chahiye.
//! 4. **BFT-only (Phase 3a)**: block banane wala is height ka sahi proposer
//!    hona chahiye aur block ko 2/3 quorum mila hona chahiye (votes block
//!    me hi hote hain). Purana PoA `add_block` hata diya gaya hai.
//! 5. Block ki structure sahi honi chahiye (hash, Merkle root, validator sig).
//! 6. Har transaction: signature sahi, nonce sahi (replay rokta hai),
//!    **spendable** balance kaafi (amount + fee) — locked coins kharch nahi ho sakte.
//! 7. Har stake op: signature sahi, nonce sahi, stake ke liye balance kaafi,
//!    unstake ke liye lock khatm hona chahiye.
//! 8. Fee validator ko milti hai; **block reward** (placeholder schedule)
//!    proposer ko + uska 0.1% team ko milta hai; released supply badhti hai.

use crate::block::{Block, GENESIS_PREV_HASH};
use crate::butpay::ButPayRegistry;
use crate::consensus::{block_reward_for, min_stake_for, team_share_of, ValidatorSet};
use crate::error::{ButError, Result};
use crate::slash::SlashOp;
use crate::staking::{StakeOp, StakeOpKind, StakingState};
use crate::swap::{SwapAsset, SwapOp, SwapOpKind, SwapState};
use crate::transaction::Transaction;
use crate::wallet::{validate_address, Wallet};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Block validate karke bani hui nayi state — `validate_block_against`
/// ka return. Ye poori state hai jo block judne par commit hoti hai.
/// Validator set bhi isme hai — stake ops usse bhi badalte hain (lock
/// badha to stake record badha, lock ghata to jail check), isliye wo bhi
/// copy par validate hota hai, stored state par nahi.
#[derive(Debug, Clone)]
pub struct ValidatedState {
    pub balances: HashMap<String, u64>,
    pub butcoin_balances: HashMap<String, u64>,
    pub nonces: HashMap<String, u64>,
    pub staking: StakingState,
    pub validators: ValidatorSet,
    pub swap: SwapState,
    pub applied_slash_ops: HashSet<String>,
    pub released_supply: u64,
    pub team_accrued: u64,
}

/// Poori chain — blocks + current state (balances, nonces, staking).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// Saare blocks, genesis (index 0) se shuru.
    pub blocks: Vec<Block>,
    /// Har address ka current **spendable** balance (smallest unit).
    /// Locked (staked) coins isme nahi hote — wo `staking.locks` me hain.
    pub balances: HashMap<String, u64>,
    /// Har address ka agla nonce (pehli tx ka nonce 0 hota hai).
    /// Transfer aur stake op ek hi nonce sequence share karte hain.
    pub nonces: HashMap<String, u64>,
    /// Authorized validators (Phase 3a: BFT).
    pub validators: ValidatorSet,
    /// Genesis allocations — shuru ka balance (address, amount).
    /// Chain verify karte waqt inhi se state rebuild hoti hai.
    pub genesis_allocations: Vec<(String, u64)>,
    /// Genesis BUT Coin allocations — shuru ka BUT Coin balance
    /// (address, amount, smallest units). Verify-rebuild ke liye.
    #[serde(default)]
    pub genesis_butcoin_allocations: Vec<(String, u64)>,
    /// Har address ka BUT Coin balance (smallest units, 1 BUT Coin = 1,000).
    /// BUT Coin ke liye alag map hai — BUT ke balances se kabhi mix nahi hota.
    #[serde(default)]
    pub butcoin_balances: HashMap<String, u64>,
    /// Swap pools — BUT ka alag pool + BUT Coin ka alag pool (§13).
    /// Verify ke waqt blocks se rebuild hota hai (genesis par dono 0).
    #[serde(default)]
    pub swap: SwapState,
    /// Ab tak apply hue slash ops ke **saboot (evidence) IDs** — ek saboot
    /// par dobara slash na ho, chahe reporter naya nonce lekar aaye.
    /// Verify ke waqt blocks se rebuild hota hai.
    #[serde(default)]
    pub applied_slash_ops: HashSet<String>,
    /// Staking state — locks + unbonding (Phase 3a).
    #[serde(default)]
    pub staking: StakingState,
    /// Ab tak release hui supply (bits) — genesis allocations + ab tak
    /// ke block rewards. Minimum stake isi ka 1% hota hai (FINAL).
    #[serde(default)]
    pub released_supply: u64,
    /// Team ka account — sirf genesis par set hota hai (None = abhi set
    /// nahi hua; tab team share `team_accrued` me ikattha hota hai).
    #[serde(default)]
    pub team_account: Option<String>,
    /// Team ka ikattha hua hissa (bits) — jab tak team account set nahi
    /// hota, har block reward ka 0.1% yahan jama hota rehta hai.
    #[serde(default)]
    pub team_accrued: u64,
}

impl Chain {
    /// Nayi chain banao — genesis block ke saath.
    /// `allocations`: shuru me kisko kitna balance milega.
    /// `timestamp`: genesis block ka time (seconds since epoch).
    /// Team account set nahi hota (None) — team share `team_accrued` me
    /// jama hota rahega. Team account ke saath genesis ke liye
    /// `new_genesis_with_team` use karo.
    pub fn new_genesis(
        allocations: Vec<(String, u64)>,
        validator: &Wallet,
        timestamp: u64,
    ) -> Result<Self> {
        Self::new_genesis_with_team(allocations, validator, timestamp, None)
    }

    /// Nayi chain banao — team account ke saath.
    /// Team account sirf genesis par set hota hai (baad me badla nahi
    /// jata — wo chain upgrade ka kaam hoga). Har block reward ka 0.1%
    /// isi account ko jayega.
    pub fn new_genesis_with_team(
        allocations: Vec<(String, u64)>,
        validator: &Wallet,
        timestamp: u64,
        team_account: Option<String>,
    ) -> Result<Self> {
        Self::new_genesis_full(allocations, Vec::new(), validator, timestamp, team_account)
    }

    /// Nayi chain banao — BUT + BUT Coin allocations ke saath (Phase 3b).
    /// BUT Coin allocations smallest units me hain (1 BUT Coin = 1,000).
    /// Pools genesis par khaali hote hain (0) — liquidity AddLiquidity
    /// op se dalni padti hai.
    pub fn new_genesis_with_butcoin(
        allocations: Vec<(String, u64)>,
        butcoin_allocations: Vec<(String, u64)>,
        validator: &Wallet,
        timestamp: u64,
    ) -> Result<Self> {
        Self::new_genesis_full(allocations, butcoin_allocations, validator, timestamp, None)
    }

    /// Poora genesis — BUT allocations + BUT Coin allocations + team account.
    /// `released_supply` sirf BUT ki hoti hai (BUT Coin alag coin hai,
    /// uski koi max supply nahi — §13).
    pub fn new_genesis_full(
        allocations: Vec<(String, u64)>,
        butcoin_allocations: Vec<(String, u64)>,
        validator: &Wallet,
        timestamp: u64,
        team_account: Option<String>,
    ) -> Result<Self> {
        for (address, amount) in &allocations {
            validate_address(address)?;
            if *amount == 0 {
                return Err(ButError::InvalidTransaction(
                    "genesis allocation 0 se zyada honi chahiye".to_string(),
                ));
            }
        }
        for (address, amount) in &butcoin_allocations {
            validate_address(address)?;
            if *amount == 0 {
                return Err(ButError::InvalidTransaction(
                    "genesis BUT Coin allocation 0 se zyada honi chahiye".to_string(),
                ));
            }
        }
        if let Some(team) = &team_account {
            validate_address(team)?;
        }
        let validators = ValidatorSet::single(validator.address.clone())?;
        let genesis = Block::new(
            0,
            timestamp,
            GENESIS_PREV_HASH.to_string(),
            Vec::new(),
            validator.address.clone(),
            &validator.signing_key(),
        )?;
        let mut balances: HashMap<String, u64> = HashMap::new();
        let mut released_supply: u64 = 0;
        for (address, amount) in &allocations {
            *balances.entry(address.clone()).or_insert(0) += *amount;
            released_supply += *amount;
        }
        let mut butcoin_balances: HashMap<String, u64> = HashMap::new();
        for (address, amount) in &butcoin_allocations {
            *butcoin_balances.entry(address.clone()).or_insert(0) += *amount;
        }
        Ok(Self {
            blocks: vec![genesis],
            balances,
            butcoin_balances,
            nonces: HashMap::new(),
            validators,
            genesis_allocations: allocations,
            genesis_butcoin_allocations: butcoin_allocations,
            swap: SwapState::new(),
            applied_slash_ops: HashSet::new(),
            staking: StakingState::new(),
            released_supply,
            team_account,
            team_accrued: 0,
        })
    }

    /// Chain ki height — kul kitne blocks hain.
    pub fn height(&self) -> usize {
        self.blocks.len()
    }

    /// Aakhri block ka hash.
    pub fn latest_hash(&self) -> String {
        self.blocks
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_default()
    }

    /// Kisi address ka **spendable** balance (na ho to 0).
    /// Locked (staked) coins isme nahi hote.
    pub fn balance(&self, address: &str) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    /// Kisi address ka BUT Coin balance (smallest units, na ho to 0).
    /// 1 BUT Coin = 1,000 smallest units.
    pub fn butcoin_balance(&self, address: &str) -> u64 {
        self.butcoin_balances.get(address).copied().unwrap_or(0)
    }

    /// Kisi address ke locked (staked) coins.
    pub fn locked_balance(&self, address: &str) -> u64 {
        self.staking.locked_of(address)
    }

    /// Kisi address ke unbonding me pade coins (abhi release nahi hue).
    pub fn unbonding_balance(&self, address: &str) -> u64 {
        self.staking.unbonding_of(address)
    }

    /// Kisi address ke abhi claim karne layak coins.
    pub fn claimable_balance(&self, address: &str, now: u64) -> u64 {
        self.staking.claimable_of(address, now)
    }

    /// Kisi address ka kul balance — spendable + locked + unbonding.
    pub fn total_balance(&self, address: &str) -> u64 {
        self.balance(address)
            .saturating_add(self.locked_balance(address))
            .saturating_add(self.unbonding_balance(address))
    }

    /// Ab tak release hui supply (bits).
    pub fn released_supply(&self) -> u64 {
        self.released_supply
    }

    /// Is waqt minimum stake (bits) — released supply ka 1% (FINAL).
    pub fn min_stake(&self) -> u64 {
        min_stake_for(self.released_supply)
    }

    /// Agle block ka reward (bits) — placeholder schedule ke hisaab se.
    pub fn next_block_reward(&self) -> u64 {
        block_reward_for(self.height() as u64, self.released_supply)
    }

    /// Kisi address ka agla nonce (na ho to 0 — matlab pehli tx ka nonce 0).
    pub fn next_nonce(&self, address: &str) -> u64 {
        self.nonces.get(address).copied().unwrap_or(0)
    }

    /// BFT tareeke se block jodo (Phase 3a — **ekmatra** rasta; purana PoA
    /// `add_block` hata diya gaya hai).
    /// Rules (koi ek bhi toota to block reject):
    /// 1. Vote tally ki height block ki height se match karni chahiye.
    /// 2. Block banane wala is height ka sahi proposer hona chahiye
    ///    (`ValidatorSet::proposer_for_round`).
    /// 3. Block ko kul active stake ke 2/3 se zyada approvals mile hon
    ///    (`VoteTally::has_quorum_for`) — stake 0 ho to bootstrap rule
    ///    (ek active validator ka self-vote).
    /// 4. Quorum milne par votes block me embed hokar save hote hain
    ///    (taaki sync karne wala node bhi quorum check kar sake).
    /// (BUT Pay ID wali transaction isme reject hogi — uske liye
    /// `add_block_bft_with_registry` use karo.)
    pub fn add_block_bft(
        &mut self,
        block: Block,
        tally: &crate::consensus::VoteTally,
    ) -> Result<()> {
        self.add_block_bft_with_registry(block, tally, &ButPayRegistry::new())
    }

    /// BFT block jodo + BUT Pay registry — ID/handle wali transaction
    /// bhi chalegi.
    pub fn add_block_bft_with_registry(
        &mut self,
        mut block: Block,
        tally: &crate::consensus::VoteTally,
        registry: &ButPayRegistry,
    ) -> Result<()> {
        if block.index == 0 {
            return Err(ButError::InvalidBlock(
                "genesis block BFT se nahi judta".to_string(),
            ));
        }
        if tally.height != block.index {
            return Err(ButError::InvalidBlock(format!(
                "vote tally ki height ({}) block ki height ({}) se match nahi karti",
                tally.height, block.index
            )));
        }
        match self.validators.proposer_for_round(block.index) {
            Some(expected) if expected == block.validator => {}
            Some(expected) => {
                return Err(ButError::InvalidBlock(format!(
                    "is height ka proposer {expected} hona chahiye tha, block { } ne banaya",
                    block.validator
                )))
            }
            None => {
                return Err(ButError::InvalidBlock(
                    "koi active validator nahi hai — proposer chuna nahi ja sakta".to_string(),
                ))
            }
        }
        if !tally.has_quorum_for(&block.hash, &self.validators) {
            return Err(ButError::InvalidBlock(
                "block ko 2/3 quorum nahi mila — BFT vote poora nahi hai".to_string(),
            ));
        }
        // Votes block me embed karo — sync karne wala node bhi inhi se
        // quorum check karega. Votes hash me nahi hote (wo hash par sign
        // hote hain), isliye embed karne se hash nahi badalta.
        block.votes = tally.votes.clone();
        self.add_validated_block(block, registry)
    }

    /// Block ko chain me jodo — pehle POORI validation, phir commit.
    /// Galat block hua to chain bilkul nahi badalti. Ye internal hai —
    /// bahar se sirf `add_block_bft*` use hota hai (BFT-only chain).
    fn add_validated_block(&mut self, block: Block, registry: &ButPayRegistry) -> Result<()> {
        let prev = self.blocks.last().ok_or_else(|| {
            ButError::InvalidChain("chain me ek bhi block nahi hai".to_string())
        })?;
        let validated = self.validate_block_against(
            &block,
            prev,
            &self.balances.clone(),
            &self.butcoin_balances.clone(),
            &self.nonces.clone(),
            &self.staking.clone(),
            &self.validators.clone(),
            &self.swap.clone(),
            &self.applied_slash_ops.clone(),
            self.released_supply,
            self.team_accrued,
            registry,
        )?;
        self.balances = validated.balances;
        self.butcoin_balances = validated.butcoin_balances;
        self.nonces = validated.nonces;
        self.staking = validated.staking;
        self.validators = validated.validators;
        self.swap = validated.swap;
        self.applied_slash_ops = validated.applied_slash_ops;
        self.released_supply = validated.released_supply;
        self.team_accrued = validated.team_accrued;
        self.blocks.push(block);
        Ok(())
    }

    /// Block validate karo aur nayi state banao (bina commit kiye).
    /// Yehi function `verify_chain` bhi use karta hai.
    /// Order: pehle saari transactions, phir saare stake ops, phir saare
    /// swap ops, phir saare slash ops, phir block reward mint. Reward
    /// height aur released supply se deterministic hai — isliye rebuild me
    /// wahi reward milega. Saare ops ek hi nonce sequence share karte hain.
    fn validate_block_against(
        &self,
        block: &Block,
        prev: &Block,
        balances: &HashMap<String, u64>,
        butcoin_balances: &HashMap<String, u64>,
        nonces: &HashMap<String, u64>,
        staking: &StakingState,
        validators: &ValidatorSet,
        swap: &SwapState,
        applied_slash_ops: &HashSet<String>,
        released_supply: u64,
        team_accrued: u64,
        registry: &ButPayRegistry,
    ) -> Result<ValidatedState> {
        // 1. Index sequence.
        if block.index != prev.index + 1 {
            return Err(ButError::InvalidBlock(format!(
                "block index galat hai: mila {}, chahiye {}",
                block.index,
                prev.index + 1
            )));
        }
        // 2. Prev hash link.
        if block.prev_hash != prev.hash {
            return Err(ButError::InvalidBlock(
                "prev_hash pichle block se match nahi karta — chain tooti hui hai".to_string(),
            ));
        }
        // 3. Timestamp peeche nahi ja sakta.
        if block.timestamp < prev.timestamp {
            return Err(ButError::InvalidBlock(
                "block ka timestamp pichle block se pehle ka hai".to_string(),
            ));
        }
        // 4. Validator authorized hona chahiye — BLOCK SE PEHLE wali
        // state ke hisaab se (parameter `validators`), `self` ke hisaab
        // se nahi. Verify-rebuild me `self` poori chain ki final state
        // hoti hai: jo validator baad me jail hua ho, uska purana block
        // phir bhi valid tha (proposer tab authorized tha).
        validators.authorize_block(&block.validator)?;
        // 5. Block structure (hash, Merkle root, validator signature).
        block.verify_structure()?;
        // 6. Har transaction, order me, state ki copy par apply karo.
        let mut balances = balances.clone();
        let mut butcoin_balances = butcoin_balances.clone();
        let mut nonces = nonces.clone();
        let mut staking = staking.clone();
        let mut validators = validators.clone();
        let mut swap = swap.clone();
        let mut applied_slash_ops = applied_slash_ops.clone();
        let mut released_supply = released_supply;
        let mut team_accrued = team_accrued;
        for tx in &block.transactions {
            self.apply_transaction(tx, &block.validator, &mut balances, &mut nonces, registry)?;
        }
        // 7. Har stake op, order me — nonce sequence tx ke saath shared hai.
        for op in &block.stake_ops {
            Self::apply_stake_op(
                op,
                block.timestamp,
                &mut balances,
                &mut nonces,
                &mut staking,
                &mut validators,
                released_supply,
            )?;
        }
        // 7b. Har swap op, order me — nonce sequence shared hai.
        for op in &block.swap_ops {
            Self::apply_swap_op(
                op,
                &mut balances,
                &mut butcoin_balances,
                &mut nonces,
                &mut swap,
            )?;
        }
        // 7c. Har slash op, order me — nonce sequence shared hai.
        for op in &block.slash_ops {
            Self::apply_slash_op(
                op,
                &mut nonces,
                &mut staking,
                &mut validators,
                &mut applied_slash_ops,
                released_supply,
            )?;
        }
        // 8. Block reward mint karo — proposer ko + 0.1% team ko.
        // Reward deterministic hai (height + released supply se).
        let reward = block_reward_for(block.index, released_supply);
        if reward > 0 {
            let team_share = team_share_of(reward);
            let proposer_share = reward - team_share;
            *balances
                .entry(block.validator.clone())
                .or_insert(0) += proposer_share;
            match &self.team_account {
                Some(team) => {
                    *balances.entry(team.clone()).or_insert(0) += team_share;
                }
                None => {
                    team_accrued += team_share;
                }
            }
            released_supply += reward;
        }
        Ok(ValidatedState {
            balances,
            butcoin_balances,
            nonces,
            staking,
            validators,
            swap,
            applied_slash_ops,
            released_supply,
            team_accrued,
        })
    }

    /// Ek swap op ko state par apply karo (validation ke saath).
    /// Galat op hua to error — state nahi badalti (caller copy use karta hai).
    /// §11: swap kabhi fail nahi hota liquidity ki kami se — output AMM
    /// formula se milta hai (0 bhi ho sakta hai), input poora pool me jata
    /// hai, koi IOU nahi. Balance kam ho to hi reject (input hi nahi hai).
    fn apply_swap_op(
        op: &SwapOp,
        balances: &mut HashMap<String, u64>,
        butcoin_balances: &mut HashMap<String, u64>,
        nonces: &mut HashMap<String, u64>,
        swap: &mut SwapState,
    ) -> Result<()> {
        // Signature + format + routing rules check.
        op.verify()?;
        // Nonce check — transfer/stake op ke saath ek hi sequence.
        let expected_nonce = nonces.get(&op.swapper).copied().unwrap_or(0);
        if op.nonce != expected_nonce {
            return Err(ButError::InvalidTransaction(format!(
                "swap op nonce galat hai: mila {}, chahiye {} (replay ya out-of-order op)",
                op.nonce, expected_nonce
            )));
        }
        match op.kind {
            SwapOpKind::AddLiquidity => {
                // Jis coin ki liquidity dalni hai, uska balance kaafi hona chahiye.
                match op.from {
                    SwapAsset::But => {
                        let spendable = balances.get(&op.swapper).copied().unwrap_or(0);
                        if spendable < op.amount {
                            return Err(ButError::InvalidTransaction(format!(
                                "liquidity ke liye BUT balance kam hai: chahiye {}, hai {}",
                                op.amount, spendable
                            )));
                        }
                        *balances.entry(op.swapper.clone()).or_insert(0) -= op.amount;
                        swap.but_pool = swap.but_pool.saturating_add(op.amount);
                    }
                    SwapAsset::ButCoin => {
                        let held = butcoin_balances.get(&op.swapper).copied().unwrap_or(0);
                        if held < op.amount {
                            return Err(ButError::InvalidTransaction(format!(
                                "liquidity ke liye BUT Coin balance kam hai: chahiye {}, hai {}",
                                op.amount, held
                            )));
                        }
                        *butcoin_balances.entry(op.swapper.clone()).or_insert(0) -= op.amount;
                        swap.butcoin_pool = swap.butcoin_pool.saturating_add(op.amount);
                    }
                    SwapAsset::External(_) => {
                        // verify() pehle hi reject kar chuka hai — yahan nahi aana chahiye.
                        return Err(ButError::InvalidTransaction(
                            "bahar ke coin ki liquidity on-chain pool me nahi dal sakte".to_string(),
                        ));
                    }
                }
            }
            SwapOpKind::Swap => {
                // AMM x·y=k — output quote jaisa hi (is waqt ke reserves se).
                let (reserve_in, reserve_out) = match (&op.from, &op.to) {
                    (SwapAsset::But, SwapAsset::ButCoin) => (swap.but_pool, swap.butcoin_pool),
                    (SwapAsset::ButCoin, SwapAsset::But) => (swap.butcoin_pool, swap.but_pool),
                    _ => {
                        // verify() pehle hi external/same-coin reject kar chuka hai.
                        return Err(ButError::InvalidTransaction(
                            "swap sirf BUT ↔ BUT Coin ke beech hota hai".to_string(),
                        ));
                    }
                };
                let out = SwapState::amm_out(op.amount, reserve_in, reserve_out);
                // Input balance check — input hi na ho to reject.
                match op.from {
                    SwapAsset::But => {
                        let spendable = balances.get(&op.swapper).copied().unwrap_or(0);
                        if spendable < op.amount {
                            return Err(ButError::InvalidTransaction(format!(
                                "swap ke liye BUT balance kam hai: chahiye {}, hai {}",
                                op.amount, spendable
                            )));
                        }
                        *balances.entry(op.swapper.clone()).or_insert(0) -= op.amount;
                        swap.but_pool = swap.but_pool.saturating_add(op.amount);
                        // Output BUT Coin milega (0 bhi ho sakta hai — §11, fail nahi).
                        if out > 0 {
                            *butcoin_balances.entry(op.swapper.clone()).or_insert(0) += out;
                            swap.butcoin_pool -= out;
                        }
                    }
                    SwapAsset::ButCoin => {
                        let held = butcoin_balances.get(&op.swapper).copied().unwrap_or(0);
                        if held < op.amount {
                            return Err(ButError::InvalidTransaction(format!(
                                "swap ke liye BUT Coin balance kam hai: chahiye {}, hai {}",
                                op.amount, held
                            )));
                        }
                        *butcoin_balances.entry(op.swapper.clone()).or_insert(0) -= op.amount;
                        swap.butcoin_pool = swap.butcoin_pool.saturating_add(op.amount);
                        if out > 0 {
                            *balances.entry(op.swapper.clone()).or_insert(0) += out;
                            swap.but_pool -= out;
                        }
                    }
                    SwapAsset::External(_) => {
                        return Err(ButError::InvalidTransaction(
                            "bahar ke coin ka swap on-chain nahi hota".to_string(),
                        ));
                    }
                }
            }
        }
        *nonces.entry(op.swapper.clone()).or_insert(0) += 1;
        Ok(())
    }

    /// Ek slash op ko state par apply karo (validation ke saath).
    /// Galat op hua to error — state nahi badalti (caller copy use karta hai).
    /// Saboot verify hota hai (do conflicting votes), reporter ka nonce
    /// check hota hai, wahi saboot dobara aaye to reject hota hai.
    /// Cut: target ke stake ka 10% (record + lock dono se).
    /// Returns: kitna stake kata (bits).
    fn apply_slash_op(
        op: &SlashOp,
        nonces: &mut HashMap<String, u64>,
        staking: &mut StakingState,
        validators: &mut ValidatorSet,
        applied_slash_ops: &mut HashSet<String>,
        released_supply: u64,
    ) -> Result<u64> {
        // Saboot + format + signature check.
        op.verify()?;
        // Reporter ka nonce check.
        let expected_nonce = nonces.get(&op.reporter).copied().unwrap_or(0);
        if op.nonce != expected_nonce {
            return Err(ButError::InvalidTransaction(format!(
                "slash op nonce galat hai: mila {}, chahiye {} (replay ya out-of-order op)",
                op.nonce, expected_nonce
            )));
        }
        // Ek saboot par ek hi baar slash — dedup **evidence ID** se hota
        // hai (op ID me nonce shamil hai; reporter naya nonce lekar wahi
        // saboot dobara bheje to bhi reject hona chahiye).
        let evidence_id = op.evidence_id();
        if applied_slash_ops.contains(&evidence_id) {
            return Err(ButError::InvalidTransaction(
                "is saboot par pehle hi slash ho chuka hai — dobara slash nahi hoga".to_string(),
            ));
        }
        // Target validator hona chahiye.
        if !validators.is_validator(&op.target) {
            return Err(ButError::InvalidTransaction(format!(
                "slash: ye validator nahi hai: {}",
                op.target
            )));
        }
        // Cut: stake ka 10% — record aur lock dono se.
        let cut = validators.slash(&op.target, &op.reason, released_supply)?;
        let _ = staking.slash_lock(&op.target, cut); // lock me kam hon to kam kat te hain
        applied_slash_ops.insert(evidence_id);
        *nonces.entry(op.reporter.clone()).or_insert(0) += 1;
        Ok(cut)
    }

    /// Ek stake op ko state par apply karo (validation ke saath).
    /// Galat op hua to error — state nahi badalti (caller copy use karta hai).
    /// Saare arguments state ki copy hain — stored state yahan nahi badalti.
    /// Validator ka stake record hamesha lock ke barabar rakha jata hai:
    /// stake badha to record badha (unjail bhi ho sakta hai), unstake ke
    /// baad lock minimum se neeche gira to jail.
    fn apply_stake_op(
        op: &StakeOp,
        block_timestamp: u64,
        balances: &mut HashMap<String, u64>,
        nonces: &mut HashMap<String, u64>,
        staking: &mut StakingState,
        validators: &mut ValidatorSet,
        released_supply: u64,
    ) -> Result<()> {
        // Signature + format check.
        op.verify()?;
        // Nonce check — transfer ke saath ek hi sequence (replay rokta hai).
        let expected_nonce = nonces.get(&op.staker).copied().unwrap_or(0);
        if op.nonce != expected_nonce {
            return Err(ButError::InvalidTransaction(format!(
                "stake op nonce galat hai: mila {}, chahiye {} (replay ya out-of-order op)",
                op.nonce, expected_nonce
            )));
        }
        match op.kind {
            StakeOpKind::Stake => {
                // Spendable balance kaafi honi chahiye — locked coins
                // isme nahi hote, isliye wo kharch nahi ho sakte.
                let spendable = balances.get(&op.staker).copied().unwrap_or(0);
                if spendable < op.amount {
                    return Err(ButError::InvalidTransaction(format!(
                        "stake ke liye spendable balance kam hai: chahiye {}, hai {} (locked coins kharch nahi ho sakte)",
                        op.amount, spendable
                    )));
                }
                let tier = op.tier.ok_or_else(|| {
                    ButError::InvalidTransaction("stake ke liye lock tier zaroori hai".to_string())
                })?;
                *balances.entry(op.staker.clone()).or_insert(0) -= op.amount;
                staking.stake(&op.staker, op.amount, tier, block_timestamp)?;
                // Validator hai to uska stake record bhi utna hi badhao.
                if validators.is_validator(&op.staker) {
                    validators.restake(&op.staker, op.amount, released_supply)?;
                }
            }
            StakeOpKind::Unstake => {
                staking.start_unbonding(&op.staker, op.amount, block_timestamp)?;
                // Validator ka locked kam hua — record sync karo; minimum
                // se neeche gira to jail.
                if validators.is_validator(&op.staker) {
                    let locked = staking.locked_of(&op.staker);
                    validators.sync_stake(&op.staker, locked, released_supply)?;
                }
            }
            StakeOpKind::Claim => {
                let claimed = staking.claim(&op.staker, block_timestamp);
                if claimed > 0 {
                    *balances.entry(op.staker.clone()).or_insert(0) += claimed;
                }
            }
            StakeOpKind::Register => {
                // Locked stake ke hisaab se validator register karo.
                // Stake record locked amount ke barabar hota hai.
                let locked = staking.locked_of(&op.staker);
                validators.add_staked_validator(op.staker.clone(), locked, released_supply)?;
            }
        }
        *nonces.entry(op.staker.clone()).or_insert(0) += 1;
        Ok(())
    }

    /// Ek transaction ko state par apply karo (validation ke saath).
    /// Galat tx hui to error — state nahi badalti (caller copy use karta hai).
    /// BUT Pay ID/handle receiver ho to registry se address nikala jata
    /// hai aur credit usi address ko milta hai.
    fn apply_transaction(
        &self,
        tx: &Transaction,
        validator: &str,
        balances: &mut HashMap<String, u64>,
        nonces: &mut HashMap<String, u64>,
        registry: &ButPayRegistry,
    ) -> Result<()> {
        // Signature + format check (ID/handle receiver registry se resolve hota hai).
        tx.verify_with_registry(registry)?;
        let receiver = registry.resolve_for_tx(&tx.receiver)?;
        // Nonce check — replay attack rokta hai.
        let expected_nonce = nonces.get(&tx.sender).copied().unwrap_or(0);
        if tx.nonce != expected_nonce {
            return Err(ButError::InvalidTransaction(format!(
                "nonce galat hai: mila {}, chahiye {} (replay ya out-of-order tx)",
                tx.nonce, expected_nonce
            )));
        }
        // Balance check — amount + fee dono cover hone chahiye.
        let total = tx
            .amount
            .checked_add(tx.fee)
            .ok_or_else(|| ButError::InvalidTransaction("amount + fee overflow".to_string()))?;
        let sender_balance = balances.get(&tx.sender).copied().unwrap_or(0);
        if sender_balance < total {
            return Err(ButError::InvalidTransaction(format!(
                "balance kam hai: chahiye {total}, hai {sender_balance}"
            )));
        }
        // Apply: sender se katao, receiver ko do, fee validator ko.
        *balances.entry(tx.sender.clone()).or_insert(0) -= total;
        *balances.entry(receiver).or_insert(0) += tx.amount;
        if tx.fee > 0 {
            *balances.entry(validator.to_string()).or_insert(0) += tx.fee;
        }
        *nonces.entry(tx.sender.clone()).or_insert(0) += 1;
        Ok(())
    }

    /// Poori chain scratch se verify karo — genesis se lekar aakhri block tak.
    /// Tampered block, galat signature, ya galat balance — kuch bhi hua to error.
    /// (BUT Pay ID wali transaction isme reject hogi — uske liye
    /// `verify_chain_with_registry` use karo.)
    pub fn verify_chain(&self) -> Result<()> {
        self.verify_chain_with_registry(&ButPayRegistry::new())
    }

    /// Registry ke saath poori chain verify karo — BUT Pay ID/handle
    /// wali transaction bhi chalegi (receiver resolve karke).
    /// Rebuild genesis allocations + blocks se hota hai — balances,
    /// BUT Coin balances, nonces, staking, validator set, swap pools,
    /// applied slash ops, released supply aur team accrued, sab stored
    /// state se match karne chahiye. Kuch bhi badla mila to error.
    /// (Phase 3b: slashing ab block-level SlashOp se hoti hai — isliye
    /// slash ke baad bhi verify pass hota hai. Seedha state badalne wala
    /// purana `slash_validator` hata diya gaya hai.)
    pub fn verify_chain_with_registry(&self, registry: &ButPayRegistry) -> Result<()> {
        if self.blocks.is_empty() {
            return Err(ButError::InvalidChain("chain khaali hai".to_string()));
        }
        // Genesis check.
        let genesis = &self.blocks[0];
        if genesis.index != 0 || genesis.prev_hash != GENESIS_PREV_HASH {
            return Err(ButError::InvalidChain(
                "genesis block sahi nahi hai".to_string(),
            ));
        }
        if !genesis.transactions.is_empty() {
            return Err(ButError::InvalidChain(
                "genesis block me transaction nahi honi chahiye".to_string(),
            ));
        }
        // Genesis validator sirf validator LIST me hona chahiye — active /
        // unjailed hona zaroori nahi. Wo genesis ke waqt authorized tha;
        // baad me slash hokar jail hua ho to bhi genesis block valid tha.
        // (Phase 3b fix: pehle yahan `authorize_block` tha, jo baad me
        // jail hue genesis validator par verify fail kar deta tha.)
        if !self.validators.is_validator(&genesis.validator) {
            return Err(ButError::InvalidChain(format!(
                "genesis validator validator list me nahi hai: {}",
                genesis.validator
            )));
        }
        genesis.verify_structure()?;
        // State genesis allocations se shuru karo.
        let mut balances: HashMap<String, u64> = HashMap::new();
        let mut released_supply: u64 = 0;
        for (address, amount) in &self.genesis_allocations {
            *balances.entry(address.clone()).or_insert(0) += *amount;
            released_supply += *amount;
        }
        let mut butcoin_balances: HashMap<String, u64> = HashMap::new();
        for (address, amount) in &self.genesis_butcoin_allocations {
            *butcoin_balances.entry(address.clone()).or_insert(0) += *amount;
        }
        let mut nonces: HashMap<String, u64> = HashMap::new();
        let mut staking = StakingState::new();
        let mut validators = ValidatorSet::single(self.blocks[0].validator.clone())?;
        let mut swap = SwapState::new();
        let mut applied_slash_ops: HashSet<String> = HashSet::new();
        let mut team_accrued: u64 = 0;
        // Har agla block validate karo.
        for i in 1..self.blocks.len() {
            let (prev, block) = (&self.blocks[i - 1], &self.blocks[i]);
            let validated = self.validate_block_against(
                block,
                prev,
                &balances,
                &butcoin_balances,
                &nonces,
                &staking,
                &validators,
                &swap,
                &applied_slash_ops,
                released_supply,
                team_accrued,
                registry,
            )?;
            balances = validated.balances;
            butcoin_balances = validated.butcoin_balances;
            nonces = validated.nonces;
            staking = validated.staking;
            validators = validated.validators;
            swap = validated.swap;
            applied_slash_ops = validated.applied_slash_ops;
            released_supply = validated.released_supply;
            team_accrued = validated.team_accrued;
        }
        // Rebuilt state stored state se match karni chahiye.
        // (ValidatorSet me `slashed_total` rebuild nahi hota — wo sirf
        // history hai, isliye usse compare nahi karte. Baaki sab match
        // karna chahiye: validators list, stakes, jailed.)
        if balances != self.balances || nonces != self.nonces {
            return Err(ButError::InvalidChain(
                "stored balances chain se match nahi karte — state badli gayi hai".to_string(),
            ));
        }
        if butcoin_balances != self.butcoin_balances {
            return Err(ButError::InvalidChain(
                "stored BUT Coin balances chain se match nahi karte — state badli gayi hai".to_string(),
            ));
        }
        if swap != self.swap {
            return Err(ButError::InvalidChain(
                "stored swap pools chain se match nahi karte — state badli gayi hai".to_string(),
            ));
        }
        if applied_slash_ops != self.applied_slash_ops {
            return Err(ButError::InvalidChain(
                "stored slash history chain se match nahi karti — state badli gayi hai".to_string(),
            ));
        }
        if staking != self.staking {
            return Err(ButError::InvalidChain(
                "stored staking state chain se match nahi karti — state badli gayi hai".to_string(),
            ));
        }
        if validators.validators != self.validators.validators
            || validators.stakes != self.validators.stakes
            || validators.jailed != self.validators.jailed
        {
            return Err(ButError::InvalidChain(
                "stored validator set chain se match nahi karta — state badli gayi hai".to_string(),
            ));
        }
        if released_supply != self.released_supply {
            return Err(ButError::InvalidChain(
                "stored released supply chain se match nahi karti — state badli gayi hai".to_string(),
            ));
        }
        if team_accrued != self.team_accrued {
            return Err(ButError::InvalidChain(
                "stored team accrued chain se match nahi karta — state badli gayi hai".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{min_stake_for, Vote, VoteTally};
    use crate::mempool::Mempool;
    use crate::staking::LockTier;

    fn test_wallet(seed_byte: u8) -> Wallet {
        Wallet::from_entropy(&[seed_byte; 32]).unwrap()
    }

    fn genesis_chain() -> (Chain, Wallet, Wallet, Wallet) {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let chain = Chain::new_genesis(
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
        )
        .unwrap();
        (chain, validator, alice, bob)
    }

    /// Test helper — BFT block banao (sahi proposer se) aur quorum votes
    /// ke saath jodo. `validators` me saare validator wallets hone chahiye.
    /// Stake 0 ho (bootstrap) to proposer ka self-vote hi kaafi hai.
    /// Returns: Ok(()) ya add karne par aaya error.
    fn bft_add(
        chain: &mut Chain,
        validators: &[Wallet],
        txs: Vec<Transaction>,
        stake_ops: Vec<StakeOp>,
        timestamp: u64,
        registry: &ButPayRegistry,
    ) -> Result<()> {
        bft_add_ops(
            chain,
            validators,
            txs,
            stake_ops,
            Vec::new(),
            Vec::new(),
            timestamp,
            registry,
        )
    }

    /// BFT block banao — txs + stake ops + swap ops + slash ops ke saath.
    fn bft_add_ops(
        chain: &mut Chain,
        validators: &[Wallet],
        txs: Vec<Transaction>,
        stake_ops: Vec<StakeOp>,
        swap_ops: Vec<SwapOp>,
        slash_ops: Vec<SlashOp>,
        timestamp: u64,
        registry: &ButPayRegistry,
    ) -> Result<()> {
        let height = chain.height() as u64;
        let proposer_addr = chain.validators.proposer_for_round(height).unwrap();
        let proposer = validators
            .iter()
            .find(|w| w.address == proposer_addr)
            .unwrap();
        let block = Block::new_with_ops(
            height,
            timestamp,
            chain.latest_hash(),
            txs,
            stake_ops,
            swap_ops,
            slash_ops,
            proposer.address.clone(),
            &proposer.signing_key(),
        )?;
        let mut tally = VoteTally::new(height);
        for w in validators {
            let vote = Vote::new(
                block.hash.clone(),
                height,
                true,
                w.address.clone(),
                &w.signing_key(),
            )?;
            let _ = tally.add_vote(vote, &chain.validators);
            if tally.has_quorum_for(&block.hash, &chain.validators) {
                break;
            }
        }
        chain.add_block_bft_with_registry(block, &tally, registry)
    }

    /// Sirf transfer txs wala block — single-validator (bootstrap) chain ke liye.
    fn bft_add_txs(
        chain: &mut Chain,
        validator: &Wallet,
        txs: Vec<Transaction>,
        timestamp: u64,
    ) -> Result<()> {
        bft_add(
            chain,
            std::slice::from_ref(validator),
            txs,
            Vec::new(),
            timestamp,
            &ButPayRegistry::new(),
        )
    }

    fn signed_tx(sender: &Wallet, receiver: &Wallet, amount: u64, fee: u64, nonce: u64) -> Transaction {
        let mut tx = Transaction::new(sender.address.clone(), receiver.address.clone(), amount, fee, nonce);
        tx.sign(&sender.signing_key()).unwrap();
        tx
    }

    #[test]
    fn genesis_aur_transfer_kaam_karta_hai() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        assert_eq!(chain.balance(&alice.address), 1_000_000);
        assert_eq!(chain.balance(&bob.address), 0);
        assert_eq!(chain.released_supply(), 1_000_000);
        chain.verify_chain().unwrap();

        let tx = signed_tx(&alice, &bob, 100, 1, 0);
        bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_001).unwrap();
        assert_eq!(chain.balance(&alice.address), 1_000_000 - 101);
        assert_eq!(chain.balance(&bob.address), 100);
        // Fee 1 + block reward 5,000 (placeholder) - team share 5 = 4,995.
        assert_eq!(chain.balance(&validator.address), 1 + 4_995);
        assert_eq!(chain.team_accrued, 5);
        assert_eq!(chain.released_supply(), 1_000_000 + 5_000);
        assert_eq!(chain.next_nonce(&alice.address), 1);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn butpay_id_se_transfer_kaam_karta_hai() {
        use crate::butpay::ButPayRegistry;
        let (mut chain, validator, alice, bob) = genesis_chain();
        // Bob ki BUT Pay ID 100001 + handle @bob register karo.
        let mut reg = ButPayRegistry::new();
        assert_eq!(reg.register(&bob.address, Some("bob")).unwrap(), 100001);
        // Alice Bob ko ID se 100 bhejti hai.
        let tx = signed_tx(&alice, &bob, 100, 1, 0);
        // NOTE: yahan tx receiver address hai — ID wala case neeche hai.
        let _ = tx;
        let mut tx_id = Transaction::new(alice.address.clone(), "100001".to_string(), 100, 1, 0);
        tx_id.sign(&alice.signing_key()).unwrap();
        // Registry ke bina (khaali registry) reject hoga.
        let res = bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            vec![tx_id.clone()],
            Vec::new(),
            1_700_000_001,
            &ButPayRegistry::new(),
        );
        assert!(res.is_err());
        // Registry ke saath jud jayega — credit Bob ke address ko milega.
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            vec![tx_id],
            Vec::new(),
            1_700_000_001,
            &reg,
        )
        .unwrap();
        assert_eq!(chain.balance(&alice.address), 1_000_000 - 101);
        assert_eq!(chain.balance(&bob.address), 100);
        chain.verify_chain_with_registry(&reg).unwrap();
        // Handle se bhi transfer chalega.
        let mut tx2 = Transaction::new(alice.address.clone(), "@bob".to_string(), 50, 0, 1);
        tx2.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            vec![tx2],
            Vec::new(),
            1_700_000_002,
            &reg,
        )
        .unwrap();
        assert_eq!(chain.balance(&bob.address), 150);
        chain.verify_chain_with_registry(&reg).unwrap();
    }

    #[test]
    fn galat_signature_wala_block_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        tx.amount = 500; // sign ke baad amount badla — signature toot gayi
        let res = bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_001);
        assert!(res.is_err());
        assert_eq!(chain.height(), 1); // chain nahi badli
    }

    #[test]
    fn balance_kam_ho_to_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let tx = signed_tx(&alice, &bob, 2_000_000, 0, 0);
        assert!(bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_001).is_err());
    }

    #[test]
    fn replay_attack_reject_nonce_se() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let tx = signed_tx(&alice, &bob, 100, 0, 0);
        bft_add_txs(&mut chain, &validator, vec![tx.clone()], 1_700_000_001).unwrap();
        // Wahi tx dobara — nonce ab 1 hai, tx ka nonce 0 hai
        assert!(bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_002).is_err());
    }

    #[test]
    fn double_spend_ek_block_me_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let tx1 = signed_tx(&alice, &bob, 600_000, 0, 0);
        let tx2 = signed_tx(&alice, &bob, 600_000, 0, 1);
        // Dono milakar 1,200,000 > balance 1,000,000
        assert!(bft_add_txs(&mut chain, &validator, vec![tx1, tx2], 1_700_000_001).is_err());
    }

    #[test]
    fn tampered_block_verify_fail() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let tx = signed_tx(&alice, &bob, 100, 0, 0);
        bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_001).unwrap();
        // Block badlo — amount badla, hash ab galat hai
        chain.blocks[1].transactions[0].amount = 999;
        assert!(chain.verify_chain().is_err());
    }

    #[test]
    fn galat_validator_ka_block_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let other = test_wallet(8); // authorized nahi hai
        let tx = signed_tx(&alice, &bob, 100, 0, 0);
        let block = Block::new(
            1,
            1_700_000_001,
            chain.latest_hash(),
            vec![tx],
            other.address.clone(),
            &other.signing_key(),
        )
        .unwrap();
        // Khud ka vote bhi ho to proposer galat hone se reject.
        let mut tally = VoteTally::new(1);
        let vote = Vote::new(
            block.hash.clone(),
            1,
            true,
            other.address.clone(),
            &other.signing_key(),
        )
        .unwrap();
        // add_vote reject karega (other active validator nahi hai) — isliye
        // khaali tally se hi add karo: proposer check pehle fail hoga.
        let _ = tally.add_vote(vote, &chain.validators);
        assert!(chain.add_block_bft(block, &tally).is_err());
        assert_eq!(chain.height(), 1);
        let _ = validator;
    }

    #[test]
    fn mempool_se_block_banana() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut pool = Mempool::new();
        let tx = signed_tx(&alice, &bob, 50, 0, 0);
        pool.add(tx).unwrap();
        let txs = pool.take_for_block(10);
        bft_add_txs(&mut chain, &validator, txs, 1_700_000_001).unwrap();
        assert_eq!(chain.balance(&bob.address), 50);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn block_bina_votes_reject_hota_hai() {
        // BFT-only: votes ke bina block nahi judta — chahe proposer sahi ho.
        let (mut chain, validator, alice, bob) = genesis_chain();
        let tx = signed_tx(&alice, &bob, 100, 0, 0);
        let block = Block::new(
            1,
            1_700_000_001,
            chain.latest_hash(),
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        let tally = VoteTally::new(1); // khaali tally — koi vote nahi
        assert!(chain.add_block_bft(block, &tally).is_err());
        assert_eq!(chain.height(), 1);
    }

    // ---- Phase 3a: stake-locking ----

    #[test]
    fn stake_lock_karta_hai_aur_locked_coins_kharch_nahi_hote() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        // Alice 100,000 bits stake karti hai (tier 90 din), nonce 0.
        let mut op = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            1_700_000_001,
            &ButPayRegistry::new(),
        )
        .unwrap();
        assert_eq!(chain.balance(&alice.address), 900_000); // spendable ghata
        assert_eq!(chain.locked_balance(&alice.address), 100_000);
        assert_eq!(chain.total_balance(&alice.address), 1_000_000);
        // Locked coins kharch nahi ho sakte: 950,000 bhejna fail (spendable 900,000).
        let tx_big = signed_tx(&alice, &bob, 950_000, 0, 1);
        assert!(bft_add_txs(&mut chain, &validator, vec![tx_big], 1_700_000_002).is_err());
        // 100 bhejna chalega.
        let tx_ok = signed_tx(&alice, &bob, 100, 0, 1);
        bft_add_txs(&mut chain, &validator, vec![tx_ok], 1_700_000_002).unwrap();
        assert_eq!(chain.balance(&bob.address), 100);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn stake_balance_kam_ho_to_reject_aur_replay_roka_jata_hai() {
        let (mut chain, validator, alice, _bob) = genesis_chain();
        // Alice ke paas 1,000,000 hain — 2,000,000 stake nahi kar sakti.
        let mut op = StakeOp::stake(alice.address.clone(), 2_000_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        let res = bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            1_700_000_001,
            &ButPayRegistry::new(),
        );
        assert!(res.is_err());
        // Sahi stake, phir wahi op dobara (nonce replay) — reject.
        let mut op2 = StakeOp::stake(alice.address.clone(), 10_000, LockTier::Days90, 0);
        op2.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op2.clone()],
            1_700_000_001,
            &ButPayRegistry::new(),
        )
        .unwrap();
        let res2 = bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op2],
            1_700_000_002,
            &ButPayRegistry::new(),
        );
        assert!(res2.is_err()); // nonce ab 1 hai, op ka nonce 0 hai
    }

    #[test]
    fn unstake_lock_khatm_hone_par_hi_aur_claim_72h_me_dheere_dheere() {
        let (mut chain, validator, alice, _bob) = genesis_chain();
        let t0 = 1_700_000_000u64;
        // Block 1: Alice 72,000 stake karti hai (90 din), nonce 0.
        let mut op = StakeOp::stake(alice.address.clone(), 72_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            t0 + 1,
            &ButPayRegistry::new(),
        )
        .unwrap();
        // Block 2: 1 din baad unstake ki koshish — lock abhi hai, reject.
        let mut op_early = StakeOp::unstake(alice.address.clone(), 72_000, 1);
        op_early.sign(&alice.signing_key()).unwrap();
        let res = bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_early],
            t0 + 86_400,
            &ButPayRegistry::new(),
        );
        assert!(res.is_err());
        // Block 2 (dobara): 90 din baad unstake — chalega. Stake block ka
        // timestamp t0+1 tha, isliye unlock bhi t0+1+90d par hota hai.
        let mut op_un = StakeOp::unstake(alice.address.clone(), 72_000, 1);
        op_un.sign(&alice.signing_key()).unwrap();
        let t_unstake = t0 + 1 + 90 * 86_400;
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_un],
            t_unstake,
            &ButPayRegistry::new(),
        )
        .unwrap();
        assert_eq!(chain.locked_balance(&alice.address), 0);
        assert_eq!(chain.unbonding_balance(&alice.address), 72_000);
        // Block 3: 36 ghante baad claim — aadha (36,000) milega.
        let mut op_claim = StakeOp::claim(alice.address.clone(), 2);
        op_claim.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_claim],
            t_unstake + 36 * 3600,
            &ButPayRegistry::new(),
        )
        .unwrap();
        assert_eq!(chain.unbonding_balance(&alice.address), 36_000);
        // Alice ka spendable: 1,000,000 - 72,000 (stake) + 36,000 (claim)
        // + block rewards (validator ko milte hain, Alice ko nahi).
        assert_eq!(chain.balance(&alice.address), 1_000_000 - 72_000 + 36_000);
        // Block 4: 72 ghante baad claim — baaki 36,000.
        let mut op_claim2 = StakeOp::claim(alice.address.clone(), 3);
        op_claim2.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_claim2],
            t_unstake + 72 * 3600,
            &ButPayRegistry::new(),
        )
        .unwrap();
        assert_eq!(chain.unbonding_balance(&alice.address), 0);
        assert_eq!(chain.balance(&alice.address), 1_000_000);
        chain.verify_chain().unwrap();
    }

    // ---- Phase 3a: block reward + team share + released supply ----

    #[test]
    fn block_reward_mint_hota_hai_team_share_accrued_me_jata_hai() {
        // Team account set nahi hai — team share team_accrued me jama hoga.
        let (mut chain, validator, alice, bob) = genesis_chain();
        assert_eq!(chain.team_accrued, 0);
        let tx = signed_tx(&alice, &bob, 100, 0, 0);
        bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_001).unwrap();
        // Reward 5,000 bits: proposer ko 4,995, team share 5 accrued.
        assert_eq!(chain.balance(&validator.address), 4_995);
        assert_eq!(chain.team_accrued, 5);
        assert_eq!(chain.released_supply(), 1_000_000 + 5_000);
        // Doosra block — reward phir 5,000.
        let tx2 = signed_tx(&alice, &bob, 100, 0, 1);
        bft_add_txs(&mut chain, &validator, vec![tx2], 1_700_000_002).unwrap();
        assert_eq!(chain.balance(&validator.address), 4_995 * 2);
        assert_eq!(chain.team_accrued, 10);
        assert_eq!(chain.released_supply(), 1_000_000 + 10_000);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn team_account_set_ho_to_team_share_seedha_milta_hai() {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let team = test_wallet(3);
        let mut chain = Chain::new_genesis_with_team(
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
            Some(team.address.clone()),
        )
        .unwrap();
        let tx = signed_tx(&alice, &bob, 100, 0, 0);
        bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_001).unwrap();
        assert_eq!(chain.balance(&team.address), 5); // team share
        assert_eq!(chain.team_accrued, 0); // accrued me kuch nahi
        assert_eq!(chain.balance(&validator.address), 4_995);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn min_stake_released_supply_ke_saath_badhta_hai() {
        let (mut chain, validator, alice, _bob) = genesis_chain();
        // Genesis released 1,000,000 bits → min stake 10,000 bits.
        assert_eq!(chain.min_stake(), 10_000);
        assert_eq!(min_stake_for(chain.released_supply()), 10_000);
        // 10 blocks mine karo — released 50,000 badhega.
        for i in 0..10u64 {
            let tx = signed_tx(&alice, &test_wallet(2), 1, 0, i);
            bft_add_txs(&mut chain, &validator, vec![tx], 1_700_000_001 + i).unwrap();
        }
        assert_eq!(chain.released_supply(), 1_000_000 + 50_000);
        assert_eq!(chain.min_stake(), 10_500);
    }

    // ---- Phase 3a: validator registration via block ----

    #[test]
    fn register_validator_lock_ke_hisaab_se_hota_hai() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        // Alice stake karti hai 100,000 bits (min 10,000 se zyada), nonce 0.
        let mut op = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            1_700_000_001,
            &ButPayRegistry::new(),
        )
        .unwrap();
        // Register op, nonce 1 — Alice validator ban jayegi.
        let mut op_reg = StakeOp::register(alice.address.clone(), 1);
        op_reg.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_reg],
            1_700_000_002,
            &ButPayRegistry::new(),
        )
        .unwrap();
        assert!(chain.validators.is_validator(&alice.address));
        assert_eq!(chain.validators.stake_of(&alice.address), 100_000);
        // Alice register ke baad proposer Alice bhi ho sakti hai (stake
        // 100,000, w9 ka stake 0) — isliye dono wallets bft_add ko do.
        let proposers = [validator.clone(), alice.clone()];
        // Dobara register — reject (pehle se validator hai).
        let mut op_reg2 = StakeOp::register(alice.address.clone(), 2);
        op_reg2.sign(&alice.signing_key()).unwrap();
        let res = bft_add(
            &mut chain,
            &proposers,
            Vec::new(),
            vec![op_reg2],
            1_700_000_003,
            &ButPayRegistry::new(),
        );
        assert!(res.is_err());
        // Bob ka lock kam hai (0) — register reject.
        let mut op_bob = StakeOp::register(bob.address.clone(), 0);
        op_bob.sign(&bob.signing_key()).unwrap();
        let res2 = bft_add(
            &mut chain,
            &proposers,
            Vec::new(),
            vec![op_bob],
            1_700_000_003,
            &ButPayRegistry::new(),
        );
        assert!(res2.is_err());
        chain.verify_chain().unwrap();
    }

    #[test]
    fn unstake_se_validator_min_se_neeche_gire_to_jail_hota_hai() {
        let (mut chain, validator, alice, _bob) = genesis_chain();
        // Alice 20,000 stake karti hai aur register hoti hai. (Exactly
        // 10,000 nahi — har block par released supply 5,000 badhti hai,
        // isliye register ke waqt min 10,000 se upar ho jata hai.)
        let mut op = StakeOp::stake(alice.address.clone(), 20_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            1_700_000_001,
            &ButPayRegistry::new(),
        )
        .unwrap();
        let mut op_reg = StakeOp::register(alice.address.clone(), 1);
        op_reg.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_reg],
            1_700_000_002,
            &ButPayRegistry::new(),
        )
        .unwrap();
        assert!(chain.validators.is_validator(&alice.address));
        assert!(!chain.validators.is_jailed(&alice.address));
        // 90 din baad poora unstake — lock 0, stake record 0 < min → jail.
        // Stake block ka timestamp 1_700_000_001 tha, unlock t0+1+90d par.
        // Proposer Alice bhi ho sakti hai — dono wallets do.
        let t_unstake = 1_700_000_001 + 90 * 86_400;
        let mut op_un = StakeOp::unstake(alice.address.clone(), 20_000, 2);
        op_un.sign(&alice.signing_key()).unwrap();
        let proposers = [validator.clone(), alice.clone()];
        bft_add(
            &mut chain,
            &proposers,
            Vec::new(),
            vec![op_un],
            t_unstake,
            &ButPayRegistry::new(),
        )
        .unwrap();
        assert!(chain.validators.is_jailed(&alice.address));
        assert!(chain.validators.authorize_block(&alice.address).is_err());
        chain.verify_chain().unwrap();
    }

    #[test]
    fn slash_op_block_me_lock_aur_record_dono_se_kaat_ta_hai() {
        // Phase 3b: slash ab block-level op se hota hai — isliye verify
        // ke baad bhi pass hota hai (purana direct slash_validator hata
        // diya gaya hai).
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut op = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            1_700_000_001,
            &ButPayRegistry::new(),
        )
        .unwrap();
        let mut op_reg = StakeOp::register(alice.address.clone(), 1);
        op_reg.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_reg],
            1_700_000_002,
            &ButPayRegistry::new(),
        )
        .unwrap();
        // Alice height 3 par do alag blocks ko vote deti hai (equivocation).
        let v1 = Vote::new(
            "a".repeat(64),
            3,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let v2 = Vote::new(
            "b".repeat(64),
            3,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        // Bob report karta hai (Bob ka nonce 0 — usne abhi koi tx nahi ki).
        let mut slash = SlashOp::new(
            alice.address.clone(),
            v1,
            v2,
            "equivocation height 3".to_string(),
            bob.address.clone(),
            0,
        );
        slash.sign(&bob.signing_key()).unwrap();
        // Slash op block me jata hai — proposer Alice bhi ho sakti hai
        // (stake 100,000), isliye dono wallets do.
        let proposers = [validator.clone(), alice.clone()];
        bft_add_ops(
            &mut chain,
            &proposers,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![slash],
            1_700_000_003,
            &ButPayRegistry::new(),
        )
        .unwrap();
        // Slash: 10% = 10,000 kata — record aur lock dono se.
        assert_eq!(chain.validators.stake_of(&alice.address), 90_000);
        assert_eq!(chain.locked_balance(&alice.address), 90_000);
        assert_eq!(chain.validators.slashed_total[&alice.address], 10_000);
        // Block-level op hai — verify PASS hona chahiye (purane direct
        // slash me yahan fail hota tha).
        chain.verify_chain().unwrap();
    }

    /// Swap ka poora flow — genesis BUT Coin, liquidity, swap, verify.
    /// Pool: BUT 100,000 + BUT Coin 500,000. Bob 50,000 BUT swap karta hai
    /// → out = 500,000 × 50,000 / (100,000 + 50,000) = 166,666.
    #[test]
    fn swap_pura_flow_liquidity_swap_verify() {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let mut chain = Chain::new_genesis_full(
            vec![
                (alice.address.clone(), 1_000_000),
                (bob.address.clone(), 500_000),
            ],
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
            None,
        )
        .unwrap();
        let registry = ButPayRegistry::new();
        // Block 1: Alice BUT liquidity 100,000 (nonce 0).
        let mut liq_but = SwapOp::add_liquidity(
            SwapAsset::But,
            100_000,
            0,
            alice.address.clone(),
        );
        liq_but.sign(&alice.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![liq_but],
            Vec::new(),
            1_700_000_001,
            &registry,
        )
        .unwrap();
        // Block 2: Alice BUT Coin liquidity 500,000 (nonce 1).
        let mut liq_coin = SwapOp::add_liquidity(
            SwapAsset::ButCoin,
            500_000,
            1,
            alice.address.clone(),
        );
        liq_coin.sign(&alice.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![liq_coin],
            Vec::new(),
            1_700_000_002,
            &registry,
        )
        .unwrap();
        assert_eq!(chain.swap.but_pool, 100_000);
        assert_eq!(chain.swap.butcoin_pool, 500_000);
        // Block 3: Bob 50,000 BUT → BUT Coin (nonce 0).
        let mut sw = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            50_000,
            0,
            bob.address.clone(),
        );
        sw.sign(&bob.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![sw],
            Vec::new(),
            1_700_000_003,
            &registry,
        )
        .unwrap();
        // Bob ko 166,666 BUT Coin mile (500,000×50,000/150,000).
        assert_eq!(chain.butcoin_balance(&bob.address), 166_666);
        assert_eq!(chain.balance(&bob.address), 500_000 - 50_000);
        assert_eq!(chain.swap.but_pool, 150_000);
        assert_eq!(chain.swap.butcoin_pool, 500_000 - 166_666);
        chain.verify_chain().unwrap();
    }

    /// §11 partial fill — bada swap, chhoti liquidity: swap FAIL nahi hota,
    /// saare input coins pool me jate hain, sirf available output milta hai.
    /// Pool: BUT 100,000 + BUT Coin 10,000. Alice 100,000 BUT swap karti hai
    /// → out = 10,000 × 100,000 / 200,000 = 5,000 (aadha pool).
    #[test]
    fn swap_partial_fill_fail_nahi_hota() {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let mut chain = Chain::new_genesis_full(
            vec![(alice.address.clone(), 1_000_000)],
            vec![(alice.address.clone(), 100_000)],
            &validator,
            1_700_000_000,
            None,
        )
        .unwrap();
        let registry = ButPayRegistry::new();
        let mut liq_but = SwapOp::add_liquidity(SwapAsset::But, 100_000, 0, alice.address.clone(),);
        liq_but.sign(&alice.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![liq_but],
            Vec::new(),
            1_700_000_001,
            &registry,
        )
        .unwrap();
        let mut liq_coin =
            SwapOp::add_liquidity(SwapAsset::ButCoin, 10_000, 1, alice.address.clone(),);
        liq_coin.sign(&alice.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![liq_coin],
            Vec::new(),
            1_700_000_002,
            &registry,
        )
        .unwrap();
        // Alice 100,000 BUT swap karti hai — output sirf 5,000 milega.
        let mut sw = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            100_000,
            2,
            alice.address.clone(),
        );
        sw.sign(&alice.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![sw],
            Vec::new(),
            1_700_000_003,
            &registry,
        )
        .unwrap();
        // Alice ke saare 100,000 BUT pool me gaye (koi IOU nahi), sirf
        // 5,000 BUT Coin mile. Genesis 100,000 − liquidity 10,000 + 5,000.
        assert_eq!(chain.balance(&alice.address), 1_000_000 - 200_000);
        assert_eq!(chain.butcoin_balance(&alice.address), 100_000 - 10_000 + 5_000);
        assert_eq!(chain.swap.but_pool, 200_000);
        assert_eq!(chain.swap.butcoin_pool, 5_000);
        chain.verify_chain().unwrap();
    }

    /// Khaali BUT Coin pool par swap — output 0, phir bhi FAIL nahi hota.
    #[test]
    fn swap_khaali_pool_par_zero_output_par_fail_nahi() {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let mut chain = Chain::new_genesis_full(
            vec![
                (alice.address.clone(), 1_000_000),
                (bob.address.clone(), 500_000),
            ],
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
            None,
        )
        .unwrap();
        let registry = ButPayRegistry::new();
        let mut liq_but = SwapOp::add_liquidity(SwapAsset::But, 100_000, 0, alice.address.clone(),);
        liq_but.sign(&alice.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![liq_but],
            Vec::new(),
            1_700_000_001,
            &registry,
        )
        .unwrap();
        // Bob swap karta hai jab BUT Coin pool khaali hai.
        let mut sw = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            10_000,
            0,
            bob.address.clone(),
        );
        sw.sign(&bob.signing_key()).unwrap();
        bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![sw],
            Vec::new(),
            1_700_000_002,
            &registry,
        )
        .unwrap();
        assert_eq!(chain.butcoin_balance(&bob.address), 0);
        assert_eq!(chain.swap.but_pool, 110_000);
        assert_eq!(chain.swap.butcoin_pool, 0);
        chain.verify_chain().unwrap();
    }

    /// §14: BTC → BUT Coin swap block me reject hota hai.
    #[test]
    fn swap_btc_se_butcoin_block_me_reject() {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let mut chain = Chain::new_genesis_full(
            vec![(alice.address.clone(), 1_000_000)],
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
            None,
        )
        .unwrap();
        let registry = ButPayRegistry::new();
        let mut sw = SwapOp::swap(
            SwapAsset::External("BTC".to_string()),
            SwapAsset::ButCoin,
            1_000,
            0,
            alice.address.clone(),
        );
        sw.sign(&alice.signing_key()).unwrap();
        let err = bft_add_ops(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            Vec::new(),
            vec![sw],
            Vec::new(),
            1_700_000_001,
            &registry,
        )
        .unwrap_err();
        assert!(err.to_string().contains("BTC"));
        chain.verify_chain().unwrap();
    }

    /// Wahi slash saboot dobara aaye to block reject hota hai.
    #[test]
    fn slash_wahi_saboot_dobara_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut op = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            1_700_000_001,
            &ButPayRegistry::new(),
        )
        .unwrap();
        let mut op_reg = StakeOp::register(alice.address.clone(), 1);
        op_reg.sign(&alice.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op_reg],
            1_700_000_002,
            &ButPayRegistry::new(),
        )
        .unwrap();
        let v1 = Vote::new(
            "a".repeat(64),
            3,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let v2 = Vote::new(
            "b".repeat(64),
            3,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        // Pehli slash op (Bob nonce 0) — block me jati hai.
        let mut slash1 = SlashOp::new(
            alice.address.clone(),
            v1.clone(),
            v2.clone(),
            "equivocation height 3".to_string(),
            bob.address.clone(),
            0,
        );
        slash1.sign(&bob.signing_key()).unwrap();
        let proposers = [validator.clone(), alice.clone()];
        bft_add_ops(
            &mut chain,
            &proposers,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![slash1],
            1_700_000_003,
            &ButPayRegistry::new(),
        )
        .unwrap();
        // Wahi saboot dobara (Bob nonce 1) — reject hona chahiye.
        let mut slash2 = SlashOp::new(
            alice.address.clone(),
            v1,
            v2,
            "equivocation height 3".to_string(),
            bob.address.clone(),
            1,
        );
        slash2.sign(&bob.signing_key()).unwrap();
        let err = bft_add_ops(
            &mut chain,
            &proposers,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![slash2],
            1_700_000_004,
            &ButPayRegistry::new(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("pehle hi slash"));
        chain.verify_chain().unwrap();
    }

    // ---- Phase 2: BFT block add (4 staked validators) ----

    /// 4 staked validators wali chain: genesis validator (w9) ko bhi stake
    /// diya jata hai, aur 3 aur validators jode jate hain. Kul stake 400 BUT.
    /// NOTE: Phase 3a me stake sirf block ops se lock hota hai — isliye
    /// yahan stake `StakeOp` se hota hai, seedha `restake` se nahi.
    fn bft_chain() -> (Chain, Vec<Wallet>, Wallet, Wallet) {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let extra: Vec<Wallet> = [8u8, 7, 6].iter().map(|s| test_wallet(*s)).collect();
        let mut all = vec![validator.clone()];
        all.extend(extra.clone());
        // Genesis me sirf Alice ko balance mila tha — validators ko pehle
        // Alice se paise bhejo, tabhi wo stake kar sakte hain.
        // Alice nonce 0: w9 ko 200,000.
        let tx0 = signed_tx(&alice, &validator, 200_000, 0, 0);
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            vec![tx0],
            Vec::new(),
            1_700_000_001,
            &ButPayRegistry::new(),
        )
        .unwrap();
        // Validator (w9) stake + register (w9 ke nonce 0, 1).
        let mut op = StakeOp::stake(validator.address.clone(), 100_000, LockTier::Days90, 0);
        op.sign(&validator.signing_key()).unwrap();
        bft_add(
            &mut chain,
            std::slice::from_ref(&validator),
            Vec::new(),
            vec![op],
            1_700_000_002,
            &ButPayRegistry::new(),
        )
        .unwrap();
        // NOTE: w9 ko Register op ki zaroorat nahi — wo genesis se hi
        // validator hai (stake 0). Stake op se uska record 100,000 ho
        // jata hai (restake arm). Register sirf naye validators ke liye hai.
        // Baaki 3 validators ko Alice se 200,000 bhejo (Alice nonce 1, 2, 3).
        let mut nonce = 1u64;
        for w in &extra {
            let tx = signed_tx(&alice, w, 200_000, 0, nonce);
            nonce += 1;
            // Proposer har height par badal sakta hai — bft_add khud sahi
            // proposer chunta hai (validators list me saare wallets hain).
            bft_add(
                &mut chain,
                &all,
                vec![tx],
                Vec::new(),
                1_700_000_003 + nonce,
                &ButPayRegistry::new(),
            )
            .unwrap();
        }
        // Ab har extra validator stake + register kare. Timestamp har
        // block par badhta hai (peeche jane par block reject hota hai).
        let mut ts = 1_700_000_010u64;
        for w in &extra {
            let mut op = StakeOp::stake(w.address.clone(), 100_000, LockTier::Days90, 0);
            op.sign(&w.signing_key()).unwrap();
            bft_add(
                &mut chain,
                &all,
                Vec::new(),
                vec![op],
                ts,
                &ButPayRegistry::new(),
            )
            .unwrap();
            ts += 1;
            let mut op_reg = StakeOp::register(w.address.clone(), 1);
            op_reg.sign(&w.signing_key()).unwrap();
            bft_add(
                &mut chain,
                &all,
                Vec::new(),
                vec![op_reg],
                ts,
                &ButPayRegistry::new(),
            )
            .unwrap();
            ts += 1;
        }
        (chain, all, alice, bob)
    }

    /// Height `h` ke liye block banao — sahi proposer se — aur `n`
    /// validators ke approve votes ke saath tally banao.
    fn bft_block_and_tally(
        chain: &Chain,
        validators: &[Wallet],
        alice: &Wallet,
        bob: &Wallet,
        approvals: usize,
    ) -> (Block, VoteTally) {
        let height = chain.height() as u64;
        let proposer_addr = chain.validators.proposer_for_round(height).unwrap();
        let proposer = validators
            .iter()
            .find(|w| w.address == proposer_addr)
            .unwrap();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        // Alice ka nonce: bft_chain me usne 4 tx ki hain (nonce 0,1,2,3).
        tx.nonce = 4;
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            height,
            1_700_000_020,
            chain.latest_hash(),
            vec![tx],
            proposer.address.clone(),
            &proposer.signing_key(),
        )
        .unwrap();
        let mut tally = VoteTally::new(height);
        for w in validators.iter().take(approvals) {
            let vote = Vote::new(
                block.hash.clone(),
                height,
                true,
                w.address.clone(),
                &w.signing_key(),
            )
            .unwrap();
            tally.add_vote(vote, &chain.validators).unwrap();
        }
        (block, tally)
    }

    #[test]
    fn bft_block_quorum_se_judta_hai() {
        let (mut chain, validators, alice, bob) = bft_chain();
        let (block, tally) = bft_block_and_tally(&chain, &validators, &alice, &bob, 3);
        chain.add_block_bft(block, &tally).unwrap();
        assert_eq!(chain.balance(&bob.address), 100);
        // Votes block me embed hue — sync karne wala node bhi check kar sakega.
        let last = chain.blocks.last().unwrap();
        assert_eq!(last.votes.len(), 3);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn bft_block_bina_quorum_reject() {
        let (mut chain, validators, alice, bob) = bft_chain();
        // Sirf 2 approvals (200/400) — 2/3 quorum nahi.
        let (block, tally) = bft_block_and_tally(&chain, &validators, &alice, &bob, 2);
        assert!(chain.add_block_bft(block, &tally).is_err());
    }

    #[test]
    fn bft_block_galat_proposer_reject() {
        let (mut chain, validators, alice, bob) = bft_chain();
        let height = chain.height() as u64;
        let proposer_addr = chain.validators.proposer_for_round(height).unwrap();
        // Proposer ke alawa koi aur validator block banaye.
        let other = validators
            .iter()
            .find(|w| w.address != proposer_addr)
            .unwrap();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.nonce = 4;
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            height,
            1_700_000_020,
            chain.latest_hash(),
            vec![tx],
            other.address.clone(),
            &other.signing_key(),
        )
        .unwrap();
        // Is block par 3 votes bhi hon to proposer galat hone se reject.
        let mut tally = VoteTally::new(height);
        for w in validators.iter().take(3) {
            let vote = Vote::new(
                block.hash.clone(),
                height,
                true,
                w.address.clone(),
                &w.signing_key(),
            )
            .unwrap();
            tally.add_vote(vote, &chain.validators).unwrap();
        }
        assert!(chain.add_block_bft(block, &tally).is_err());
    }
}
