//! Mempool — pending transactions ki list.
//!
//! User transaction bhejta hai → pehle mempool me aati hai → validator
//! usme se block banata hai → chain me judti hai. Mempool sirf signature
//! aur format check karta hai; balance aur nonce ki final checking
//! `Chain::add_block` karta hai (kyunki wahi current state janta hai).

use crate::butpay::ButPayRegistry;
use crate::error::{ButError, Result};
use crate::slash::SlashOp;
use crate::staking::StakeOp;
use crate::swap::SwapOp;
use crate::transaction::Transaction;

/// Pending transactions ka pool.
/// Phase 3a se staking operations ka pool bhi yahin hai — stake op bhi
/// signed hota hai aur block me jata hai, isliye uska pending pool bhi
/// yahin rehta hai. Phase 3b se swap aur slash ops ka pool bhi yahin hai.
#[derive(Debug, Default)]
pub struct Mempool {
    /// Transaction ID se transaction — duplicate tx dobara nahi aayegi.
    pending: Vec<Transaction>,
    /// Pending staking operations (stake/unstake/claim).
    pending_stake_ops: Vec<StakeOp>,
    /// Pending swap operations (swap/add-liquidity).
    pending_swap_ops: Vec<SwapOp>,
    /// Pending slash operations (equivocation report).
    pending_slash_ops: Vec<SlashOp>,
}

impl Mempool {
    /// Khaali mempool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Kitni pending transactions hain.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Khaali hai ya nahi.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Transaction mempool me dalo.
    /// Check: signature sahi honi chahiye (public key transaction me hi
    /// hoti hai), aur wahi transaction pehle se pending nahi honi chahiye.
    pub fn add(&mut self, tx: Transaction) -> Result<()> {
        tx.verify()?;
        self.add_checked(tx)
    }

    /// BUT Pay receiver wali transaction mempool me dalo.
    /// Receiver ID (jaise "100001") ya handle (jaise "@sanjay") ho sakta
    /// hai — registry se resolve karke verify hota hai. Signature ID wali
    /// payload par hi check hoti hai (jaisi user ne sign ki thi).
    pub fn add_with_registry(&mut self, tx: Transaction, registry: &ButPayRegistry) -> Result<()> {
        tx.verify_with_registry(registry)?;
        self.add_checked(tx)
    }

    /// Receiver pehle se resolve karke transaction mempool me dalo.
    /// `resolved_receiver` hamesha valid address hona chahiye.
    pub fn add_resolved(&mut self, tx: Transaction, resolved_receiver: &str) -> Result<()> {
        tx.verify_with_resolved_receiver(resolved_receiver)?;
        self.add_checked(tx)
    }

    /// Duplicate check + push (verify ke baad).
    fn add_checked(&mut self, tx: Transaction) -> Result<()> {
        if self.pending.iter().any(|t| t.id() == tx.id()) {
            return Err(ButError::InvalidTransaction(
                "ye transaction pehle se mempool me hai".to_string(),
            ));
        }
        self.pending.push(tx);
        Ok(())
    }

    /// Block banane ke liye `limit` transactions nikalo (pehle aayi, pehle).
    /// Nikali hui transactions mempool se hat jati hain.
    pub fn take_for_block(&mut self, limit: usize) -> Vec<Transaction> {
        let n = limit.min(self.pending.len());
        self.pending.drain(..n).collect()
    }

    /// Saari pending transactions dekho (bina hataye).
    pub fn list(&self) -> &[Transaction] {
        &self.pending
    }

    /// Staking operation mempool me dalo.
    /// Check: signature sahi honi chahiye, aur wahi op pehle se pending
    /// nahi hona chahiye. Balance/nonce ki final checking `Chain` karta hai.
    pub fn add_stake_op(&mut self, op: StakeOp) -> Result<()> {
        op.verify()?;
        if self.pending_stake_ops.iter().any(|o| o.id() == op.id()) {
            return Err(ButError::InvalidTransaction(
                "ye stake op pehle se mempool me hai".to_string(),
            ));
        }
        self.pending_stake_ops.push(op);
        Ok(())
    }

    /// Kitni pending stake ops hain.
    pub fn stake_ops_len(&self) -> usize {
        self.pending_stake_ops.len()
    }

    /// Block banane ke liye `limit` stake ops nikalo (pehle aayi, pehle).
    pub fn take_stake_ops_for_block(&mut self, limit: usize) -> Vec<StakeOp> {
        let n = limit.min(self.pending_stake_ops.len());
        self.pending_stake_ops.drain(..n).collect()
    }

    /// Saari pending stake ops dekho (bina hataye).
    pub fn stake_ops(&self) -> &[StakeOp] {
        &self.pending_stake_ops
    }

    /// Swap operation mempool me dalo.
    /// Check: signature + format + routing rules sahi hone chahiye, aur
    /// wahi op pehle se pending nahi hona chahiye. Balance/nonce/pool ki
    /// final checking `Chain` karta hai (block apply karte waqt).
    pub fn add_swap_op(&mut self, op: SwapOp) -> Result<()> {
        op.verify()?;
        if self.pending_swap_ops.iter().any(|o| o.id() == op.id()) {
            return Err(ButError::InvalidTransaction(
                "ye swap op pehle se mempool me hai".to_string(),
            ));
        }
        self.pending_swap_ops.push(op);
        Ok(())
    }

