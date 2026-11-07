//! HTTP/JSON API — Phase 2: local node API (std-only, koi framework nahi).
//!
//! `but-node serve` localhost par ek chhota HTTP server chalata hai, jisse
//! asli balance seedha chain se aata hai — localStorage wale fake ledger se nahi.
//!
//! Endpoints (sab JSON):
//! - `GET /` — node ki jaankari (height, latest hash, endpoints ki list).
//! - `GET /chain/height` — `{"height": N, "latest_hash": "..."}`.
//! - `GET /chain` — poori chain (blocks ke saath).
//! - `GET /block/<index>` — ek block, ya 404.
//! - `GET /balance/<address>` — `{"address", "balance", "balance_but",
//!   "locked", "locked_but", "unbonding", "claimable", "total", "total_but",
//!   "nonce"}`. Balance hamesha chain state se aata hai. Galat address par 400.
//! - `GET /mempool` — pending transactions + pending stake/swap/slash ops.
//! - `GET /swap/quote/<from>/<to>/<amount>` — swap se PEHLE quote (§11):
//!   kitna output milega, pool me kitna available hai, Hindi note.
//!   Amount smallest units me (BUT ke liye bits, BUT Coin ke liye uski
//!   smallest unit). Jaise `/swap/quote/BUT/BUTCOIN/50000`.
//! - `GET /swap/pools` — dono pools ki reserves (BUT alag, BUT Coin alag).
//! - `POST /swap` — signed SwapOp JSON body me bhejo (swap ya liquidity).
//!   Signature + routing rules verify hokar mempool me jayegi.
//! - `POST /slash` — signed SlashOp JSON body me bhejo (equivocation ka
//!   saboot). Saboot verify hokar mempool me jayegi.
//! - `GET /validators` — validator set (addresses, stake, jail status,
//!   min_stake, released_supply).
//! - `GET /staking` — released supply, min stake, locks, unbonding,
//!   team account/accrued (sab bits aur BUT dono me).
//! - `GET /stake/<address>` — us address ka lock/unbonding status.
//! - `POST /stake` — signed StakeOp JSON body me bhejo (stake/unstake/
//!   claim/register). Signature verify hokar mempool me jayegi.
//! - `POST /validators/register` — signed Register StakeOp bhejo —
//!   validator banne ke liye (locked stake >= min stake hona chahiye).
//! - `POST /tx` — transaction JSON body me bhejo (signature ke saath).
//!   Signature verify hokar mempool me jayegi. Bina signature ke 400.
//!   Receiver address ke saath-saath BUT Pay ID (jaise "100001") ya handle
//!   (jaise "@sanjay") bhi chalega — registry se resolve hota hai.
//! - `POST /mine` — mempool se block banao (txs + stake ops + swap ops +
//!   slash ops) aur BFT se chain me jodo — dev validator proposer hona
//!   chahiye aur self-vote karta hai. Chain disk par save hoti hai.
//! - `GET /butpay/<id-ya-handle>` — ID/handle se address nikalo.
//! - `POST /butpay/register` — naya BUT Pay ID lo: body
//!   `{"address": "but1...", "handle": "sanjay"}` (handle optional).
//!   Jawab: `{"ok": true, "id": 100001, "address": ..., "handle": ...}`.
//!
//! SACH BATA RAHE HAIN — kya abhi bhi simplified hai:
//! - **Localhost only, koi auth nahi**: ye API sirf apni machine par chalane
//!   ke liye hai. Is par koi password/API-key nahi hai — internet par khula
//!   chhoda to koi bhi transaction bhej sakta hai (galat tx waise bhi
//!   signature check par reject hogi, par spam ho sakta hai).
//! - **Dev validator**: `/mine` fixed demo validator (entropy `[9u8; 32]`)
//!   se block sign karta hai — wahi validator jo `but-node demo` ki genesis
//!   me hai. Production me validator key aise hardcode nahi hogi; wo
//!   secure keystore me hogi. `/mine` tabhi chalta hai jab dev validator
//!   us height ka proposer ho — doosre validator ke proposer hone par
//!   `/mine` 400 dega (BFT ka sahi vyavhaar; proposer hi block banata hai).
//! - **Mempool disk par save nahi hoti**: node band hua to pending
//!   transactions gayab. Chain disk par save hoti hai, mempool nahi.
//! - **HTTP, HTTPS nahi**: localhost par theek hai; internet par nahi.
//!
//! Web demo (client/) is phase me chheda nahi gaya — is API se UI jodne
//! ki wiring docs/REAL-NETWORK-PLAN.md me likhi hai.

use crate::block::Block;
use crate::butpay::ButPayRegistry;
use crate::chain::Chain;
use crate::consensus::{Vote, VoteTally};
use crate::error::{ButError, Result};
use crate::mempool::Mempool;
use crate::slash::SlashOp;
use crate::staking::StakeOp;
use crate::storage::{butpay_path_for, load_chain_with_registry, save_butpay, save_chain};
use crate::swap::{format_butcoin, SwapAsset, SwapOp};
use crate::transaction::Transaction;
use crate::units::{format_but, format_but_with_unit};
use crate::wallet::{validate_address, Wallet};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Demo/dev validator — wahi jo `but-node demo` ki genesis me hai.
/// Sirf local dev ke liye. Production me key aise nahi rakhi jayegi.
pub fn dev_validator() -> Wallet {
    Wallet::from_entropy(&[9u8; 32]).expect("dev validator wallet hamesha banta hai")
}

/// API ka shared state — P2P node ke saath bhi yahi Arcs share hote hain.
#[derive(Debug, Clone)]
pub struct ApiState {
    /// Node ki chain.
    pub chain: Arc<Mutex<Chain>>,
    /// Pending transactions.
    pub mempool: Arc<Mutex<Mempool>>,
    /// Chain file ka path (mine ke baad save karne ke liye).
    pub chain_path: PathBuf,
    /// BUT Pay registry — ID/handle → address. Disk par `butpay.json`
    /// (chain file ke saath) me save hoti hai.
    pub butpay: Arc<Mutex<ButPayRegistry>>,
    /// Block mine karne wala validator wallet.
    pub validator: Wallet,
}

