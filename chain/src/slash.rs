//! Slashing — block-level op (Phase 3b).
//!
//! Phase 3a tak problem thi: `Chain::slash_validator` seedha chain state
//! badalta tha, blocks se nahi — isliye uske baad `verify_chain` fail ho
//! jata tha (rebuilt state stored state se match nahi karti thi). Ab
//! slashing **block me darj hone wala signed op** hai — jaise StakeOp:
//! - Slash op block me jata hai, chain verify karte waqt rebuild bhi
//!   wahi slash lagata hai — isliye verify ke baad fail nahi hota.
//! - Saboot (evidence) op ke andar hi hota hai: do conflicting votes.
//!   Koi bhi node saboot check karke slash ko verify kar sakta hai.
//!
//! **Equivocation kya hai (BFT ka sabse bada jurm):** ek validator ek hi
//! height par do ALAG blocks ko approve vote de — isse fork banaya ja
//! sakta hai. Dono votes usi validator ki Ed25519 signature se sign hote
//! hain, isliye saboot khud hi pakka hota hai — na koi bharosa chahiye,
//! na koi committee.
//!
//! Kaise kaam karta hai:
//! 1. Koi node P2P gossip me equivocation dekhta hai (do conflicting votes).
//! 2. Wo node `SlashOp` banata hai — target, dono votes (saboot), reason,
//!    apna address (reporter) + nonce — aur apni key se sign karta hai.
//!    Reporter koi bhi ho sakta hai (validator hona zaroori nahi).
//! 3. Op mempool → block me jata hai. Chain apply karte waqt:
//!    saboot verify karta hai, reporter ka nonce check karta hai,
//!    target validator ka stake 10% kaat ta hai (lock se bhi utna hi),
//!    stake minimum se neeche gira to jail.
//! 4. **Ek saboot par ek hi baar slash:** dedup **evidence ID** se hota
//!    hai (target + height + dono block hash, order-independent —
//!    `evidence_id()`). Op ID me nonce bhi shamil hai, isliye dedup op ID
//!    se nahi hota: reporter naya nonce lekar wahi saboot dobara bheje to
//!    bhi evidence ID pehle se applied set me milegi aur reject hoga.
//!    Isse ek hi galti par baar-baar slash nahi hota.
//!
//! SACH BATA RAHE HAIN — kya abhi bhi simplified hai:
//! - Abhi sirf **equivocation** (double-vote) par slash hota hai —
//!   downtime (validator ka lambe time tak offline rehna) ya galat block
//!   propose karne par slash abhi nahi hai (wo alag evidence chahte hain).
//! - Reporter ko koi inaam (bounty) nahi milta — wo sirf network ki
//!   suraksha ke liye report karta hai. Bounty Phase 3c/testnet me.
//! - Slash op ka reporter nonce shared sequence se aata hai — reporter ne
//!   beech me koi aur tx ki ho to nonce badal jayega aur op reject hoga
//!   (dobara sahi nonce se bhejna padega).

use crate::consensus::Vote;
use crate::error::{ButError, Result};
use crate::hash::sha256_hex;
use crate::wallet::validate_address;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Block me jaane wala signed slash operation.
///
/// Payload: `BUT-SLASH|<target>|<height>|<hash_min>|<hash_max>|<reason>|<reporter>|<nonce>`.
/// Dono block hash sort karke likhe jate hain — taaki (a,b) aur (b,a) se
/// wahi op ID bane aur ek saboot par do baar slash na ho.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashOp {
    /// Jis validator ko slash karna hai.
    pub target: String,
    /// Equivocation ka saboot — pehla conflicting vote.
    pub vote_a: Vote,
    /// Equivocation ka saboot — doosra conflicting vote (alag block hash).
    pub vote_b: Vote,
    /// Reason — jaise "equivocation height 7".
    pub reason: String,
    /// Report karne wale ka address (koi bhi ho sakta hai).
    pub reporter: String,
    /// Reporter ka nonce — transfer/stake op ke saath ek hi sequence.
    pub nonce: u64,
    /// Reporter ka Ed25519 public key (hex, 64 characters).
    pub reporter_public_key_hex: String,
    /// Ed25519 signature (hex, 128 characters) — op ID par sign.
    pub signature_hex: String,
}

impl SlashOp {
    /// Bina signature ke op banao (sign karne se pehle).
    /// `vote_a` aur `vote_b` conflicting votes hone chahiye — verify() check karega.
    pub fn new(
        target: String,
        vote_a: Vote,
        vote_b: Vote,
        reason: String,
        reporter: String,
        nonce: u64,
    ) -> Self {
        Self {
            target,
            vote_a,
            vote_b,
            reason,
            reporter,
            nonce,
            reporter_public_key_hex: String::new(),
            signature_hex: String::new(),
        }
    }

