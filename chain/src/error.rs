//! Chain ke errors — ek jagah, saaf messages (Hindi + English).

use std::fmt;

/// BUT chain ka error.
#[derive(Debug)]
pub enum ButError {
    /// Mnemonic / seed phrase galat hai.
    InvalidMnemonic(String),
    /// Kisi cheez ki length galat hai (na kam, na zyada — fixed spec).
    InvalidLength(String),
    /// Transaction ka signature galat hai ya verify nahi hua.
    InvalidSignature(String),
    /// Transaction galat hai (balance kam, nonce galat, amount zero, ...).
    InvalidTransaction(String),
    /// Block galat hai (hash mismatch, prev hash galat, validator galat, ...).
    InvalidBlock(String),
    /// BFT vote galat hai (signature galat, voter validator nahi hai, quorum nahi, ...).
    InvalidVote(String),
    /// Chain galat hai (verify fail).
    InvalidChain(String),
    /// Disk storage error.
    Storage(String),
    /// P2P network / HTTP API error.
    Network(String),
    /// Koi aur error.
    Other(String),
}

impl fmt::Display for ButError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ButError::InvalidMnemonic(m) => write!(f, "galat mnemonic: {m}"),
            ButError::InvalidLength(m) => write!(f, "galat length: {m}"),
            ButError::InvalidSignature(m) => write!(f, "galat signature: {m}"),
            ButError::InvalidTransaction(m) => write!(f, "galat transaction: {m}"),
            ButError::InvalidBlock(m) => write!(f, "galat block: {m}"),
            ButError::InvalidVote(m) => write!(f, "galat vote: {m}"),
            ButError::InvalidChain(m) => write!(f, "galat chain: {m}"),
            ButError::Storage(m) => write!(f, "storage error: {m}"),
            ButError::Network(m) => write!(f, "network error: {m}"),
            ButError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ButError {}

impl From<std::io::Error> for ButError {
    fn from(e: std::io::Error) -> Self {
        ButError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for ButError {
    fn from(e: serde_json::Error) -> Self {
        ButError::Storage(format!("chain file padha nahi gaya (JSON galat): {e}"))
    }
}

impl From<ed25519_dalek::SignatureError> for ButError {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        ButError::InvalidSignature(e.to_string())
    }
}

/// Chain ke functions ka Result type.
pub type Result<T> = std::result::Result<T, ButError>;
