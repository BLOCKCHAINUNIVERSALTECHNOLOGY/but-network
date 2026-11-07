//! Hash helpers — BUT chain me har hash SHA-256 hai.
//!
//! SHA-256 wahi hash hai jo TypeScript wallet module (`shared/crypto-secure.ts`)
//! bhi use karta hai, isliye address derivation dono taraf ek jaisa hai.

use sha2::{Digest, Sha256};

/// Data ka SHA-256 hash, hex string me (64 characters).
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Data ka SHA-256 hash, raw 32 bytes me.
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&out);
    bytes
}