impl ApiState {
    /// Chain file load karo; file na ho to demo genesis banao aur save karo.
    /// BUT Pay registry bhi saath me load/save hoti hai.
    pub fn load_or_create(chain_path: &Path) -> Result<Self> {
        let validator = dev_validator();
        let (chain, butpay) = if chain_path.exists() {
            load_chain_with_registry(chain_path)?
        } else {
            let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
            let chain = Chain::new_genesis(
                vec![(alice.address.clone(), 1_000_000)],
                &validator,
                now_secs(),
            )?;
            save_chain(&chain, chain_path)?;
            let butpay = ButPayRegistry::new();
            save_butpay(&butpay_path_for(chain_path), &butpay)?;
            (chain, butpay)
        };
        Ok(Self {
            chain: Arc::new(Mutex::new(chain)),
            mempool: Arc::new(Mutex::new(Mempool::new())),
            chain_path: chain_path.to_path_buf(),
            butpay: Arc::new(Mutex::new(butpay)),
            validator,
        })
    }

    /// BUT Pay ID register karo — naya ID do (ya handle), disk par save karo.
    /// Duplicate address/handle hua to error.
    pub fn butpay_register(&self, address: &str, handle: Option<&str>) -> Result<u64> {
        let mut reg = self.butpay.lock().unwrap();
        let id = reg.register(address, handle)?;
        save_butpay(&butpay_path_for(&self.chain_path), &reg)?;
        Ok(id)
    }

    /// Signed StakeOp mempool me dalo (signature + format verify hota hai).
    pub fn submit_stake_op(&self, op: StakeOp) -> Result<String> {
        let op_id = op.id();
        let mut pool = self.mempool.lock().unwrap();
        pool.add_stake_op(op)?;
        Ok(op_id)
    }

    /// Signed SwapOp mempool me dalo (signature + format + routing rules
    /// verify hote hain). Balance/pool ki final checking block apply karte
    /// waqt hoti hai.
    pub fn submit_swap_op(&self, op: SwapOp) -> Result<String> {
        let op_id = op.id();
        let mut pool = self.mempool.lock().unwrap();
        pool.add_swap_op(op)?;
        Ok(op_id)
    }

    /// Signed SlashOp mempool me dalo (saboot + signature verify hote hain).
    /// Nonce aur dedup ki final checking block apply karte waqt hoti hai.
    pub fn submit_slash_op(&self, op: SlashOp) -> Result<String> {
        let op_id = op.id();
        let mut pool = self.mempool.lock().unwrap();
        pool.add_slash_op(op)?;
        Ok(op_id)
    }

    /// Mempool se block mine karo: block banao (txs + stake ops + swap ops
    /// + slash ops), BFT se chain me jodo, disk par save karo.
    /// Dev validator us height ka proposer hona chahiye — warna 400.
    /// Self-vote karta hai (bootstrap quorum jab stake 0 ho, ya apna hissa).
    pub fn mine(&self) -> Result<Block> {
        let (txs, stake_ops, swap_ops, slash_ops) = {
            let mut pool = self.mempool.lock().unwrap();
            (
                pool.take_for_block(10),
                pool.take_stake_ops_for_block(10),
                pool.take_swap_ops_for_block(10),
                pool.take_slash_ops_for_block(10),
            )
        };
        if txs.is_empty() && stake_ops.is_empty() && swap_ops.is_empty() && slash_ops.is_empty() {
            return Err(ButError::InvalidTransaction(
                "mempool khaali hai — mine karne ko koi transaction ya op nahi hai".to_string(),
            ));
        }
        let (block, tally) = {
            let chain = self.chain.lock().unwrap();
            let height = chain.height() as u64;
            // Proposer check — dev validator hi proposer hona chahiye.
            match chain.validators.proposer_for_round(height) {
                Some(p) if p == self.validator.address => {}
                Some(p) => {
                    return Err(ButError::InvalidVote(format!(
                        "is height ka proposer {p} hai, dev validator nahi — /mine sirf proposer chala sakta hai"
                    )));
                }
                None => {
                    return Err(ButError::InvalidVote(
                        "koi active validator nahi hai — proposer nahi chuna ja sakta".to_string(),
                    ));
                }
            }
            let block = Block::new_with_ops(
                height,
                now_secs(),
                chain.latest_hash(),
                txs,
                stake_ops,
                swap_ops,
                slash_ops,
                self.validator.address.clone(),
                &self.validator.signing_key(),
            )?;
            let mut tally = VoteTally::new(height);
            let vote = Vote::new(
                block.hash.clone(),
                height,
                true,
                self.validator.address.clone(),
                &self.validator.signing_key(),
            )?;
            tally.add_vote(vote, &chain.validators)?;
            (block, tally)
        };
        {
            let mut chain = self.chain.lock().unwrap();
            let reg = self.butpay.lock().unwrap();
            chain.add_block_bft_with_registry(block.clone(), &tally, &reg)?;
            save_chain(&chain, &self.chain_path)?;
            save_butpay(&butpay_path_for(&self.chain_path), &reg)?;
        }
        Ok(block)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_700_000_000)
}