    /// Kitni pending swap ops hain.
    pub fn swap_ops_len(&self) -> usize {
        self.pending_swap_ops.len()
    }

    /// Block banane ke liye `limit` swap ops nikalo (pehle aayi, pehle).
    pub fn take_swap_ops_for_block(&mut self, limit: usize) -> Vec<SwapOp> {
        let n = limit.min(self.pending_swap_ops.len());
        self.pending_swap_ops.drain(..n).collect()
    }

    /// Saari pending swap ops dekho (bina hataye).
    pub fn swap_ops(&self) -> &[SwapOp] {
        &self.pending_swap_ops
    }

    /// Slash operation mempool me dalo.
    /// Check: saboot + signature sahi hone chahiye, aur wahi op pehle se
    /// pending nahi hona chahiye. Nonce aur dedup ki final checking
    /// `Chain` karta hai (block apply karte waqt).
    pub fn add_slash_op(&mut self, op: SlashOp) -> Result<()> {
        op.verify()?;
        if self.pending_slash_ops.iter().any(|o| o.id() == op.id()) {
            return Err(ButError::InvalidTransaction(
                "ye slash op pehle se mempool me hai".to_string(),
            ));
        }
        self.pending_slash_ops.push(op);
        Ok(())
    }

    /// Kitni pending slash ops hain.
    pub fn slash_ops_len(&self) -> usize {
        self.pending_slash_ops.len()
    }

    /// Block banane ke liye `limit` slash ops nikalo (pehle aayi, pehle).
    pub fn take_slash_ops_for_block(&mut self, limit: usize) -> Vec<SlashOp> {
        let n = limit.min(self.pending_slash_ops.len());
        self.pending_slash_ops.drain(..n).collect()
    }

    /// Saari pending slash ops dekho (bina hataye).
    pub fn slash_ops(&self) -> &[SlashOp] {
        &self.pending_slash_ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    #[test]
    fn add_verify_take() {
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let mut pool = Mempool::new();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 10, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        pool.add(tx.clone()).unwrap();
        assert_eq!(pool.len(), 1);
        // Duplicate reject
        assert!(pool.add(tx).is_err());
        let taken = pool.take_for_block(10);
        assert_eq!(taken.len(), 1);
        assert!(pool.is_empty());
    }

    #[test]
    fn bina_signature_mempool_reject() {
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let mut pool = Mempool::new();
        let tx = Transaction::new(alice.address.clone(), bob.address.clone(), 10, 0, 0);
        assert!(pool.add(tx).is_err());
    }

    #[test]
    fn id_receiver_mempool_me_aati_hai() {
        use crate::butpay::ButPayRegistry;
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let mut reg = ButPayRegistry::new();
        reg.register(&bob.address, Some("bob")).unwrap();
        let mut pool = Mempool::new();
        // ID receiver wali tx — plain add() reject karega.
        let mut tx = Transaction::new(alice.address.clone(), "100001".to_string(), 10, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        assert!(pool.add(tx.clone()).is_err());
        // Registry ke saath add hogi.
        pool.add_with_registry(tx, &reg).unwrap();
        assert_eq!(pool.len(), 1);
        // Galat ID wali tx reject.
        let mut bad = Transaction::new(alice.address.clone(), "999999".to_string(), 10, 0, 1);
        bad.sign(&alice.signing_key()).unwrap();
        assert!(pool.add_with_registry(bad, &reg).is_err());
    }

    #[test]
    fn swap_op_mempool_me_aati_hai_duplicate_reject() {
        use crate::swap::{SwapAsset, SwapOp};
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let mut pool = Mempool::new();
        let mut op = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            1_000,
            0,
            alice.address.clone(),
        );
        op.sign(&alice.signing_key()).unwrap();
        pool.add_swap_op(op.clone()).unwrap();
        assert_eq!(pool.swap_ops_len(), 1);
        // Duplicate reject.
        assert!(pool.add_swap_op(op).is_err());
        let taken = pool.take_swap_ops_for_block(10);
        assert_eq!(taken.len(), 1);
        assert_eq!(pool.swap_ops_len(), 0);
    }

    #[test]
    fn swap_op_btc_se_butcoin_mempool_reject() {
        use crate::swap::{SwapAsset, SwapOp};
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let mut pool = Mempool::new();
        let mut op = SwapOp::swap(
            SwapAsset::External("BTC".to_string()),
            SwapAsset::ButCoin,
            1_000,
            0,
            alice.address.clone(),
        );
        op.sign(&alice.signing_key()).unwrap();
        assert!(pool.add_swap_op(op).is_err());
    }

    #[test]
    fn slash_op_mempool_me_aati_hai() {
        use crate::consensus::Vote;
        use crate::slash::SlashOp;
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
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
        let mut op = SlashOp::new(
            alice.address.clone(),
            v1,
            v2,
            "equivocation".to_string(),
            bob.address.clone(),
            0,
        );
        op.sign(&bob.signing_key()).unwrap();
        let mut pool = Mempool::new();
        pool.add_slash_op(op.clone()).unwrap();
        assert_eq!(pool.slash_ops_len(), 1);
        assert!(pool.add_slash_op(op).is_err());
        let taken = pool.take_slash_ops_for_block(10);
        assert_eq!(taken.len(), 1);
        assert_eq!(pool.slash_ops_len(), 0);
    }
}
