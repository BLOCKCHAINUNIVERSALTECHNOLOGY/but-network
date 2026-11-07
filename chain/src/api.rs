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
//! - `GET /balance/<address>` — `{"address", "balance", "nonce"}`.
//!   Balance hamesha chain state se aata hai. Galat address par 400.
//! - `GET /mempool` — pending transactions.
//! - `GET /validators` — validator set (addresses, stake, jail status).
//! - `POST /tx` — transaction JSON body me bhejo (signature ke saath).
//!   Signature verify hokar mempool me jayegi. Bina signature ke 400.
//!   Receiver address ke saath-saath BUT Pay ID (jaise "100001") ya handle
//!   (jaise "@sanjay") bhi chalega — registry se resolve hota hai.
//! - `POST /mine` — mempool se block banao aur chain me jodo (dev validator
//!   se sign hota hai), chain disk par save hoti hai.
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
//!   secure keystore me hogi.
//! - **Mempool disk par save nahi hoti**: node band hua to pending
//!   transactions gayab. Chain disk par save hoti hai, mempool nahi.
//! - **HTTP, HTTPS nahi**: localhost par theek hai; internet par nahi.
//!
//! Web demo (client/) is phase me chheda nahi gaya — is API se UI jodne
//! ki wiring docs/REAL-NETWORK-PLAN.md me likhi hai.

use crate::block::Block;
use crate::butpay::ButPayRegistry;
use crate::chain::Chain;
use crate::error::{ButError, Result};
use crate::mempool::Mempool;
use crate::storage::{butpay_path_for, load_chain_with_registry, save_butpay, save_chain};
use crate::transaction::Transaction;
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

    /// Mempool se block mine karo: block banao, chain me jodo, disk par save karo.
    pub fn mine(&self) -> Result<Block> {
        let txs = {
            let mut pool = self.mempool.lock().unwrap();
            pool.take_for_block(10)
        };
        if txs.is_empty() {
            return Err(ButError::InvalidTransaction(
                "mempool khaali hai — mine karne ko koi transaction nahi hai".to_string(),
            ));
        }
        let block = {
            let chain = self.chain.lock().unwrap();
            Block::new(
                chain.height() as u64,
                now_secs(),
                chain.latest_hash(),
                txs,
                self.validator.address.clone(),
                &self.validator.signing_key(),
            )?
        };
        {
            let mut chain = self.chain.lock().unwrap();
            let reg = self.butpay.lock().unwrap();
            chain.add_block_with_registry(block.clone(), &reg)?;
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
                    "node": "but-node (Phase 2 local API)",
                    "height": height,
                    "latest_hash": latest_hash,
                    "endpoints": [
                        "GET /chain/height",
                        "GET /chain",
                        "GET /block/<index>",
                        "GET /balance/<address>",
                        "GET /mempool",
                        "GET /validators",
                        "GET /butpay/<id-ya-handle>",
                        "POST /butpay/register",
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
            match serde_json::to_value(pool.list()) {
                Ok(v) => (200, json!({"ok": true, "pending": v})),
                Err(e) => (500, json!({"ok": false, "error": format!("mempool JSON nahi bana: {e}")})),
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
                }),
            )
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
                    (
                        200,
                        json!({
                            "ok": true,
                            "address": address,
                            "balance": chain.balance(address),
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
        // Temp dir me chain file — test ke baad saaf hoti hai.
        let dir = std::env::temp_dir().join("but_api_test_state");
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
