//! Real transaction — sign karo, verify karo, bina signature ke reject.
//!
//! Transaction ke fields:
//! - sender: "but1..." address (56 characters, fixed)
//! - receiver: "but1..." address (56 characters, fixed)
//! - amount: kitne smallest unit bhejne hain (u64, 0 se zyada hona chahiye)
//! - fee: validator ko kitni fee milegi (u64, 0 bhi chalega)
//! - nonce: sender ka sequence number — replay attack rokta hai.
//!   Pehli transaction ka nonce 0 hota hai, phir 1, 2, 3...
//! - sender_public_key_hex: sender ka Ed25519 public key (hex, 64 chars).
//!   Ye transaction me hi rehta hai, taaki koi bhi node bina kisi registry
//!   ke signature verify kar sake (Ed25519 me public key signature se
//!   recover nahi hoti, isliye key saath rakhni padti hai).
//! - signature: sender ke Ed25519 secret se transaction-id par sign.
//!
//! Transaction ID = SHA-256(sender + receiver + amount + fee + nonce).
//! Sign hota hai transaction ID par — isliye amount ya receiver badla to
//! signature fail ho jayega (tamper-proof).

use crate::butpay::ButPayRegistry;
use crate::error::{ButError, Result};
use crate::hash::sha256_hex;
use crate::wallet::validate_address;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Ek transaction — sender se receiver ko amount + fee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Sender ka address ("but1...", 56 characters).
    pub sender: String,
    /// Receiver ka address ("but1...", 56 characters).
    pub receiver: String,
    /// Kitne smallest unit bhejne hain. 0 se zyada hona chahiye.
    pub amount: u64,
    /// Validator ko fee (smallest unit). 0 bhi chalega.
    pub fee: u64,
    /// Sender ka sequence number. Pehli tx ka nonce 0.
    pub nonce: u64,
    /// Sender ka Ed25519 public key (hex, 64 characters).
    /// `sign()` se bharta hai.
    pub sender_public_key_hex: String,
    /// Ed25519 signature (hex, 128 characters) — transaction ID par sign.
    /// Bina signature ke transaction hamesha reject hogi.
    pub signature_hex: String,
}

impl Transaction {
    /// Bina signature ke transaction banao (sign karne se pehle).
    /// `sender_public_key_hex` aur `signature_hex` khali rahenge —
    /// `sign()` se bharo.
    pub fn new(sender: String, receiver: String, amount: u64, fee: u64, nonce: u64) -> Self {
        Self {
            sender,
            receiver,
            amount,
            fee,
            nonce,
            sender_public_key_hex: String::new(),
            signature_hex: String::new(),
        }
    }

    /// Transaction ka ID — fields ka SHA-256 hash (signature ke bina).
    /// Ye ID sign hota hai aur block me Merkle tree me jata hai.
    pub fn id(&self) -> String {
        sha256_hex(self.canonical_payload().as_bytes())
    }

    /// Sign karne wala payload — hamesha ek hi format, taaki verify bhi
    /// wahi bytes check kare. Format badla to purani signature toot jayegi,
    /// isliye ye format kabhi mat badlo.
    fn canonical_payload(&self) -> String {
        format!(
            "BUT-TX|{}|{}|{}|{}|{}",
            self.sender, self.receiver, self.amount, self.fee, self.nonce
        )
    }

