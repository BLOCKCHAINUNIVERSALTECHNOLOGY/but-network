//! Real block — transactions ka group, hash se pichle block se juda hua.
//!
//! Block ke fields:
//! - index: block number (genesis = 0)
//! - timestamp: seconds since epoch
//! - prev_hash: pichle block ka hash (genesis me "0" x 64)
//! - merkle_root: saare transactions ka Merkle root
//! - transactions: is block ki transactions
//! - stake_ops: is block ki staking operations (Phase 3a — stake/unstake/
//!   claim). Ye bhi block hash me shamil hain, taaki inhe badla na ja sake.
//! - validator: block banane wale validator ka address
//! - validator_public_key_hex: validator ka Ed25519 public key (hex, 64 chars).
//!   Ye block me hi rehta hai, taaki koi bhi node bina kisi registry ke
//!   validator signature verify kar sake.
//! - validator_signature: validator ne block hash par sign kiya (hex, 128 chars)
//! - hash: is block ka SHA-256 hash
//!
//! Block hash = SHA-256(index + timestamp + prev_hash + merkle_root +
//! validator + transaction IDs + stake op IDs). Ek bhi field badli to hash
//! badal jayega.
//!
//! **Format note (Phase 3a):** hash me stake op IDs judne se purani
//! `data/chain.json` files ka hash ab match nahi karega — Phase 3a ke
//! baad purani chain file load nahi hogi. Ye dev data hai (gitignored),
//! isliye delete karke `demo` dobara chalao.

use crate::consensus::Vote;
use crate::error::{ButError, Result};
use crate::hash::sha256_hex;
use crate::merkle::merkle_root;
use crate::slash::SlashOp;
use crate::staking::StakeOp;
use crate::swap::SwapOp;
use crate::transaction::Transaction;
use crate::wallet::validate_address;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Genesis block ka prev_hash — 64 baar "0".
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Ek block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Block number. Genesis = 0.
    pub index: u64,
    /// Seconds since epoch.
    pub timestamp: u64,
    /// Pichle block ka hash.
    pub prev_hash: String,
    /// Saare transactions ka Merkle root.
    pub merkle_root: String,
    /// Is block ki transactions.
    pub transactions: Vec<Transaction>,
    /// Is block ki staking operations (Phase 3a). Purani files me nahi
    /// hoti — `#[serde(default)]` se khaali mani jayengi.
    #[serde(default)]
    pub stake_ops: Vec<StakeOp>,
    /// Is block ki swap operations (Phase 3b — AMM swap / liquidity).
    /// Hash me shamil hain, taaki inhe badla na ja sake.
    #[serde(default)]
    pub swap_ops: Vec<SwapOp>,
    /// Is block ki slash operations (Phase 3b — equivocation par slash).
    /// Hash me shamil hain, taaki inhe badla na ja sake.
    #[serde(default)]
    pub slash_ops: Vec<SlashOp>,
    /// Is block ko quorum dene wale votes (Phase 3a, BFT-only chain).
    /// Votes **hash me nahi hote** — wo block hash par sign hote hain,
    /// isliye vote judne se hash nahi badalta. Votes block ke saath save
    /// hote hain taaki sync karne wala node bhi quorum check kar sake.
    /// Purani files me nahi hoti — khaali mani jayengi.
    #[serde(default)]
    pub votes: Vec<Vote>,
    /// Block banane wale validator ka address.
    pub validator: String,
    /// Validator ka Ed25519 public key (hex, 64 characters).
    pub validator_public_key_hex: String,
    /// Validator ka signature — block hash par (hex, 128 characters).
    pub validator_signature_hex: String,
    /// Is block ka hash.
    pub hash: String,
}

impl Block {
    /// Naya block banao aur validator se sign karwao.
    /// `timestamp` caller deta hai (test me fixed time use kar sakte hain).
    /// Stake ops nahi hain (khaali list).
    pub fn new(
        index: u64,
        timestamp: u64,
        prev_hash: String,
        transactions: Vec<Transaction>,
        validator: String,
        validator_key: &SigningKey,
    ) -> Result<Self> {
        Self::new_with_stake_ops(
            index,
            timestamp,
            prev_hash,
            transactions,
            Vec::new(),
            validator,
            validator_key,
        )
    }

    /// Naya block banao — transactions + staking operations ke saath.
    pub fn new_with_stake_ops(
        index: u64,
        timestamp: u64,
        prev_hash: String,
        transactions: Vec<Transaction>,
        stake_ops: Vec<StakeOp>,
        validator: String,
        validator_key: &SigningKey,
    ) -> Result<Self> {
        Self::new_with_ops(
            index,
            timestamp,
            prev_hash,
            transactions,
            stake_ops,
            Vec::new(),
            Vec::new(),
            validator,
            validator_key,
        )
    }

    /// Naya block banao — transactions + stake ops + swap ops + slash ops.
    pub fn new_with_ops(
        index: u64,
        timestamp: u64,
        prev_hash: String,
        transactions: Vec<Transaction>,
        stake_ops: Vec<StakeOp>,
        swap_ops: Vec<SwapOp>,
        slash_ops: Vec<SlashOp>,
        validator: String,
        validator_key: &SigningKey,
    ) -> Result<Self> {
        validate_address(&validator)?;
        let ids: Vec<String> = transactions.iter().map(|tx| tx.id()).collect();
        let merkle_root = merkle_root(&ids);
        let validator_public_key_hex = hex::encode(validator_key.verifying_key().to_bytes());
        let mut block = Self {
            index,
            timestamp,
            prev_hash,
            merkle_root,
            transactions,
            stake_ops,
            swap_ops,
            slash_ops,
            votes: Vec::new(),
            validator,
            validator_public_key_hex,
            validator_signature_hex: String::new(),
            hash: String::new(),
        };
        block.hash = block.compute_hash();
        // Validator block hash par sign karta hai.
        let expected_validator =
            crate::wallet::address_from_public_key(&validator_key.verifying_key().to_bytes());
        if expected_validator != block.validator {
            return Err(ButError::InvalidBlock(
                "validator key validator address se match nahi karti".to_string(),
            ));
        }
        let signature = validator_key.sign(block.hash.as_bytes());
        block.validator_signature_hex = hex::encode(signature.to_bytes());
        Ok(block)
    }