/// Ek HTTP request handle karo (ek connection = ek request).
pub fn handle_http(stream: TcpStream, state: &ApiState) -> Result<()> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
        ButError::Network(format!("connection clone nahi hua: {e}"))
    })?);
    let mut writer = stream;

    // Request line: "GET /balance/but1... HTTP/1.1"
    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|e| {
        ButError::Network(format!("request padha nahi gaya: {e}"))
    })?;
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return write_json(&mut writer, 400, json!({"ok": false, "error": "request samajh nahi aaya"}));
    }
    let method = parts[0];
    let path = parts[1];

    // Headers padho — sirf Content-Length chahiye.
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| {
            ButError::Network(format!("headers padhe nahi gaye: {e}"))
        })?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    // Body padho (POST ke liye).
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).map_err(|e| {
            ButError::Network(format!("body padha nahi gaya: {e}"))
        })?;
    }

    // CORS preflight — browser se aane wale fetch ke liye.
    // Ye API sirf local dev ke liye hai (localhost), isliye har origin ko
    // allow karna yahan theek hai. Internet par khule node par CORS
    // aise nahi chhodna — wahan allowlist + auth chahiye hoga.
    if method == "OPTIONS" {
        return write_cors_preflight(&mut writer);
    }

    let (status, payload) = route(method, path, &body, state);
    write_json(&mut writer, status, payload)
}