    /// Sender ke secret key se transaction par sign karo.
    /// Sign karne se pehle sender address aur public key match hone chahiye.
    /// Sign karne par `sender_public_key_hex` bhi bhar jata hai.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        let public_key = signing_key.verifying_key().to_bytes();
        let expected_sender = crate::wallet::address_from_public_key(&public_key);
        if expected_sender != self.sender {
            return Err(ButError::InvalidSignature(
                "sign karne wali key sender address se match nahi karti".to_string(),
            ));
        }
        self.sender_public_key_hex = hex::encode(public_key);
        let tx_id = self.id();
        let signature = signing_key.sign(tx_id.as_bytes());
        self.signature_hex = hex::encode(signature.to_bytes());
        Ok(())
    }

    /// Transaction verify karo — bina sahi signature ke hamesha reject.
    /// Public key transaction me se hi li jati hai, isliye koi bhi node
    /// bina kisi registry ke verify kar sakta hai. Check karta hai:
    /// 1. Address format sahi hai (56 chars, "but1" se shuru).
    /// 2. Amount 0 se zyada hai, sender aur receiver alag hain.
    /// 3. Public key 32 bytes ki hai aur usi se sender address bana hai
    ///    (address = sha256(pubkey) — galat key pakdi jayegi).
    /// 4. Signature 64 bytes ki hai aur us public key se transaction ID
    ///    par verify hoti hai.
    pub fn verify(&self) -> Result<()> {
        validate_address(&self.receiver)?;
        self.verify_with_resolved_receiver(&self.receiver)
    }

    /// BUT Pay receiver wali transaction verify karo.
    /// Receiver BUT Pay ID (jaise "100001") ya handle (jaise "@sanjay") bhi
    /// ho sakta hai — registry se address nikala jata hai. Signature
    /// hamesha **jaisi likhi gayi payload** par verify hoti hai (receiver
    /// = ID string, jaisa user ne sign kiya tha) — isliye registry
    /// immutable hona zaroori hai: ID ka matlab kabhi nahi badlega, to
    /// signature ka matlab bhi kabhi nahi badlega.
    pub fn verify_with_registry(&self, registry: &ButPayRegistry) -> Result<()> {
        let resolved = registry.resolve_for_tx(&self.receiver)?;
        self.verify_with_resolved_receiver(&resolved)
    }

    /// Receiver pehle se resolve karke verify karo (API/mempool ke liye).
    /// `resolved_receiver` hamesha valid address hona chahiye.
    pub fn verify_with_resolved_receiver(&self, resolved_receiver: &str) -> Result<()> {
        validate_address(&self.sender)?;
        validate_address(resolved_receiver)?;
        if self.amount == 0 {
            return Err(ButError::InvalidTransaction(
                "amount 0 se zyada hona chahiye".to_string(),
            ));
        }
        if self.sender == resolved_receiver {
            return Err(ButError::InvalidTransaction(
                "sender aur receiver ek hi nahi ho sakte".to_string(),
            ));
        }
        self.verify_signature()
    }

    /// Sirf signature check — public key + signature, transaction ID par.
    /// Payload wahi rehta hai jaisa sign hua tha (receiver = original
    /// string, chahe wo address ho ya BUT Pay ID/handle).
    fn verify_signature(&self) -> Result<()> {
        // Public key transaction se lo aur address se match karo.
        let pk_bytes = hex::decode(&self.sender_public_key_hex).map_err(|_| {
            ButError::InvalidSignature("sender public key hex format sahi nahi hai".to_string())
        })?;
        if pk_bytes.len() != 32 {
            return Err(ButError::InvalidSignature(
                "sender public key 32 bytes ki honi chahiye".to_string(),
            ));
        }
        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&pk_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| {
            ButError::InvalidSignature("sender public key valid Ed25519 key nahi hai".to_string())
        })?;
        let expected_sender = crate::wallet::address_from_public_key(&pk_array);
        if expected_sender != self.sender {
            return Err(ButError::InvalidSignature(
                "public key sender address se match nahi karti".to_string(),
            ));
        }
        // Signature 64 bytes (128 hex chars) honi chahiye.
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
                    "signature verify nahi hui — transaction badli gayi ya galat key se sign hui"
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

    fn wallet_and_tx() -> (Wallet, Wallet, Transaction) {
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 1, 0);
        (alice, bob, tx)
    }

    #[test]
    fn sahi_signature_verify_hota_hai() {
        let (alice, _, mut tx) = wallet_and_tx();
        tx.sign(&alice.signing_key()).unwrap();
        assert_eq!(tx.signature_hex.len(), 128);
        assert_eq!(tx.sender_public_key_hex.len(), 64);
        tx.verify().unwrap();
    }

    #[test]
    fn bina_signature_reject() {
        let (_, _, tx) = wallet_and_tx();
        assert!(tx.verify().is_err());
    }

    #[test]
    fn amount_badla_to_signature_fail() {
        let (alice, _, mut tx) = wallet_and_tx();
        tx.sign(&alice.signing_key()).unwrap();
        tx.amount = 999; // tamper: amount badla
        assert!(tx.verify().is_err());
    }

    #[test]
    fn receiver_badla_to_signature_fail() {
        let (alice, _, mut tx) = wallet_and_tx();
        tx.sign(&alice.signing_key()).unwrap();
        let carol = Wallet::from_entropy(&[3u8; 32]).unwrap();
        tx.receiver = carol.address; // tamper: receiver badla
        assert!(tx.verify().is_err());
    }

    #[test]
    fn galat_key_se_sign_reject() {
        let (alice, bob, mut tx) = wallet_and_tx();
        // Bob Alice ki tx sign nahi kar sakta — sign() khud rokega.
        assert!(tx.sign(&bob.signing_key()).is_err());
        // Zabardasti Bob ki public key laga do to address mismatch pakda jayega.
        let mut tx2 = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 1, 0);
        tx2.sign(&alice.signing_key()).unwrap();
        tx2.sender_public_key_hex = hex::encode(bob.public_key);
        assert!(tx2.verify().is_err());
    }

    #[test]
    fn zero_amount_reject() {
        let (alice, _, mut tx) = wallet_and_tx();
        tx.amount = 0;
        tx.sign(&alice.signing_key()).unwrap();
        assert!(tx.verify().is_err());
    }

    #[test]
    fn khud_ko_payment_reject() {
        let (alice, _, mut tx) = wallet_and_tx();
        tx.receiver = alice.address.clone();
        tx.sign(&alice.signing_key()).unwrap();
        assert!(tx.verify().is_err());
    }

    // ---- BUT Pay: ID/handle receiver ----

    #[test]
    fn id_receiver_signature_verify_hoti_hai() {
        use crate::butpay::ButPayRegistry;
        let (alice, bob, _) = wallet_and_tx();
        // Bob ki ID 100001 register karo.
        let mut reg = ButPayRegistry::new();
        assert_eq!(reg.register(&bob.address, Some("bob")).unwrap(), 100001);
        // Alice Bob ko ID se bhejti hai — sign ID wali string par hota hai.
        let mut tx = Transaction::new(alice.address.clone(), "100001".to_string(), 100, 1, 0);
        tx.sign(&alice.signing_key()).unwrap();
        // Plain verify() reject karega (receiver address nahi hai).
        assert!(tx.verify().is_err());
        // Registry ke saath verify hoga — signature ID wali payload par hi check hoti hai.
        tx.verify_with_registry(&reg).unwrap();
        // Handle se bhi chalega.
        let mut tx2 = Transaction::new(alice.address.clone(), "@bob".to_string(), 50, 0, 1);
        tx2.sign(&alice.signing_key()).unwrap();
        tx2.verify_with_registry(&reg).unwrap();
    }

    #[test]
    fn galat_id_receiver_verify_fail() {
        use crate::butpay::ButPayRegistry;
        let (alice, _, _) = wallet_and_tx();
        let reg = ButPayRegistry::new(); // khaali registry
        let mut tx = Transaction::new(alice.address.clone(), "999999".to_string(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        assert!(tx.verify_with_registry(&reg).is_err());
        // Khud ko ID se bhejna bhi reject (resolve ke baad sender == receiver).
        let mut reg2 = ButPayRegistry::new();
        reg2.register(&alice.address, Some("alice")).unwrap();
        let mut tx2 = Transaction::new(alice.address.clone(), "100001".to_string(), 10, 0, 0);
        tx2.sign(&alice.signing_key()).unwrap();
        assert!(tx2.verify_with_registry(&reg2).is_err());
    }
}
