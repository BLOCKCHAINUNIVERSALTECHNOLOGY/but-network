//! Real wallet — BIP39 mnemonic, Ed25519 keypair, fixed-length address.
//!
//! Sanjay ka fixed-length spec (na kam, na zyada):
//! - Seed phrase: bilkul 24 words (real BIP39, official 2048-word list).
//! - Private key: bilkul 128 hex characters = 64 bytes
//!   (pehle 32 bytes secret seed + aakhri 32 bytes public key).
//! - Wallet address: bilkul 56 characters — "but1" + SHA-256(public key)
//!   ke pehle 52 hex characters.
//!
//! IMPORTANT — TypeScript demo se fark:
//! TypeScript demo (`shared/wallet-secure.ts`) me mnemonic aur keypair
//! alag-alag random bante hain — mnemonic sirf dikhane ke liye hai, keypair
//! usse nahi banta. REAL core me mnemonic HI wallet hai: mnemonic se
//! BIP39 seed banta hai (PBKDF2-HMAC-SHA512, 2048 iterations, salt
//! "mnemonic"), uske pehle 32 bytes se Ed25519 keypair banta hai. Isliye
//! wahi 24 words hamesha wahi address denge — jaise asli wallet me hota hai.
//!
//! Key derivation (BUT standard, document kar rahe hain):
//!   seed = PBKDF2-HMAC-SHA512(mnemonic, salt "mnemonic", 2048 iters, 64 bytes)
//!   secret = seed[0..32]
//!   public_key = Ed25519(secret)
//!   address = "but1" + sha256(public_key)[0..52 hex]
//!   private_key_hex = hex(secret) + hex(public_key)   (128 characters)

use crate::error::{ButError, Result};
use crate::hash::sha256_hex;
use bip39::Mnemonic;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;

/// Seed phrase hamesha bilkul 24 words ki hogi (real BIP39).
pub const MNEMONIC_WORD_COUNT: usize = 24;

/// Private key hamesha bilkul 128 hex characters ki hogi.
pub const PRIVATE_KEY_LENGTH: usize = 128;

/// Wallet address hamesha bilkul 56 characters ka hoga.
pub const ADDRESS_LENGTH: usize = 56;

/// Address hamesha "but1" se shuru hoga.
pub const ADDRESS_PREFIX: &str = "but1";

/// BIP39 entropy: 24 words ke liye 256 bit (32 bytes).
const BIP39_ENTROPY_BYTES: usize = 32;

/// Real wallet — mnemonic se bana hua, deterministic.
#[derive(Debug, Clone)]
pub struct Wallet {
    /// 24-word BIP39 mnemonic (kabhi store mat karo, kabhi share mat karo).
    pub mnemonic: String,
    /// 32-byte Ed25519 secret seed.
    pub secret_seed: [u8; 32],
    /// 32-byte Ed25519 public key.
    pub public_key: [u8; 32],
    /// Bilkul 56 characters: "but1" + sha256(public_key) ke pehle 52 hex chars.
    pub address: String,
    /// Bilkul 128 hex characters: hex(secret_seed) + hex(public_key).
    pub private_key_hex: String,
}

impl Wallet {
    /// Naya real wallet banao — CSPRNG (OS entropy) se 24-word mnemonic.
    pub fn generate() -> Result<Self> {
        let mut entropy = [0u8; BIP39_ENTROPY_BYTES];
        OsRng.fill_bytes(&mut entropy);
        Self::from_entropy(&entropy)
    }

    /// 32-byte entropy se wallet banao (test vector ke liye bhi use hota hai).
    pub fn from_entropy(entropy: &[u8; BIP39_ENTROPY_BYTES]) -> Result<Self> {
        let mnemonic = Mnemonic::from_entropy(entropy)
            .map_err(|e| ButError::InvalidMnemonic(format!("entropy se mnemonic nahi bana: {e}")))?;
        Self::from_mnemonic(&mnemonic.to_string())
    }