/// CORS preflight ka jawab (204, koi body nahi).
fn write_cors_preflight(writer: &mut TcpStream) -> Result<()> {
    writer
        .write_all(
            b"HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .and_then(|()| writer.flush())
        .map_err(|e| ButError::Network(format!("preflight jawab bheja nahi gaya: {e}")))?;
    Ok(())
}

/// Routing — method + path se JSON jawab banao. Network ke bina test karne
/// ke liye alag function hai.
fn route(method: &str, path: &str, body: &[u8], state: &ApiState) -> (u16, Value) {
    match (method, path) {
        ("GET", "/") => {
            let (height, latest_hash) = {
                let chain = state.chain.lock().unwrap();
                (chain.height(), chain.latest_hash())
            };
            (
                200,
                json!({
                    "ok": true,
                    "node": "but-node (Phase 3b local API)",
                    "height": height,
                    "latest_hash": latest_hash,
                    "endpoints": [
                        "GET /chain/height",
                        "GET /chain",
                        "GET /block/<index>",
                        "GET /balance/<address>",
                        "GET /mempool",
                        "GET /validators",
                        "GET /staking",
                        "GET /stake/<address>",
                        "POST /stake",
                        "POST /validators/register",
                        "GET /butpay/<id-ya-handle>",
                        "POST /butpay/register",
                        "GET /swap/quote/<from>/<to>/<amount>",
                        "GET /swap/pools",
                        "POST /swap",
                        "POST /slash",
                        "POST /tx",
                        "POST /mine",
                    ],
                }),
            )
        }
        ("GET", "/chain/height") => {
            let (height, latest_hash) = {
                let chain = state.chain.lock().unwrap();
                (chain.height(), chain.latest_hash())
            };
            (200, json!({"ok": true, "height": height, "latest_hash": latest_hash}))
        }
        ("GET", "/chain") => {
            let chain = state.chain.lock().unwrap();
            match serde_json::to_value(&*chain) {
                Ok(v) => (200, json!({"ok": true, "chain": v})),
                Err(e) => (500, json!({"ok": false, "error": format!("chain JSON nahi bana: {e}")})),
            }
        }
        ("GET", "/mempool") => {
            let pool = state.mempool.lock().unwrap();
            match (
                serde_json::to_value(pool.list()),
                serde_json::to_value(pool.stake_ops()),
                serde_json::to_value(pool.swap_ops()),
                serde_json::to_value(pool.slash_ops()),
            ) {
                (Ok(txs), Ok(stake_ops), Ok(swap_ops), Ok(slash_ops)) => (
                    200,
                    json!({"ok": true, "pending": txs, "stake_ops": stake_ops, "swap_ops": swap_ops, "slash_ops": slash_ops}),
                ),
                _ => (500, json!({"ok": false, "error": "mempool JSON nahi bana"})),
            }
        }
        ("GET", "/swap/pools") => {
            let chain = state.chain.lock().unwrap();
            let swap = &chain.swap;
            (
                200,
                json!({
                    "ok": true,
                    "but_pool": swap.but_pool,
                    "but_pool_but": format_but(swap.but_pool),
                    "butcoin_pool": swap.butcoin_pool,
                    "butcoin_pool_coins": format_butcoin(swap.butcoin_pool),
                    "note": "Dono pools alag hain — BUT ka pool aur BUT Coin ka pool kabhi mix nahi hote (§13). Swap sirf available liquidity tak hota hai (§11).",
                }),
            )
        }
        ("GET", p) if p.starts_with("/swap/quote/") => {
            // /swap/quote/<from>/<to>/<amount> — amount smallest units me.
            let rest = &p["/swap/quote/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 3 {
                return (
                    400,
                    json!({"ok": false, "error": "quote URL aisi honi chahiye: /swap/quote/BUT/BUTCOIN/50000"}),
                );
            }
            let (from_s, to_s, amount_s) = (parts[0], parts[1], parts[2]);
            let amount: u64 = match amount_s.parse() {
                Ok(n) => n,
                Err(_) => {
                    return (
                        400,
                        json!({"ok": false, "error": format!("amount number hona chahiye, mila: {amount_s}")}),
                    )
                }
            };
            let (from, to) = match (SwapAsset::parse(from_s), SwapAsset::parse(to_s)) {
                (Ok(f), Ok(t)) => (f, t),
                (Err(e), _) | (_, Err(e)) => {
                    return (400, json!({"ok": false, "error": e.to_string()}));
                }
            };
            let chain = state.chain.lock().unwrap();
            match chain.swap.quote(&from, &to, amount) {
                Ok(quote) => (200, json!({"ok": true, "quote": quote})),
                Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
            }
        }
        ("POST", "/swap") => {
            let op: std::result::Result<SwapOp, _> = serde_json::from_slice(body);
            match op {
                Ok(op) => match state.submit_swap_op(op) {
                    Ok(op_id) => (200, json!({"ok": true, "op_id": op_id})),
                    Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
                },
                Err(e) => (
                    400,
                    json!({"ok": false, "error": format!("swap op JSON samajh nahi aaya: {e}")}),
                ),
            }
        }
        ("POST", "/slash") => {
            let op: std::result::Result<SlashOp, _> = serde_json::from_slice(body);
            match op {
                Ok(op) => match state.submit_slash_op(op) {
                    Ok(op_id) => (200, json!({"ok": true, "op_id": op_id})),
                    Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
                },
                Err(e) => (
                    400,
                    json!({"ok": false, "error": format!("slash op JSON samajh nahi aaya: {e}")}),
                ),
            }
        }
        ("GET", "/validators") => {
            let chain = state.chain.lock().unwrap();
            let vs = &chain.validators;
            (
                200,
                json!({
                    "ok": true,
                    "validators": vs.validators,
                    "stakes": vs.stakes,
                    "jailed": vs.jailed,
                    "total_active_stake": vs.total_active_stake(),
                    "min_stake": chain.min_stake(),
                    "min_stake_but": format_but(chain.min_stake()),
                    "released_supply": chain.released_supply(),
                    "released_supply_but": format_but(chain.released_supply()),
                }),
            )
        }
        ("GET", "/staking") => {
            let chain = state.chain.lock().unwrap();
            let locks: Vec<Value> = chain
                .staking
                .locks
                .iter()
                .map(|(addr, lock)| {
                    json!({
                        "address": addr,
                        "locked": lock.amount,
                        "locked_but": format_but(lock.amount),
                        "tier_days": lock.tier.lock_seconds() / 86_400,
                        "locked_at": lock.locked_at,
                        "unlock_at": lock.unlock_at(),
                    })
                })
                .collect();
            let unbonding: Vec<Value> = chain
                .staking
                .unbonding
                .iter()
                .map(|u| {
                    json!({
                        "staker": u.staker,
                        "total": u.total_amount,
                        "released": u.released_amount,
                        "remaining": u.total_amount - u.released_amount,
                        "started_at": u.started_at,
                    })
                })
                .collect();
            (
                200,
                json!({
                    "ok": true,
                    "released_supply": chain.released_supply(),
                    "released_supply_but": format_but(chain.released_supply()),
                    "min_stake": chain.min_stake(),
                    "min_stake_but": format_but(chain.min_stake()),
                    "min_stake_percent": 1,
                    "team_account": chain.team_account,
                    "team_accrued": chain.team_accrued,
                    "team_accrued_but": format_but(chain.team_accrued),
                    "next_block_reward": chain.next_block_reward(),
                    "next_block_reward_but": format_but(chain.next_block_reward()),
                    "locks": locks,
                    "unbonding": unbonding,
                }),
            )
        }
        ("GET", p) if p.starts_with("/stake/") => {
            let address = &p["/stake/".len()..];
            match validate_address(address) {
                Ok(()) => {
                    let chain = state.chain.lock().unwrap();
                    let lock = chain.staking.locks.get(address);
                    let claimable = chain.claimable_balance(address, now_secs());
                    (
                        200,
                        json!({
                            "ok": true,
                            "address": address,
                            "locked": chain.locked_balance(address),
                            "locked_but": format_but(chain.locked_balance(address)),
                            "tier_days": lock.map(|l| l.tier.lock_seconds() / 86_400),
                            "unlock_at": lock.map(|l| l.unlock_at()),
                            "unbonding": chain.unbonding_balance(address),
                            "unbonding_but": format_but(chain.unbonding_balance(address)),
                            "claimable": claimable,
                            "claimable_but": format_but(claimable),
                            "min_stake": chain.min_stake(),
                        }),
                    )
                }
                Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
            }
        }
        ("POST", "/stake") => {
            let op: std::result::Result<StakeOp, _> = serde_json::from_slice(body);
            match op {
                Ok(op) => match state.submit_stake_op(op) {
                    Ok(op_id) => (200, json!({"ok": true, "op_id": op_id})),
                    Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
                },
                Err(e) => (
                    400,
                    json!({"ok": false, "error": format!("stake op JSON samajh nahi aaya: {e}")}),
                ),
            }
        }
        ("POST", "/validators/register") => {
            let op: std::result::Result<StakeOp, _> = serde_json::from_slice(body);
            match op {
                Ok(op) => {
                    if op.kind != crate::staking::StakeOpKind::Register {
                        return (
                            400,
                            json!({"ok": false, "error": "is endpoint par sirf Register op chalega"}),
                        );
                    }
                    match state.submit_stake_op(op) {
                        Ok(op_id) => (200, json!({"ok": true, "op_id": op_id})),
                        Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
                    }
                }
                Err(e) => (
                    400,
                    json!({"ok": false, "error": format!("register op JSON samajh nahi aaya: {e}")}),
                ),
            }
        }
        ("POST", "/tx") => {
            let tx: std::result::Result<Transaction, _> = serde_json::from_slice(body);
            match tx {
                Ok(tx) => {
                    let tx_id = tx.id();
                    // Receiver ID/handle bhi ho sakta hai — registry se
                    // resolve karke verify hota hai.
                    let reg = state.butpay.lock().unwrap().clone();
                    let mut pool = state.mempool.lock().unwrap();
                    match pool.add_with_registry(tx, &reg) {
                        Ok(()) => (200, json!({"ok": true, "tx_id": tx_id})),
                        Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
                    }
                }
                Err(e) => (
                    400,
                    json!({"ok": false, "error": format!("transaction JSON samajh nahi aaya: {e}")}),
                ),
            }
        }
        ("POST", "/butpay/register") => {
            let parsed: std::result::Result<Value, _> = serde_json::from_slice(body);
            match parsed {
                Ok(v) => {
                    let address = v.get("address").and_then(|a| a.as_str()).unwrap_or("");
                    let handle = v.get("handle").and_then(|h| h.as_str());
                    match state.butpay_register(address, handle) {
                        Ok(id) => {
                            let reg = state.butpay.lock().unwrap();
                            let entry = reg.lookup(&id.to_string());
                            (
                                200,
                                json!({
                                    "ok": true,
                                    "id": id,
                                    "address": address,
                                    "handle": entry.and_then(|e| e.handle.clone()),
                                }),
                            )
                        }
                        Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
                    }
                }
                Err(e) => (
                    400,
                    json!({"ok": false, "error": format!("register JSON samajh nahi aaya: {e}")}),
                ),
            }
        }
        ("GET", p) if p.starts_with("/butpay/") => {
            let query = &p["/butpay/".len()..];
            // URL-decoding: handle me "%40" (@) aa sakta hai.
            let query = query.replace("%40", "@");
            let reg = state.butpay.lock().unwrap();
            match reg.lookup(&query) {
                Some(entry) => (
                    200,
                    json!({
                        "ok": true,
                        "id": entry.id,
                        "handle": entry.handle,
                        "address": entry.address,
                    }),
                ),
                None => (
                    404,
                    json!({"ok": false, "error": format!("BUT Pay ID/handle '{query}' nahi mila")}),
                ),
            }
        }
        ("POST", "/mine") => match state.mine() {
            Ok(block) => {
                let height = state.chain.lock().unwrap().height();
                (
                    200,
                    json!({"ok": true, "height": height, "hash": block.hash, "tx_count": block.transactions.len()}),
                )
            }
            Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
        },
        ("GET", p) if p.starts_with("/balance/") => {
            let address = &p["/balance/".len()..];
            match validate_address(address) {
                Ok(()) => {
                    let chain = state.chain.lock().unwrap();
                    let balance = chain.balance(address);
                    let butcoin = chain.butcoin_balance(address);
                    let locked = chain.locked_balance(address);
                    let unbonding = chain.unbonding_balance(address);
                    let total = chain.total_balance(address);
                    (
                        200,
                        json!({
                            "ok": true,
                            "address": address,
                            "balance": balance,
                            "balance_but": format_but(balance),
                            "balance_display": format_but_with_unit(balance),
                            "butcoin_balance": butcoin,
                            "butcoin_balance_coins": format_butcoin(butcoin),
                            "locked": locked,
                            "locked_but": format_but(locked),
                            "unbonding": unbonding,
                            "unbonding_but": format_but(unbonding),
                            "claimable": chain.claimable_balance(address, now_secs()),
                            "claimable_but": format_but(chain.claimable_balance(address, now_secs())),
                            "total": total,
                            "total_but": format_but(total),
                            "nonce": chain.next_nonce(address),
                        }),
                    )
                }
                Err(e) => (400, json!({"ok": false, "error": e.to_string()})),
            }
        }
        ("GET", p) if p.starts_with("/block/") => {
            let index: std::result::Result<usize, _> = p["/block/".len()..].parse();
            match index {
                Ok(i) => {
                    let chain = state.chain.lock().unwrap();
                    match chain.blocks.get(i) {
                        Some(block) => match serde_json::to_value(block) {
                            Ok(v) => (200, json!({"ok": true, "block": v})),
                            Err(e) => (
                                500,
                                json!({"ok": false, "error": format!("block JSON nahi bana: {e}")}),
                            ),
                        },
                        None => (
                            404,
                            json!({"ok": false, "error": format!("block {i} nahi hai — height {} hai", chain.height())}),
                        ),
                    }
                }
                Err(_) => (400, json!({"ok": false, "error": "block index number hona chahiye"})),
            }
        }
        _ => (
            404,
            json!({"ok": false, "error": format!("unknown route: {method} {path}")}),
        ),
    }
}

/// JSON jawab HTTP response ke roop me likho.
fn write_json(writer: &mut TcpStream, status: u16, payload: Value) -> Result<()> {
    let body = serde_json::to_string(&payload)
        .map_err(|e| ButError::Network(format!("jawab JSON nahi bana: {e}")))?;
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    writer
        .write_all(response.as_bytes())
        .and_then(|()| writer.flush())
        .map_err(|e| ButError::Network(format!("jawab bheja nahi gaya: {e}")))?;
    Ok(())
}

/// Listener par `n` requests handle karke band ho jao (tests ke liye).
pub fn serve_n(listener: TcpListener, state: &ApiState, n: usize) -> Result<()> {
    for stream in listener.incoming().take(n) {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_http(stream, state) {
                    eprintln!("api connection error: {e}");
                }
            }
            Err(e) => {
                return Err(ButError::Network(format!(
                    "connection accept nahi hui: {e}"
                )))
            }
        }
    }
    Ok(())
}