    /// Block ka hash dobara calculate karo (validation ke liye).
    /// Stake/swap/slash op IDs bhi hash me hain — op badla to hash badal
    /// jayega. **Format note (Phase 3b):** swap/slash op IDs judne se
    /// purani `data/chain.json` files ka hash ab match nahi karega —
    /// Phase 3b ke baad purani chain file load nahi hogi. Ye dev data hai
    /// (gitignored), isliye delete karke `demo` dobara chalao.
    pub fn compute_hash(&self) -> String {
        let ids: Vec<String> = self.transactions.iter().map(|tx| tx.id()).collect();
        let op_ids: Vec<String> = self.stake_ops.iter().map(|op| op.id()).collect();
        let swap_ids: Vec<String> = self.swap_ops.iter().map(|op| op.id()).collect();
        let slash_ids: Vec<String> = self.slash_ops.iter().map(|op| op.id()).collect();
        sha256_hex(
            format!(
                "BUT-BLOCK|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                self.index,
                self.timestamp,
                self.prev_hash,
                self.merkle_root,
                self.validator,
                ids.join(","),
                op_ids.join(","),
                swap_ids.join(","),
                slash_ids.join(",")
            )
            .as_bytes(),
        )
    }

    /// Block ki structure verify karo (chain context ke bina, self-contained):
    /// hash sahi hai, Merkle root sahi hai, validator public key address se
    /// match karti hai, aur validator signature sahi hai.
    /// Transaction signatures aur balances ki checking `Chain` karta hai.
    pub fn verify_structure(&self) -> Result<()> {
        if self.hash != self.compute_hash() {
            return Err(ButError::InvalidBlock(
                "block hash galat hai — block badla gaya hai".to_string(),
            ));
        }
        let ids: Vec<String> = self.transactions.iter().map(|tx| tx.id()).collect();
        if self.merkle_root != merkle_root(&ids) {
            return Err(ButError::InvalidBlock(
                "merkle root galat hai — transaction badli gayi hai".to_string(),
            ));
        }
        // Validator public key block se lo aur address se match karo.
        let pk_bytes = hex::decode(&self.validator_public_key_hex).map_err(|_| {
            ButError::InvalidBlock("validator public key hex format sahi nahi hai".to_string())
        })?;
        if pk_bytes.len() != 32 {
            return Err(ButError::InvalidBlock(
                "validator public key 32 bytes ki honi chahiye".to_string(),
            ));
        }
        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&pk_bytes);
        let validator_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| {
            ButError::InvalidBlock("validator public key valid Ed25519 key nahi hai".to_string())
        })?;
        let expected_validator = crate::wallet::address_from_public_key(&pk_array);
        if expected_validator != self.validator {
            return Err(ButError::InvalidBlock(
                "validator public key validator address se match nahi karti".to_string(),
            ));
        }
        let sig_bytes = hex::decode(&self.validator_signature_hex).map_err(|_| {
            ButError::InvalidBlock("validator signature hex format sahi nahi hai".to_string())
        })?;
        if sig_bytes.len() != 64 {
            return Err(ButError::InvalidBlock(
                "validator signature 64 bytes ki honi chahiye".to_string(),
            ));
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_array);
        validator_key
            .verify(self.hash.as_bytes(), &signature)
            .map_err(|_| {
                ButError::InvalidBlock(
                    "validator signature verify nahi hui — block badla gaya hai".to_string(),
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn test_wallet(seed_byte: u8) -> Wallet {
        Wallet::from_entropy(&[seed_byte; 32]).unwrap()
    }

    fn signed_tx(sender: &Wallet, receiver: &Wallet, amount: u64, nonce: u64) -> Transaction {
        let mut tx = Transaction::new(
            sender.address.clone(),
            receiver.address.clone(),
            amount,
            0,
            nonce,
        );
        tx.sign(&sender.signing_key()).unwrap();
        tx
    }

    #[test]
    fn block_banta_aur_verify_hota_hai() {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let tx = signed_tx(&alice, &bob, 50, 0);
        let block = Block::new(
            1,
            1_700_000_000,
            GENESIS_PREV_HASH.to_string(),
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        block.verify_structure().unwrap();
        assert_eq!(block.hash, block.compute_hash());
        assert_eq!(block.validator_public_key_hex.len(), 64);
    }

    #[test]
    fn tampered_block_reject() {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let tx = signed_tx(&alice, &bob, 50, 0);
        let mut block = Block::new(
            1,
            1_700_000_000,
            GENESIS_PREV_HASH.to_string(),
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        block.transactions[0].amount = 999; // tamper
        assert!(block.verify_structure().is_err());
    }

    #[test]
    fn galat_validator_sign_reject() {
        let validator = test_wallet(9);
        let other = test_wallet(8);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let tx = signed_tx(&alice, &bob, 50, 0);
        // Block validator=validator, par sign other ki key se — new() khud rokega
        let err = Block::new(
            1,
            1_700_000_000,
            GENESIS_PREV_HASH.to_string(),
            vec![tx],
            validator.address.clone(),
            &other.signing_key(),
        );
        assert!(err.is_err());
    }
}