    /// 24-word mnemonic se wallet banao — deterministic: wahi words, wahi wallet.
    pub fn from_mnemonic(mnemonic: &str) -> Result<Self> {
        let parsed = Mnemonic::parse(mnemonic)
            .map_err(|e| ButError::InvalidMnemonic(format!("mnemonic sahi nahi hai: {e}")))?;
        if parsed.word_count() != MNEMONIC_WORD_COUNT {
            return Err(ButError::InvalidMnemonic(format!(
                "seed phrase bilkul {MNEMONIC_WORD_COUNT} words ki honi chahiye, mili: {}",
                parsed.word_count()
            )));
        }
        // BIP39 seed: PBKDF2-HMAC-SHA512, salt "mnemonic", 2048 iterations.
        let seed = parsed.to_seed("");
        let mut secret_seed = [0u8; 32];
        secret_seed.copy_from_slice(&seed[..32]);
        let signing_key = SigningKey::from_bytes(&secret_seed);
        let public_key = signing_key.verifying_key().to_bytes();
        let address = address_from_public_key(&public_key);
        let private_key_hex = format!("{}{}", hex::encode(secret_seed), hex::encode(public_key));
        assert_fixed_lengths(&address, &private_key_hex)?;
        Ok(Self {
            mnemonic: parsed.to_string(),
            secret_seed,
            public_key,
            address,
            private_key_hex,
        })
    }

    /// 128-char private key (hex) se wallet banao — CLI/API ke liye.
    /// Private key = hex(secret 32 bytes) + hex(public 32 bytes).
    /// Public key secret se dobara banakar check hoti hai — match na kare
    /// to key galat hai (error). **Mnemonic is wallet me nahi hoti**
    /// (private key se mnemonic wapas nahi banti) — `mnemonic` khaali
    /// rehta hai; ye wallet sirf sign karne ke liye hai, backup ke liye
    /// nahi. Backup ke liye hamesha 24-word mnemonic chahiye.
    pub fn from_private_key_hex(private_key_hex: &str) -> Result<Self> {
        if private_key_hex.len() != PRIVATE_KEY_LENGTH {
            return Err(ButError::InvalidTransaction(format!(
                "private key bilkul {PRIVATE_KEY_LENGTH} characters ki honi chahiye, mili: {}",
                private_key_hex.len()
            )));
        }
        let bytes = hex::decode(private_key_hex).map_err(|_| {
            ButError::InvalidTransaction("private key hex format sahi nahi hai".to_string())
        })?;
        if bytes.len() != 64 {
            return Err(ButError::InvalidTransaction(
                "private key 64 bytes ki honi chahiye (128 hex chars)".to_string(),
            ));
        }
        let mut secret_seed = [0u8; 32];
        secret_seed.copy_from_slice(&bytes[..32]);
        let mut public_key = [0u8; 32];
        public_key.copy_from_slice(&bytes[32..]);
        // Public key secret se match karni chahiye — warna key tuti hui hai.
        let signing_key = SigningKey::from_bytes(&secret_seed);
        let derived_public = signing_key.verifying_key().to_bytes();
        if derived_public != public_key {
            return Err(ButError::InvalidTransaction(
                "private key ke andar public key secret se match nahi karti — key galat hai".to_string(),
            ));
        }
        let address = address_from_public_key(&public_key);
        assert_fixed_lengths(&address, private_key_hex)?;
        Ok(Self {
            mnemonic: String::new(),
            secret_seed,
            public_key,
            address,
            private_key_hex: private_key_hex.to_string(),
        })
    }

    /// Is wallet ka Ed25519 signing key (transaction sign karne ke liye).
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.secret_seed)
    }

    /// Is wallet ka Ed25519 verifying (public) key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey::from_bytes(&self.public_key)
            .expect("wallet ka public key hamesha valid Ed25519 key hai")
    }
}

/// 32-byte Ed25519 public key se BUT address banao (bilkul 56 characters).
pub fn address_from_public_key(public_key: &[u8; 32]) -> String {
    let hash = sha256_hex(public_key);
    format!("{ADDRESS_PREFIX}{}", &hash[..52])
}

/// Private key hex (128 characters) se public key nikalo — aakhri 64 chars.
/// Sirf format check ke liye; secret seed pehle 64 chars me hota hai.
pub fn public_key_from_private_key_hex(private_key_hex: &str) -> Result<[u8; 32]> {
    if private_key_hex.len() != PRIVATE_KEY_LENGTH {
        return Err(ButError::InvalidLength(format!(
            "private key bilkul {PRIVATE_KEY_LENGTH} characters ki honi chahiye, mili: {}",
            private_key_hex.len()
        )));
    }
    let bytes = hex::decode(private_key_hex)
        .map_err(|_| ButError::InvalidLength("private key hex format sahi nahi hai".to_string()))?;
    if bytes.len() != 64 {
        return Err(ButError::InvalidLength(
            "private key 64 bytes ki honi chahiye".to_string(),
        ));
    }
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&bytes[32..64]);
    Ok(public_key)
}