    /// Saboot ka ID — target + height + dono block hash (sort karke).
    /// Order-independent hai: (a,b) aur (b,a) se wahi ID banta hai.
    pub fn evidence_id(&self) -> String {
        let (lo, hi) = if self.vote_a.block_hash <= self.vote_b.block_hash {
            (&self.vote_a.block_hash, &self.vote_b.block_hash)
        } else {
            (&self.vote_b.block_hash, &self.vote_a.block_hash)
        };
        sha256_hex(
            format!(
                "BUT-SLASH-EVIDENCE|{}|{}|{}|{}",
                self.target, self.vote_a.height, lo, hi
            )
            .as_bytes(),
        )
    }

    /// Op ka ID — evidence ID + reason + reporter + nonce ka hash.
    /// Wahi saboot, wahi reporter, wahi nonce → wahi ID (replay rokta hai).
    pub fn id(&self) -> String {
        sha256_hex(self.canonical_payload().as_bytes())
    }

    /// Sign karne wala payload — hamesha ek hi format.
    /// Format badla to purani signature toot jayegi.
    fn canonical_payload(&self) -> String {
        format!(
            "BUT-SLASH|{}|{}|{}|{}",
            self.evidence_id(),
            self.reason,
            self.reporter,
            self.nonce
        )
    }

