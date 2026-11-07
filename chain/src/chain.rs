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
//! 4. Validator authorized hona chahiye (Phase 1: PoA, consensus.rs dekho).
//! 5. Block ki structure sahi honi chahiye (hash, Merkle root, validator sig).
//! 6. Har transaction: signature sahi, nonce sahi (replay rokta hai),
//!    balance kaafi (amount + fee).
//! 7. Fee validator ko milti hai.

use crate::block::{Block, GENESIS_PREV_HASH};
use crate::butpay::ButPayRegistry;
use crate::consensus::ValidatorSet;
use crate::error::{ButError, Result};
use crate::transaction::Transaction;
use crate::wallet::{validate_address, Wallet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Poori chain — blocks + current state (balances, nonces).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// Saare blocks, genesis (index 0) se shuru.
    pub blocks: Vec<Block>,
    /// Har address ka current balance (smallest unit).
    pub balances: HashMap<String, u64>,
    /// Har address ka agla nonce (pehli tx ka nonce 0 hota hai).
    pub nonces: HashMap<String, u64>,
    /// Authorized validators (Phase 1: PoA).
    pub validators: ValidatorSet,
    /// Genesis allocations — shuru ka balance (address, amount).
    /// Chain verify karte waqt inhi se state rebuild hoti hai.
    pub genesis_allocations: Vec<(String, u64)>,
}

