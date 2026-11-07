//! BUT Network — real blockchain core (Phase 1 foundation).
//!
//! Ye Rust core hai — strategy.md Phase 2 (Core) ka shuruwaati hissa.
//! Isme REAL cheezein hain: real block, real chain, real Ed25519
//! signature wali transaction, real BIP39 wallet, disk storage.
//!
//! SACH BATA RAHE HAIN — ye foundation hai, mainnet-grade NAHI:
//! - P2P network Phase 2 me aa gaya hai (plain TCP, `p2p` module) —
//!   nodes jud sakte hain aur chain sync kar sakte hain. Par encryption,
//!   peer discovery aur fork-choice/reorg abhi nahi hain.
//! - Consensus Phase 2 me PoS + BFT ki machinery hai (`consensus` module:
//!   staking, proposer chunav, 2/3 vote, slashing). Slashing abhi
//!   validator-set bookkeeping hai — chain-state stake-locking baaki hai.
//!   BFT block-add opt-in hai (`Chain::add_block_bft`); purana
//!   `add_block` abhi bhi PoA hai.
//! - HTTP API aa gayi hai (`api` module) — asli balance chain se aata hai.
//!   Par ye localhost-only dev API hai, koi auth nahi hai.
//! - Koi independent audit nahi hua hai.
//! - Tokenomics abhi PROPOSAL hai (docs/TOKENOMICS-DRAFT.md) — final nahi.
//! - Ye core abhi web demo se juda nahi hai — web demo abhi bhi
//!   localStorage wala fake ledger dikhata hai (docs/REAL-NETWORK-PLAN.md dekho).
//!
//! Modules:
//! - `wallet`: BIP39 24-word mnemonic, Ed25519 keypair, fixed lengths.
//! - `transaction`: sign/verify wali transaction.
//! - `merkle`: Merkle root.
//! - `block`: block + hash + validator signature.
//! - `mempool`: pending transactions.
//! - `consensus`: Phase 2 PoS + BFT — staking, proposer, vote, quorum, slashing.
//! - `chain`: genesis, block add (PoA + BFT path), balances, nonce, full verify.
//! - `storage`: chain disk par save/load (JSON, verify ke saath).
//! - `p2p`: Phase 2 plain-TCP P2P — height handshake, block sync, tx gossip.
//! - `api`: Phase 2 local HTTP/JSON API — balance, height, tx submit, mine.
//! - `butpay`: BUT Pay — chhoti numeric ID + handle → wallet address registry.
//! - `hash`: SHA-256 helpers.

pub mod api;
pub mod block;
pub mod butpay;
pub mod chain;
pub mod consensus;
pub mod error;
pub mod faucet;
pub mod hash;
pub mod mempool;
pub mod merkle;
pub mod p2p;
pub mod slash;
pub mod staking;
pub mod storage;
pub mod swap;
pub mod transaction;
pub mod units;
pub mod wallet;

pub use block::Block;
pub use butpay::{ButPayRegistry, BUTPAY_FIRST_ID};
pub use chain::Chain;
pub use consensus::{ValidatorSet, Vote, VoteTally};
pub use error::{ButError, Result};
pub use faucet::{Faucet, FaucetConfig};
pub use mempool::Mempool;
pub use p2p::Node;
pub use slash::SlashOp;
pub use staking::{LockTier, StakeOp, StakeOpKind, StakingState};
pub use swap::{SwapAsset, SwapOp, SwapOpKind, SwapQuote, SwapState, BUTCOIN_PER_COIN};
pub use transaction::Transaction;
pub use units::{format_but, format_but_with_unit, parse_but};
pub use wallet::Wallet;