/// Hamesha chalo — har connection ke liye naya thread.
pub fn serve_forever(listener: TcpListener, state: &ApiState) -> Result<()> {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_http(stream, &state) {
                        eprintln!("api connection error: {e}");
                    }
                });
            }
            Err(e) => {
                return Err(ButError::Network(format!(
                    "connection accept nahi hui: {e}"
                )))
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Transaction;
    use crate::wallet::Wallet;
    use std::io::Read;

    fn test_state() -> ApiState {
        // Har test ko apna alag temp dir — warna parallel tests ek hi
        // chain.json par ladte hain aur flaky fail hote hain.
        static COUNTER: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("but_api_test_state_{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("chain.json");
        ApiState::load_or_create(&path).unwrap()
    }

    fn serve_test_api(state: &ApiState, n: usize) -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let state = state.clone();
        let handle = std::thread::spawn(move || {
            serve_n(listener, &state, n).unwrap();
        });
        (port, handle)
    }

    /// Raw HTTP request bhejo, poora response string lo.
    fn http_raw(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    fn body_of(response: &str) -> &str {
        response.split("\r\n\r\n").nth(1).unwrap_or("")
    }

    #[test]
    fn height_aur_balance_endpoint() {
        let state = test_state();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let (port, handle) = serve_test_api(&state, 2);
        let r1 = http_raw(port, "GET /chain/height HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r1.starts_with("HTTP/1.1 200 OK"));
        assert!(body_of(&r1).contains("\"height\":1"));
        let req = format!(
            "GET /balance/{} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
            alice.address
        );
        let r2 = http_raw(port, &req);
        assert!(r2.starts_with("HTTP/1.1 200 OK"));
        // Genesis me Alice ko 1,000,000 mile the.
        assert!(body_of(&r2).contains("\"balance\":1000000"));
        handle.join().unwrap();
    }

    #[test]
    fn tx_submit_mine_balance_badalta_hai() {
        let state = test_state();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let tx_json = serde_json::to_string(&tx).unwrap();
        let (port, handle) = serve_test_api(&state, 3);
        // 1. POST /tx
        let req = format!(
            "POST /tx HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            tx_json.len(),
            tx_json
        );
        let r1 = http_raw(port, &req);
        assert!(r1.starts_with("HTTP/1.1 200 OK"), "tx submit fail: {r1}");
        assert!(body_of(&r1).contains("\"ok\":true"));
        // 2. POST /mine
        let r2 = http_raw(port, "POST /mine HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        assert!(r2.starts_with("HTTP/1.1 200 OK"), "mine fail: {r2}");
        assert!(body_of(&r2).contains("\"height\":2"));
        // 3. Bob ka balance ab 100 hona chahiye — chain se aaya, fake nahi.
        let req3 = format!(
            "GET /balance/{} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
            bob.address
        );
        let r3 = http_raw(port, &req3);
        assert!(body_of(&r3).contains("\"balance\":100"));
        handle.join().unwrap();
    }

    #[test]
    fn bina_signature_tx_aur_galat_address_reject() {
        let state = test_state();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let bad_tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        let tx_json = serde_json::to_string(&bad_tx).unwrap();
        let (port, handle) = serve_test_api(&state, 2);
        let req = format!(
            "POST /tx HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            tx_json.len(),
            tx_json
        );
        let r1 = http_raw(port, &req);
        assert!(r1.starts_with("HTTP/1.1 400"), "unsigned tx 400 honi chahiye: {r1}");
        assert!(body_of(&r1).contains("\"ok\":false"));
        let r2 = http_raw(port, "GET /balance/but1abc HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r2.starts_with("HTTP/1.1 400"));
        handle.join().unwrap();
    }

    #[test]
    fn butpay_register_lookup_aur_id_se_tx() {
        let state = test_state();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let (port, handle) = serve_test_api(&state, 6);
        // 1. POST /butpay/register — Bob ko ID + handle do.
        let reg_body = serde_json::to_string(&json!({"address": bob.address, "handle": "bob"})).unwrap();
        let req = format!(
            "POST /butpay/register HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            reg_body.len(),
            reg_body
        );
        let r1 = http_raw(port, &req);
        assert!(r1.starts_with("HTTP/1.1 200 OK"), "register fail: {r1}");
        assert!(body_of(&r1).contains("\"id\":100001"), "pehli ID 100001 honi chahiye: {}", body_of(&r1));
        // 2. GET /butpay/100001 — address milega.
        let r2 = http_raw(port, "GET /butpay/100001 HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r2.starts_with("HTTP/1.1 200 OK"), "lookup fail: {r2}");
        assert!(body_of(&r2).contains(&bob.address));
        assert!(body_of(&r2).contains("\"handle\":\"bob\""));
        // 3. GET /butpay/@bob — handle se bhi milega.
        let r3 = http_raw(port, "GET /butpay/%40bob HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r3.starts_with("HTTP/1.1 200 OK"), "handle lookup fail: {r3}");
        assert!(body_of(&r3).contains(&bob.address));
        // 4. POST /tx — receiver ID "100001" se bhejo, mine karo.
        let mut tx = Transaction::new(alice.address.clone(), "100001".to_string(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let tx_json = serde_json::to_string(&tx).unwrap();
        let req4 = format!(
            "POST /tx HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            tx_json.len(),
            tx_json
        );
        let r4 = http_raw(port, &req4);
        assert!(r4.starts_with("HTTP/1.1 200 OK"), "ID tx fail: {r4}");
        let r5 = http_raw(port, "POST /mine HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        assert!(r5.starts_with("HTTP/1.1 200 OK"), "mine fail: {r5}");
        // Bob ka balance 100 — ID se bheja paisa address par pahuncha.
        let req6 = format!(
            "GET /balance/{} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
            bob.address
        );
        let r6 = http_raw(port, &req6);
        assert!(body_of(&r6).contains("\"balance\":100"), "bob balance: {}", body_of(&r6));
        handle.join().unwrap();
    }

    #[test]
    fn cors_preflight_aur_header_milte_hain() {
        // Browser se fetch ke liye: OPTIONS preflight 204 + CORS headers,
        // aur normal GET jawab me bhi Access-Control-Allow-Origin.
        let state = test_state();
        let (port, handle) = serve_test_api(&state, 2);
        let r1 = http_raw(
            port,
            "OPTIONS /balance/but1abc HTTP/1.1\r\nHost: x\r\nOrigin: http://localhost:5173\r\nAccess-Control-Request-Method: GET\r\nConnection: close\r\n\r\n",
        );
        assert!(r1.starts_with("HTTP/1.1 204"), "preflight 204 honi chahiye: {r1}");
        assert!(r1.contains("Access-Control-Allow-Origin: *"), "preflight me CORS header: {r1}");
        let r2 = http_raw(port, "GET /chain/height HTTP/1.1\r\nHost: x\r\nOrigin: http://localhost:5173\r\nConnection: close\r\n\r\n");
        assert!(r2.starts_with("HTTP/1.1 200 OK"), "height 200: {r2}");
        assert!(r2.contains("Access-Control-Allow-Origin: *"), "GET jawab me CORS header: {r2}");
        handle.join().unwrap();
    }

    #[test]
    fn stake_endpoint_aur_staking_status() {
        use crate::staking::{LockTier, StakeOp};
        let state = test_state();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        // Alice 100,000 bits stake karti hai (tier 90 din), nonce 0.
        let mut op = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 0);
        op.sign(&alice.signing_key()).unwrap();
        let op_json = serde_json::to_string(&op).unwrap();
        let (port, handle) = serve_test_api(&state, 5);
        // 1. POST /stake — mempool me jayega.
        let req = format!(
            "POST /stake HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            op_json.len(),
            op_json
        );
        let r1 = http_raw(port, &req);
        assert!(r1.starts_with("HTTP/1.1 200 OK"), "stake submit fail: {r1}");
        assert!(body_of(&r1).contains("\"ok\":true"));
        // 2. POST /mine — stake op block me jayega.
        let r2 = http_raw(port, "POST /mine HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        assert!(r2.starts_with("HTTP/1.1 200 OK"), "mine fail: {r2}");
        // 3. GET /stake/<alice> — locked 100,000 dikhna chahiye.
        let req3 = format!(
            "GET /stake/{} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
            alice.address
        );
        let r3 = http_raw(port, &req3);
        assert!(r3.starts_with("HTTP/1.1 200 OK"), "stake status fail: {r3}");
        assert!(body_of(&r3).contains("\"locked\":100000"), "locked: {}", body_of(&r3));
        // format_but trailing zeros hatata hai: 100,000 bits = "100" BUT.
        assert!(body_of(&r3).contains("\"locked_but\":\"100\""), "locked_but: {}", body_of(&r3));
        // 4. GET /balance/<alice> — spendable 900,000, total 1,000,000.
        let req4 = format!(
            "GET /balance/{} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
            alice.address
        );
        let r4 = http_raw(port, &req4);
        assert!(body_of(&r4).contains("\"balance\":900000"), "balance: {}", body_of(&r4));
        assert!(body_of(&r4).contains("\"total\":1000000"), "total: {}", body_of(&r4));
        // 5. GET /staking — 1 block mine hua: released 1,005,000,
        // min stake uska 1% = 10,050.
        let r5 = http_raw(port, "GET /staking HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r5.starts_with("HTTP/1.1 200 OK"), "staking fail: {r5}");
        assert!(body_of(&r5).contains("\"min_stake\":10050"), "min_stake: {}", body_of(&r5));
        assert!(body_of(&r5).contains("\"released_supply\":1005000"), "released: {}", body_of(&r5));
        assert!(body_of(&r5).contains("\"team_accrued\":5"), "team_accrued: {}", body_of(&r5));
        handle.join().unwrap();
    }

    #[test]
    fn bina_signature_stake_op_aur_galat_register_reject() {
        use crate::staking::{LockTier, StakeOp};
        let state = test_state();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        // Bina signature wali stake op.
        let op = StakeOp::stake(alice.address.clone(), 10_000, LockTier::Days90, 0);
        let op_json = serde_json::to_string(&op).unwrap();
        let (port, handle) = serve_test_api(&state, 2);
        let req = format!(
            "POST /stake HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            op_json.len(),
            op_json
        );
        let r1 = http_raw(port, &req);
        assert!(r1.starts_with("HTTP/1.1 400"), "unsigned stake op 400 honi chahiye: {r1}");
        // /validators/register par Stake op (Register nahi) — 400.
        let mut op2 = StakeOp::stake(alice.address.clone(), 10_000, LockTier::Days90, 0);
        op2.sign(&alice.signing_key()).unwrap();
        let op2_json = serde_json::to_string(&op2).unwrap();
        let req2 = format!(
            "POST /validators/register HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            op2_json.len(),
            op2_json
        );
        let r2 = http_raw(port, &req2);
        assert!(r2.starts_with("HTTP/1.1 400"), "galat kind register par 400: {r2}");
        handle.join().unwrap();
    }

    #[test]
    fn validators_endpoint_me_min_stake_aur_released_supply() {
        let state = test_state();
        let (port, handle) = serve_test_api(&state, 1);
        let r1 = http_raw(port, "GET /validators HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r1.starts_with("HTTP/1.1 200 OK"), "validators fail: {r1}");
        assert!(body_of(&r1).contains("\"min_stake\":10000"), "min_stake: {}", body_of(&r1));
        assert!(body_of(&r1).contains("\"released_supply\":1000000"), "released: {}", body_of(&r1));
        handle.join().unwrap();
    }

    #[test]
    fn swap_quote_pools_swap_mine_flow() {
        use crate::swap::{SwapAsset, SwapOp};
        // Custom genesis: Alice ko BUT + BUT Coin dono, Bob ko BUT.
        let validator = dev_validator();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let dir = std::env::temp_dir().join("but_api_test_swap_flow");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let chain = Chain::new_genesis_full(
            vec![
                (alice.address.clone(), 1_000_000),
                (bob.address.clone(), 500_000),
            ],
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            now_secs(),
            None,
        )
        .unwrap();
        let state = ApiState {
            chain: Arc::new(Mutex::new(chain)),
            mempool: Arc::new(Mutex::new(Mempool::new())),
            chain_path: dir.join("chain.json"),
            butpay: Arc::new(Mutex::new(ButPayRegistry::new())),
            validator,
        };
        let (port, handle) = serve_test_api(&state, 9);
        // 1. GET /swap/pools — shuru me dono khaali.
        let r1 = http_raw(port, "GET /swap/pools HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r1.starts_with("HTTP/1.1 200 OK"), "pools fail: {r1}");
        assert!(body_of(&r1).contains("\"but_pool\":0"));
        assert!(body_of(&r1).contains("\"butcoin_pool\":0"));
        // 2. POST /swap — Alice BUT liquidity 100,000 (nonce 0).
        let mut liq_but = SwapOp::add_liquidity(SwapAsset::But, 100_000, 0, alice.address.clone());
        liq_but.sign(&alice.signing_key()).unwrap();
        let liq_but_json = serde_json::to_string(&liq_but).unwrap();
        let req2 = format!(
            "POST /swap HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            liq_but_json.len(),
            liq_but_json
        );
        let r2 = http_raw(port, &req2);
        assert!(r2.starts_with("HTTP/1.1 200 OK"), "liquidity BUT fail: {r2}");
        // 3. POST /swap — Alice BUT Coin liquidity 500,000 (nonce 1).
        let mut liq_coin =
            SwapOp::add_liquidity(SwapAsset::ButCoin, 500_000, 1, alice.address.clone());
        liq_coin.sign(&alice.signing_key()).unwrap();
        let liq_coin_json = serde_json::to_string(&liq_coin).unwrap();
        let req3 = format!(
            "POST /swap HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            liq_coin_json.len(),
            liq_coin_json
        );
        let r3 = http_raw(port, &req3);
        assert!(r3.starts_with("HTTP/1.1 200 OK"), "liquidity BUT Coin fail: {r3}");
        // 4. POST /mine — dono liquidity ops block me jayenge.
        let r4 = http_raw(port, "POST /mine HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        assert!(r4.starts_with("HTTP/1.1 200 OK"), "mine fail: {r4}");
        // 5. GET /swap/quote/BUT/BUTCOIN/50000 — out 166,666 hona chahiye.
        let r5 = http_raw(port, "GET /swap/quote/BUT/BUTCOIN/50000 HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r5.starts_with("HTTP/1.1 200 OK"), "quote fail: {r5}");
        assert!(body_of(&r5).contains("\"expected_out\":166666"), "quote: {}", body_of(&r5));
        assert!(body_of(&r5).contains("\"available_out\":500000"), "available: {}", body_of(&r5));
        // 6. POST /swap — Bob 50,000 BUT → BUT Coin (nonce 0).
        let mut sw = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            50_000,
            0,
            bob.address.clone(),
        );
        sw.sign(&bob.signing_key()).unwrap();
        let sw_json = serde_json::to_string(&sw).unwrap();
        let req6 = format!(
            "POST /swap HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            sw_json.len(),
            sw_json
        );
        let r6 = http_raw(port, &req6);
        assert!(r6.starts_with("HTTP/1.1 200 OK"), "swap fail: {r6}");
        // 7. POST /mine — swap block me jayega.
        let r7 = http_raw(port, "POST /mine HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        assert!(r7.starts_with("HTTP/1.1 200 OK"), "mine 2 fail: {r7}");
        // 8. GET /balance/<bob> — BUT Coin 166,666 milna chahiye.
        let req8 = format!(
            "GET /balance/{} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
            bob.address
        );
        let r8 = http_raw(port, &req8);
        assert!(body_of(&r8).contains("\"butcoin_balance\":166666"), "bob butcoin: {}", body_of(&r8));
        // 9. GET /swap/pools — BUT 150,000, BUT Coin 333,334.
        let r9 = http_raw(port, "GET /swap/pools HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(body_of(&r9).contains("\"but_pool\":150000"), "pool but: {}", body_of(&r9));
        assert!(body_of(&r9).contains("\"butcoin_pool\":333334"), "pool coin: {}", body_of(&r9));
        handle.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn swap_quote_btc_se_butcoin_reject_aur_slash_submit() {
        use crate::consensus::Vote;
        use crate::slash::SlashOp;
        use crate::swap::{SwapAsset, SwapOp};
        let state = test_state();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let (port, handle) = serve_test_api(&state, 3);
        // 1. GET /swap/quote/BTC/BUTCOIN/1000 — §14 routing reject (400).
        let r1 = http_raw(port, "GET /swap/quote/BTC/BUTCOIN/1000 HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r1.starts_with("HTTP/1.1 400"), "BTC quote 400 honi chahiye: {r1}");
        assert!(body_of(&r1).contains("BTC"));
        // 2. POST /swap — BTC → BUT Coin op bhi reject (400).
        let mut bad = SwapOp::swap(
            SwapAsset::External("BTC".to_string()),
            SwapAsset::ButCoin,
            1_000,
            0,
            alice.address.clone(),
        );
        bad.sign(&alice.signing_key()).unwrap();
        let bad_json = serde_json::to_string(&bad).unwrap();
        let req2 = format!(
            "POST /swap HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            bad_json.len(),
            bad_json
        );
        let r2 = http_raw(port, &req2);
        assert!(r2.starts_with("HTTP/1.1 400"), "BTC swap 400 hona chahiye: {r2}");
        // 3. POST /slash — Alice ke khilaf equivocation saboot (Bob reporter).
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
        let mut slash = SlashOp::new(
            alice.address.clone(),
            v1,
            v2,
            "equivocation height 3".to_string(),
            bob.address.clone(),
            0,
        );
        slash.sign(&bob.signing_key()).unwrap();
        let slash_json = serde_json::to_string(&slash).unwrap();
        let req3 = format!(
            "POST /slash HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            slash_json.len(),
            slash_json
        );
        let r3 = http_raw(port, &req3);
        assert!(r3.starts_with("HTTP/1.1 200 OK"), "slash submit fail: {r3}");
        assert!(body_of(&r3).contains("\"ok\":true"));
        handle.join().unwrap();
    }

    #[test]
    fn unknown_route_404_aur_block_endpoint() {
        let state = test_state();
        let (port, handle) = serve_test_api(&state, 2);
        let r1 = http_raw(port, "GET /kuch-bhi-nahi HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r1.starts_with("HTTP/1.1 404"));
        let r2 = http_raw(port, "GET /block/0 HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r2.starts_with("HTTP/1.1 200 OK"));
        assert!(body_of(&r2).contains("\"index\":0"));
        handle.join().unwrap();
    }
}