impl Chain {
    /// Nayi chain banao — genesis block ke saath.
    /// `allocations`: shuru me kisko kitna balance milega.
    /// `timestamp`: genesis block ka time (seconds since epoch).
    pub fn new_genesis(
        allocations: Vec<(String, u64)>,
        validator: &Wallet,
        timestamp: u64,
    ) -> Result<Self> {
        for (address, amount) in &allocations {
            validate_address(address)?;
            if *amount == 0 {
                return Err(ButError::InvalidTransaction(
                    "genesis allocation 0 se zyada honi chahiye".to_string(),
                ));
            }
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
        for (address, amount) in &allocations {
            *balances.entry(address.clone()).or_insert(0) += *amount;
        }
        Ok(Self {
            blocks: vec![genesis],
            balances,
            nonces: HashMap::new(),
            validators,
            genesis_allocations: allocations,
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

    /// Kisi address ka balance (na ho to 0).
    pub fn balance(&self, address: &str) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    /// Kisi address ka agla nonce (na ho to 0 — matlab pehli tx ka nonce 0).
    pub fn next_nonce(&self, address: &str) -> u64 {
        self.nonces.get(address).copied().unwrap_or(0)
    }

    /// Block ko chain me jodo — pehle POORI validation, phir commit.
    /// Galat block hua to chain bilkul nahi badalti.
    /// (BUT Pay ID wali transaction isme reject hogi — uske liye
    /// `add_block_with_registry` use karo, kyunki receiver resolve
    /// karne ke liye registry chahiye hoti hai.)
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        self.add_block_with_registry(block, &ButPayRegistry::new())
    }

    /// Registry ke saath block jodo — BUT Pay ID/handle wali transaction
    /// bhi chalegi: receiver registry se address me badal kar credit hota
    /// hai. Signature hamesha original payload (ID string) par verify
    /// hoti hai.
    pub fn add_block_with_registry(
        &mut self,
        block: Block,
        registry: &ButPayRegistry,
    ) -> Result<()> {
        let prev = self.blocks.last().ok_or_else(|| {
            ButError::InvalidChain("chain me ek bhi block nahi hai".to_string())
        })?;
        self.validate_block_against(
            &block,
            prev,
            &self.balances.clone(),
            &self.nonces.clone(),
            registry,
        )
        .map(|(balances, nonces)| {
            self.balances = balances;
            self.nonces = nonces;
            self.blocks.push(block);
        })
    }

    /// BFT tareeke se block jodo (Phase 2) — pehle quorum + proposer check,
    /// phir wahi poori validation jo `add_block` karta hai.
    /// Rules (koi ek bhi toota to block reject):
    /// 1. Vote tally ki height block ki height se match karni chahiye.
    /// 2. Block banane wala is height ka sahi proposer hona chahiye
    ///    (`ValidatorSet::proposer_for_round`).
    /// 3. Block ko kul active stake ke 2/3 se zyada approvals mile hon
    ///    (`VoteTally::has_quorum_for`).
    /// Note: ye opt-in path hai — purana `add_block` abhi bhi PoA tareeke
    /// se chalta hai. Har block ke liye vote zaroori karna (BFT-only chain)
    /// abhi baaki hai — uske liye P2P par vote gossip chahiye.
    pub fn add_block_bft(
        &mut self,
        block: Block,
        tally: &crate::consensus::VoteTally,
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
        self.add_block(block)
    }

    /// BFT block jodo + BUT Pay registry — ID/handle wali transaction
    /// bhi chalegi.
    pub fn add_block_bft_with_registry(
        &mut self,
        block: Block,
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
        self.add_block_with_registry(block, registry)
    }

    /// Block validate karo aur nayi state banao (bina commit kiye).
    /// Yehi function `verify_chain` bhi use karta hai.
    fn validate_block_against(
        &self,
        block: &Block,
        prev: &Block,
        balances: &HashMap<String, u64>,
        nonces: &HashMap<String, u64>,
        registry: &ButPayRegistry,
    ) -> Result<(HashMap<String, u64>, HashMap<String, u64>)> {
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
        // 4. Validator authorized hona chahiye.
        self.validators.authorize_block(&block.validator)?;
        // 5. Block structure (hash, Merkle root, validator signature).
        block.verify_structure()?;
        // 6. Har transaction, order me, state ki copy par apply karo.
        let mut balances = balances.clone();
        let mut nonces = nonces.clone();
        for tx in &block.transactions {
            self.apply_transaction(tx, &block.validator, &mut balances, &mut nonces, registry)?;
        }
        Ok((balances, nonces))
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
        self.validators.authorize_block(&genesis.validator)?;
        genesis.verify_structure()?;
        // State genesis allocations se shuru karo.
        let mut balances: HashMap<String, u64> = HashMap::new();
        for (address, amount) in &self.genesis_allocations {
            *balances.entry(address.clone()).or_insert(0) += *amount;
        }
        let mut nonces: HashMap<String, u64> = HashMap::new();
        // Har agla block validate karo.
        for i in 1..self.blocks.len() {
            let (prev, block) = (&self.blocks[i - 1], &self.blocks[i]);
            let (b, n) = self.validate_block_against(block, prev, &balances, &nonces, registry)?;
            balances = b;
            nonces = n;
        }
        // Rebuilt state stored state se match karni chahiye.
        if balances != self.balances || nonces != self.nonces {
            return Err(ButError::InvalidChain(
                "stored balances chain se match nahi karte — state badli gayi hai".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mempool::Mempool;

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

    fn make_block(
        chain: &Chain,
        validator: &Wallet,
        txs: Vec<Transaction>,
        timestamp: u64,
    ) -> Block {
        Block::new(
            chain.height() as u64,
            timestamp,
            chain.latest_hash(),
            txs,
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap()
    }

    #[test]
    fn genesis_aur_transfer_kaam_karta_hai() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        assert_eq!(chain.balance(&alice.address), 1_000_000);
        assert_eq!(chain.balance(&bob.address), 0);
        chain.verify_chain().unwrap();

        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 1, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = make_block(&chain, &validator, vec![tx], 1_700_000_001);
        chain.add_block(block).unwrap();
        assert_eq!(chain.balance(&alice.address), 1_000_000 - 101);
        assert_eq!(chain.balance(&bob.address), 100);
        assert_eq!(chain.balance(&validator.address), 1); // fee
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
        let mut tx = Transaction::new(alice.address.clone(), "100001".to_string(), 100, 1, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = make_block(&chain, &validator, vec![tx], 1_700_000_001);
        // Plain add_block reject karega (registry nahi hai).
        assert!(chain.add_block(block.clone()).is_err());
        // Registry ke saath jud jayega — credit Bob ke address ko milega.
        chain.add_block_with_registry(block, &reg).unwrap();
        assert_eq!(chain.balance(&alice.address), 1_000_000 - 101);
        assert_eq!(chain.balance(&bob.address), 100);
        chain.verify_chain_with_registry(&reg).unwrap();
        // Handle se bhi transfer chalega.
        let mut tx2 = Transaction::new(alice.address.clone(), "@bob".to_string(), 50, 0, 1);
        tx2.sign(&alice.signing_key()).unwrap();
        let block2 = make_block(&chain, &validator, vec![tx2], 1_700_000_002);
        chain.add_block_with_registry(block2, &reg).unwrap();
        assert_eq!(chain.balance(&bob.address), 150);
        chain.verify_chain_with_registry(&reg).unwrap();
    }

    #[test]
    fn galat_signature_wala_block_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        tx.amount = 500; // sign ke baad amount badla — signature toot gayi
        let block = make_block(&chain, &validator, vec![tx], 1_700_000_001);
        assert!(chain.add_block(block).is_err());
        assert_eq!(chain.height(), 1); // chain nahi badli
    }

    #[test]
    fn balance_kam_ho_to_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut tx = Transaction::new(
            alice.address.clone(),
            bob.address.clone(),
            2_000_000,
            0,
            0,
        );
        tx.sign(&alice.signing_key()).unwrap();
        let block = make_block(&chain, &validator, vec![tx], 1_700_000_001);
        assert!(chain.add_block(block).is_err());
    }

    #[test]
    fn replay_attack_reject_nonce_se() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = make_block(&chain, &validator, vec![tx.clone()], 1_700_000_001);
        chain.add_block(block).unwrap();
        // Wahi tx dobara — nonce ab 1 hai, tx ka nonce 0 hai
        let block2 = make_block(&chain, &validator, vec![tx], 1_700_000_002);
        assert!(chain.add_block(block2).is_err());
    }

    #[test]
    fn double_spend_ek_block_me_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut tx1 = Transaction::new(alice.address.clone(), bob.address.clone(), 600_000, 0, 0);
        tx1.sign(&alice.signing_key()).unwrap();
        let mut tx2 = Transaction::new(alice.address.clone(), bob.address.clone(), 600_000, 0, 1);
        tx2.sign(&alice.signing_key()).unwrap();
        // Dono milakar 1,200,000 > balance 1,000,000
        let block = make_block(&chain, &validator, vec![tx1, tx2], 1_700_000_001);
        assert!(chain.add_block(block).is_err());
    }

    #[test]
    fn tampered_block_verify_fail() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = make_block(&chain, &validator, vec![tx], 1_700_000_001);
        chain.add_block(block).unwrap();
        // Block badlo — amount badla, hash ab galat hai
        chain.blocks[1].transactions[0].amount = 999;
        assert!(chain.verify_chain().is_err());
    }

    #[test]
    fn galat_validator_ka_block_reject() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let other = test_wallet(8); // authorized nahi hai
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            1,
            1_700_000_001,
            chain.latest_hash(),
            vec![tx],
            other.address.clone(),
            &other.signing_key(),
        )
        .unwrap();
        assert!(chain.add_block(block).is_err());
        let _ = validator;
    }

    #[test]
    fn mempool_se_block_banana() {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let mut pool = Mempool::new();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 50, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        pool.add(tx).unwrap();
        let txs = pool.take_for_block(10);
        let block = make_block(&chain, &validator, txs, 1_700_000_001);
        chain.add_block(block).unwrap();
        assert_eq!(chain.balance(&bob.address), 50);
        chain.verify_chain().unwrap();
    }

    // ---- Phase 2: BFT block add ----

    use crate::consensus::{Vote, VoteTally, MIN_STAKE};

    /// 4 staked validators wali chain: genesis validator (w9) ko bhi stake
    /// diya jata hai, aur 3 aur validators jode jate hain. Kul stake 400 BUT.
    fn bft_chain() -> (Chain, Vec<Wallet>, Wallet, Wallet) {
        let (mut chain, validator, alice, bob) = genesis_chain();
        let extra: Vec<Wallet> = [8u8, 7, 6].iter().map(|s| test_wallet(*s)).collect();
        chain
            .validators
            .restake(&validator.address, MIN_STAKE)
            .unwrap();
        for w in &extra {
            chain
                .validators
                .add_staked_validator(w.address.clone(), MIN_STAKE)
                .unwrap();
        }
        let mut all = vec![validator];
        all.extend(extra);
        (chain, all, alice, bob)
    }

    /// Height 1 ke liye block banao — sahi proposer se — aur `n` validators
    /// ke approve votes ke saath tally banao.
    fn bft_block_and_tally(
        chain: &Chain,
        validators: &[Wallet],
        alice: &Wallet,
        bob: &Wallet,
        approvals: usize,
    ) -> (Block, VoteTally) {
        let proposer_addr = chain.validators.proposer_for_round(1).unwrap();
        let proposer = validators
            .iter()
            .find(|w| w.address == proposer_addr)
            .unwrap();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            1,
            1_700_000_001,
            chain.latest_hash(),
            vec![tx],
            proposer.address.clone(),
            &proposer.signing_key(),
        )
        .unwrap();
        let mut tally = VoteTally::new(1);
        for w in validators.iter().take(approvals) {
            let vote = Vote::new(
                block.hash.clone(),
                1,
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
        assert_eq!(chain.height(), 2);
        assert_eq!(chain.balance(&bob.address), 100);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn bft_block_bina_quorum_reject() {
        let (mut chain, validators, alice, bob) = bft_chain();
        // Sirf 2 approvals (200/400) — 2/3 quorum nahi.
        let (block, tally) = bft_block_and_tally(&chain, &validators, &alice, &bob, 2);
        assert!(chain.add_block_bft(block, &tally).is_err());
        assert_eq!(chain.height(), 1);
    }

    #[test]
    fn bft_block_galat_proposer_reject() {
        let (mut chain, validators, alice, bob) = bft_chain();
        let proposer_addr = chain.validators.proposer_for_round(1).unwrap();
        // Proposer ke alawa koi aur validator block banaye.
        let other = validators
            .iter()
            .find(|w| w.address != proposer_addr)
            .unwrap();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            1,
            1_700_000_001,
            chain.latest_hash(),
            vec![tx],
            other.address.clone(),
            &other.signing_key(),
        )
        .unwrap();
        // Is block par 3 votes bhi hon to proposer galat hone se reject.
        let mut tally = VoteTally::new(1);
        for w in validators.iter().take(3) {
            let vote = Vote::new(
                block.hash.clone(),
                1,
                true,
                w.address.clone(),
                &w.signing_key(),
            )
            .unwrap();
            tally.add_vote(vote, &chain.validators).unwrap();
        }
        assert!(chain.add_block_bft(block, &tally).is_err());
        assert_eq!(chain.height(), 1);
    }
}