    /// Reporter ki key se op par sign karo.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        let public_key = signing_key.verifying_key().to_bytes();
        let expected = crate::wallet::address_from_public_key(&public_key);
        if expected != self.reporter {
            return Err(ButError::InvalidSignature(
                "sign karne wali key reporter address se match nahi karti".to_string(),
            ));
        }
        self.reporter_public_key_hex = hex::encode(public_key);
        let op_id = self.id();
        let signature = signing_key.sign(op_id.as_bytes());
        self.signature_hex = hex::encode(signature.to_bytes());
        Ok(())
    }

    /// Op verify karo — saboot + format + signature.
    /// Nonce, validator status aur double-slash ki checking `Chain` karta
    /// hai (state janta hai).
    pub fn verify(&self) -> Result<()> {
        validate_address(&self.target)?;
        validate_address(&self.reporter)?;
        if self.reason.trim().is_empty() {
            return Err(ButError::InvalidTransaction(
                "slash op me reason likhna zaroori hai".to_string(),
            ));
        }
        if self.target == self.reporter {
            return Err(ButError::InvalidTransaction(
                "validator khud ko slash nahi kar sakta — koi aur report kare".to_string(),
            ));
        }
        // Saboot check: dono votes verify hon, target ke hon, ek hi height
        // par hon, do alag blocks ko approve karte hon.
        self.vote_a.verify().map_err(|e| {
            ButError::InvalidTransaction(format!("saboot ka pehla vote galat hai: {e}"))
        })?;
        self.vote_b.verify().map_err(|e| {
            ButError::InvalidTransaction(format!("saboot ka doosra vote galat hai: {e}"))
        })?;
        if self.vote_a.voter != self.target || self.vote_b.voter != self.target {
            return Err(ButError::InvalidTransaction(
                "saboot ke votes target validator ke hone chahiye".to_string(),
            ));
        }
        if self.vote_a.height != self.vote_b.height {
            return Err(ButError::InvalidTransaction(format!(
                "saboot ke votes ek hi height par hone chahiye — mile: {} aur {}",
                self.vote_a.height, self.vote_b.height
            )));
        }
        if self.vote_a.block_hash == self.vote_b.block_hash {
            return Err(ButError::InvalidTransaction(
                "saboot ke votes do ALAG blocks ke hone chahiye — dono ka block hash ek hi hai (ye equivocation nahi hai)".to_string(),
            ));
        }
        if !self.vote_a.approve || !self.vote_b.approve {
            return Err(ButError::InvalidTransaction(
                "saboot ke dono votes approve hone chahiye — reject vote equivocation nahi hai".to_string(),
            ));
        }
        self.verify_signature()
    }

    fn verify_signature(&self) -> Result<()> {
        let pk_bytes = hex::decode(&self.reporter_public_key_hex).map_err(|_| {
            ButError::InvalidSignature("reporter public key hex format sahi nahi hai".to_string())
        })?;
        if pk_bytes.len() != 32 {
            return Err(ButError::InvalidSignature(
                "reporter public key 32 bytes ki honi chahiye".to_string(),
            ));
        }
        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&pk_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| {
            ButError::InvalidSignature("reporter public key valid Ed25519 key nahi hai".to_string())
        })?;
        if crate::wallet::address_from_public_key(&pk_array) != self.reporter {
            return Err(ButError::InvalidSignature(
                "public key reporter address se match nahi karti".to_string(),
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
                    "slash op signature verify nahi hui — op badla gaya ya galat key se sign hua"
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

    fn conflicting_votes(target: &Wallet, height: u64) -> (Vote, Vote) {
        let a = Vote::new(
            "a".repeat(64),
            height,
            true,
            target.address.clone(),
            &target.signing_key(),
        )
        .unwrap();
        let b = Vote::new(
            "b".repeat(64),
            height,
            true,
            target.address.clone(),
            &target.signing_key(),
        )
        .unwrap();
        (a, b)
    }

    fn signed_slash(target: &Wallet, reporter: &Wallet, height: u64, nonce: u64) -> SlashOp {
        let (a, b) = conflicting_votes(target, height);
        let mut op = SlashOp::new(
            target.address.clone(),
            a,
            b,
            format!("equivocation height {height}"),
            reporter.address.clone(),
            nonce,
        );
        op.sign(&reporter.signing_key()).unwrap();
        op
    }

    #[test]
    fn sahi_saboot_verify_hota_hai() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let op = signed_slash(&alice, &bob, 7, 0);
        op.verify().unwrap();
    }

    #[test]
    fn evidence_id_order_independent_hai() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let op1 = signed_slash(&alice, &bob, 7, 0);
        // Votes ulta karke bhi wahi evidence ID.
        let mut op2 = SlashOp::new(
            alice.address.clone(),
            op1.vote_b.clone(),
            op1.vote_a.clone(),
            "equivocation height 7".to_string(),
            bob.address.clone(),
            0,
        );
        op2.sign(&bob.signing_key()).unwrap();
        assert_eq!(op1.evidence_id(), op2.evidence_id());
        assert_eq!(op1.id(), op2.id());
        op2.verify().unwrap();
    }

    #[test]
    fn ek_hi_block_ke_vote_saboot_nahi() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let a = Vote::new(
            "a".repeat(64),
            7,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let b = a.clone(); // wahi vote dobara — alag block nahi
        let mut op = SlashOp::new(
            alice.address.clone(),
            a,
            b,
            "equivocation height 7".to_string(),
            bob.address.clone(),
            0,
        );
        op.sign(&bob.signing_key()).unwrap();
        assert!(op.verify().is_err());
    }

    #[test]
    fn alag_height_ke_vote_saboot_nahi() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let a = Vote::new(
            "a".repeat(64),
            7,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let b = Vote::new(
            "b".repeat(64),
            8, // alag height
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let mut op = SlashOp::new(
            alice.address.clone(),
            a,
            b,
            "equivocation".to_string(),
            bob.address.clone(),
            0,
        );
        op.sign(&bob.signing_key()).unwrap();
        assert!(op.verify().is_err());
    }

    #[test]
    fn reject_vote_saboot_nahi() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let a = Vote::new(
            "a".repeat(64),
            7,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let b = Vote::new(
            "b".repeat(64),
            7,
            false, // reject vote — equivocation nahi
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let mut op = SlashOp::new(
            alice.address.clone(),
            a,
            b,
            "equivocation".to_string(),
            bob.address.clone(),
            0,
        );
        op.sign(&bob.signing_key()).unwrap();
        assert!(op.verify().is_err());
    }

    #[test]
    fn dusre_validator_ke_vote_saboot_nahi() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let carol = test_wallet(3);
        // Ek vote Alice ka, ek Carol ka — target Alice hai.
        let a = Vote::new(
            "a".repeat(64),
            7,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        let b = Vote::new(
            "b".repeat(64),
            7,
            true,
            carol.address.clone(),
            &carol.signing_key(),
        )
        .unwrap();
        let mut op = SlashOp::new(
            alice.address.clone(),
            a,
            b,
            "equivocation".to_string(),
            bob.address.clone(),
            0,
        );
        op.sign(&bob.signing_key()).unwrap();
        assert!(op.verify().is_err());
    }

    #[test]
    fn tampered_slash_op_reject() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let mut op = signed_slash(&alice, &bob, 7, 0);
        // Tamper: reason badla — signature toot gayi.
        op.reason = "kuch aur".to_string();
        assert!(op.verify().is_err());
        // Bina signature reject.
        let (a, b) = conflicting_votes(&alice, 7);
        let unsigned = SlashOp::new(
            alice.address.clone(),
            a,
            b,
            "equivocation height 7".to_string(),
            bob.address.clone(),
            0,
        );
        assert!(unsigned.verify().is_err());
        // Khud ko slash karna reject.
        let op_self = signed_slash(&alice, &alice, 7, 0);
        assert!(op_self.verify().is_err());
    }
}
