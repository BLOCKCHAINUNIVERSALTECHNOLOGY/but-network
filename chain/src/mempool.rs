//! Mempool — pending transactions ki list.
//!
//! User transaction bhejta hai → pehle mempool me aati hai → validator
//! usme se block banata hai → chain me judti hai. Mempool sirf signature
//! aur format check karta hai; balance aur nonce ki final checking
//! `Chain::add_block` karta hai (kyunki wahi current state janta hai).

use crate::butpay::ButPayRegistry;
use crate::error::{ButError, Result};
use crate::transaction::Transaction;

/// Pending transactions ka pool.
#[derive(Debug, Default)]
pub struct Mempool {
    /// Transaction ID se transaction — duplicate tx dobara nahi aayegi.
    pending: Vec<Transaction>,
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
}