/// Address format check: bilkul 56 characters, "but1" se shuru, baaki hex.
pub fn validate_address(address: &str) -> Result<()> {
    if address.len() != ADDRESS_LENGTH {
        return Err(ButError::InvalidLength(format!(
            "address bilkul {ADDRESS_LENGTH} characters ka hona chahiye, mila: {}",
            address.len()
        )));
    }
    if !address.starts_with(ADDRESS_PREFIX) {
        return Err(ButError::InvalidLength(
            "address \"but1\" se shuru hona chahiye".to_string(),
        ));
    }
    if hex::decode(&address[4..]).is_err() {
        return Err(ButError::InvalidLength(
            "address ke aakhri 52 characters hex hone chahiye".to_string(),
        ));
    }
    Ok(())
}

/// Fixed lengths check — galat length ho to error (na kam, na zyada).
pub fn assert_fixed_lengths(address: &str, private_key_hex: &str) -> Result<()> {
    validate_address(address)?;
    if private_key_hex.len() != PRIVATE_KEY_LENGTH {
        return Err(ButError::InvalidLength(format!(
            "private key bilkul {PRIVATE_KEY_LENGTH} characters ki honi chahiye, mili: {}",
            private_key_hex.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonic_bilkul_24_words() {
        let w = Wallet::generate().unwrap();
        assert_eq!(w.mnemonic.split_whitespace().count(), MNEMONIC_WORD_COUNT);
    }

    #[test]
    fn fixed_lengths_hamesha_sahi() {
        let w = Wallet::generate().unwrap();
        assert_eq!(w.address.len(), ADDRESS_LENGTH);
        assert!(w.address.starts_with(ADDRESS_PREFIX));
        assert_eq!(w.private_key_hex.len(), PRIVATE_KEY_LENGTH);
    }

    #[test]
    fn wahi_mnemonic_wahi_wallet() {
        let w1 = Wallet::generate().unwrap();
        let w2 = Wallet::from_mnemonic(&w1.mnemonic).unwrap();
        assert_eq!(w1.address, w2.address);
        assert_eq!(w1.private_key_hex, w2.private_key_hex);
        assert_eq!(w1.public_key, w2.public_key);
    }

    #[test]
    fn bip39_test_vector_zero_entropy() {
        // Mashhoor BIP39 test vector: 32 bytes zero entropy ->
        // "abandon" x23 + "art". Isse pata chalta hai BIP39 REAL hai.
        let entropy = [0u8; 32];
        let w = Wallet::from_entropy(&entropy).unwrap();
        let words: Vec<&str> = w.mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 24);
        assert!(words[..23].iter().all(|x| *x == "abandon"));
        assert_eq!(words[23], "art");
    }

    #[test]
    fn galat_mnemonic_reject() {
        assert!(Wallet::from_mnemonic("hello world").is_err());
        // 12 words reject (sirf 24 chalenge)
        let twelve = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        assert!(Wallet::from_mnemonic(twelve).is_err());
    }

    #[test]
    fn galat_address_reject() {
        assert!(validate_address("but1abc").is_err()); // bahut chhota
        let long = format!("but1{}", "a".repeat(53));
        assert!(validate_address(&long).is_err()); // 57 characters
        assert!(validate_address("xyz1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err());
    }

    #[test]
    fn private_key_se_public_key() {
        let w = Wallet::generate().unwrap();
        let pk = public_key_from_private_key_hex(&w.private_key_hex).unwrap();
        assert_eq!(pk, w.public_key);
        assert!(public_key_from_private_key_hex("abcd").is_err());
    }

    #[test]
    fn private_key_hex_se_wallet_wapas_banta_hai() {
        let w = Wallet::generate().unwrap();
        let w2 = Wallet::from_private_key_hex(&w.private_key_hex).unwrap();
        assert_eq!(w2.address, w.address);
        assert_eq!(w2.public_key, w.public_key);
        assert_eq!(w2.secret_seed, w.secret_seed);
        // Mnemonic private key se nahi banti — khaali rehti hai.
        assert!(w2.mnemonic.is_empty());
        // Galat length reject.
        assert!(Wallet::from_private_key_hex("abcd").is_err());
        // Public key badli hui ho to reject (secret se match nahi karegi).
        let mut bad = w.private_key_hex.clone();
        let last = bad.pop().unwrap();
        bad.push(if last == '0' { '1' } else { '0' });
        assert!(Wallet::from_private_key_hex(&bad).is_err());
    }
}
