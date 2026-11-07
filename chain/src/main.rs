//! but-node — BUT chain ka CLI (Phase 2).
//!
//! Commands:
//! - `but-node new-wallet` — naya real wallet banao (24-word mnemonic).
//! - `but-node demo` — ek chhoti chain banao: genesis, 2 transfers,
//!   2 blocks, verify, disk par save, wapas load karke verify.
//! - `but-node verify <chain.json>` — chain file load karke verify karo.
//! - `but-node serve [http-port] [chain-file]` — local HTTP/JSON API chalao
//!   (default port 3030, default file data/chain.json). Endpoints: /,
//!   /chain/height, /chain, /block/<n>, /balance/<addr>, /mempool,
//!   /validators, POST /tx, POST /mine.
//! - `but-node node [p2p-port] [http-port] [chain-file]` — poora node chalao:
//!   P2P listener + HTTP API, ek saath (default 9001 / 3030).
//! - `but-node p2p-demo` — do nodes (alag ports) banao aur sync karke dikhao.
//! - `but-node butpay-register <address> [handle] [chain-file]` — wallet ko
//!   BUT Pay ID do (100001 se shuru). Registry `data/butpay.json` me save hoti hai.
//! - `but-node butpay-lookup <id-ya-handle> [chain-file]` — ID/handle se address nikalo.
//!
//! WARNING: `new-wallet` mnemonic aur private key screen par dikhata hai.
//! Inhe kabhi kisi ko mat dikhao, kabhi screenshot me mat rakho.

use but_chain::api::{serve_forever as serve_api, ApiState};
use but_chain::butpay::ButPayRegistry;
use but_chain::p2p::Node;
use but_chain::storage::{
    butpay_path_for, load_butpay, load_chain, load_chain_with_registry, save_butpay, save_chain,
};
use but_chain::{Block, Chain, Mempool, Transaction, Wallet};
use std::net::TcpListener;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_700_000_000)
}

fn print_wallet(label: &str, w: &Wallet) {
    println!("--- {label} ---");
    println!("address     : {}", w.address);
    println!("private key : {} (128 chars — KABHI SHARE MAT KARO)", w.private_key_hex);
    println!("mnemonic    : {} (24 words — KABHI SHARE MAT KARO)", w.mnemonic);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    match cmd {
        "new-wallet" => {
            let w = Wallet::generate().expect("wallet nahi bana");
            print_wallet("NAYA REAL WALLET", &w);
            println!();
            println!("Mnemonic aur private key ko kagaz par likh kar safe jagah rakho.");
            println!("Ye demo nahi hai — ye asli BIP39 wallet hai.");
        }
        "demo" => {
            if let Err(e) = run_demo() {
                eprintln!("demo fail: {e}");
                std::process::exit(1);
            }
        }
        "verify" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or("data/chain.json");
            match load_chain(Path::new(path)) {
                Ok(chain) => {
                    println!("Chain OK: {} blocks, latest hash {}", chain.height(), chain.latest_hash());
                    println!("Verify pass — chain badli nahi gayi hai.");
                }
                Err(e) => {
                    eprintln!("Chain VERIFY FAIL: {e}");
                    std::process::exit(1);
                }
            }
        }
        "serve" => {
            let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3030);
            let path = args.get(3).map(|s| s.as_str()).unwrap_or("data/chain.json");
            if let Err(e) = run_serve(port, path) {
                eprintln!("serve fail: {e}");
                std::process::exit(1);
            }
        }
        "node" => {
            let p2p_port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(9001);
            let http_port: u16 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3030);
            let path = args.get(4).map(|s| s.as_str()).unwrap_or("data/chain.json");
            if let Err(e) = run_node(p2p_port, http_port, path) {
                eprintln!("node fail: {e}");
                std::process::exit(1);
            }
        }
        "p2p-demo" => {
            if let Err(e) = run_p2p_demo() {
                eprintln!("p2p-demo fail: {e}");
                std::process::exit(1);
            }
        }
        "butpay-register" => {
            let address = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let handle = args.get(3).map(|s| s.as_str());
            let chain_file = args.get(4).map(|s| s.as_str()).unwrap_or("data/chain.json");
            if let Err(e) = run_butpay_register(address, handle, chain_file) {
                eprintln!("butpay-register fail: {e}");
                std::process::exit(1);
            }
        }
        "butpay-lookup" => {
            let query = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let chain_file = args.get(3).map(|s| s.as_str()).unwrap_or("data/chain.json");
            if let Err(e) = run_butpay_lookup(query, chain_file) {
                eprintln!("butpay-lookup fail: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            println!("BUT node — real chain core (Phase 2)");
            println!("Usage:");
            println!("  but-node new-wallet        naya real wallet banao");
            println!("  but-node demo              chhoti chain banao + verify + save");
            println!("  but-node verify <file>     chain file verify karo");
            println!("  but-node serve [port] [chain-file]   local HTTP API (default 3030, data/chain.json)");
            println!("  but-node node [p2p-port] [http-port] [chain-file]   poora node: P2P + HTTP API");
            println!("  but-node p2p-demo          do nodes banao aur sync karke dikhao");
            println!("  but-node butpay-register <address> [handle] [chain-file]   BUT Pay ID do");
            println!("  but-node butpay-lookup <id-ya-handle> [chain-file]   ID/handle se address nikalo");
        }
    }
}

/// Local HTTP API chalao (Ctrl-C se band karo).
fn run_serve(port: u16, chain_file: &str) -> Result<(), String> {
    let state = ApiState::load_or_create(Path::new(chain_file)).map_err(|e| e.to_string())?;
    let (height, hash) = {
        let chain = state.chain.lock().unwrap();
        (chain.height(), chain.latest_hash())
    };
    println!("BUT node API chal rahi hai: http://127.0.0.1:{port}/");
    println!("Chain: {height} blocks, latest hash {hash}");
    println!("Endpoints: /chain/height, /balance/<address>, POST /tx, POST /mine");
    println!("Band karne ke liye Ctrl-C dabao.");
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("port {port} par sun nahi paya: {e}"))?;
    serve_api(listener, &state).map_err(|e| e.to_string())?;
    Ok(())
}

/// Poora node chalao: P2P listener (alag thread) + HTTP API (is thread).
fn run_node(p2p_port: u16, http_port: u16, chain_file: &str) -> Result<(), String> {
    let state = ApiState::load_or_create(Path::new(chain_file)).map_err(|e| e.to_string())?;
    // P2P node aur API wahi chain/mempool share karte hain.
    let node = Node {
        chain: state.chain.clone(),
        mempool: state.mempool.clone(),
        peers: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
    };
    let p2p_listener = TcpListener::bind(("127.0.0.1", p2p_port))
        .map_err(|e| format!("P2P port {p2p_port} par sun nahi paya: {e}"))?;
    let node_thread = node.clone();
    std::thread::spawn(move || {
        if let Err(e) = node_thread.serve_forever(p2p_listener) {
            eprintln!("P2P server band ho gaya: {e}");
        }
    });
    let (height, hash) = {
        let chain = state.chain.lock().unwrap();
        (chain.height(), chain.latest_hash())
    };
    println!("BUT node chal raha hai — chain: {height} blocks, latest hash {hash}");
    println!("P2P: 127.0.0.1:{p2p_port}   HTTP API: http://127.0.0.1:{http_port}/");
    println!("Band karne ke liye Ctrl-C dabao.");
    let http_listener = TcpListener::bind(("127.0.0.1", http_port))
        .map_err(|e| format!("HTTP port {http_port} par sun nahi paya: {e}"))?;
    serve_api(http_listener, &state).map_err(|e| e.to_string())?;
    Ok(())
}

/// Do nodes, alag-alag ports, sync karke dikhao.
fn run_p2p_demo() -> Result<(), String> {
    let validator = Wallet::from_entropy(&[9u8; 32]).map_err(|e| e.to_string())?;
    let alice = Wallet::from_entropy(&[1u8; 32]).map_err(|e| e.to_string())?;
    let bob = Wallet::from_entropy(&[2u8; 32]).map_err(|e| e.to_string())?;
    let t0 = now_secs();

    // Node A: genesis + 2 blocks (poori chain).
    let mut chain_a = Chain::new_genesis(
        vec![(alice.address.clone(), 1_000_000)],
        &validator,
        t0,
    )
    .map_err(|e| e.to_string())?;
    for (i, (amount, nonce)) in [(100u64, 0u64), (25, 1)].iter().enumerate() {
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), *amount, 0, *nonce);
        tx.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
        let block = Block::new(
            chain_a.height() as u64,
            t0 + 1 + i as u64,
            chain_a.latest_hash(),
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .map_err(|e| e.to_string())?;
        chain_a.add_block(block).map_err(|e| e.to_string())?;
    }
    // Node B: sirf genesis (wahi params, isliye genesis hash ek hai).
    let chain_b = Chain::new_genesis(
        vec![(alice.address.clone(), 1_000_000)],
        &validator,
        t0,
    )
    .map_err(|e| e.to_string())?;

    let node_a = Node::new(chain_a);
    let node_b = Node::new(chain_b);
    println!("Node A (port 9001): height {}", node_a.height_and_hash().0);
    println!("Node B (port 9002): height {} (sync se pehle)", node_b.height_and_hash().0);

    // Node A ko port 9001 par sunao (background thread, hamesha chalega).
    let listener = TcpListener::bind(("127.0.0.1", 9001))
        .map_err(|e| format!("port 9001 par sun nahi paya: {e}"))?;
    let node_a_server = node_a.clone();
    std::thread::spawn(move || {
        let _ = node_a_server.serve_forever(listener);
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Node B, Node A se sync kare.
    let (peer_height, _) = Node::request_height("127.0.0.1:9001").map_err(|e| e.to_string())?;
    println!("Node B ne Node A se height poochhi: {peer_height}");
    let added = node_b.sync_from_peer("127.0.0.1:9001").map_err(|e| e.to_string())?;
    println!("Node B ne {added} blocks sync kiye.");
    println!("Node B (sync ke baad): height {}", node_b.height_and_hash().0);
    let chain_b = node_b.chain.lock().unwrap();
    chain_b.verify_chain().map_err(|e| e.to_string())?;
    println!("Verify: PASS ({} blocks)", chain_b.height());
    println!(
        "Balances — Alice: {}, Bob: {}",
        chain_b.balance(&alice.address),
        chain_b.balance(&bob.address)
    );
    println!("Sync kaam kar gaya — Node B ki chain ab Node A jaisi hai.");
    Ok(())
}

/// BUT Pay ID register karo — wallet address ko chhota number do.
/// Registry chain file ke saath `butpay.json` me save hoti hai.
fn run_butpay_register(address: &str, handle: Option<&str>, chain_file: &str) -> Result<(), String> {
    let reg_path = butpay_path_for(Path::new(chain_file));
    let mut reg = load_butpay(&reg_path).map_err(|e| e.to_string())?;
    let id = reg.register(address, handle).map_err(|e| e.to_string())?;
    save_butpay(&reg_path, &reg).map_err(|e| e.to_string())?;
    println!("BUT Pay ID mil gayi: {id}");
    println!("address : {address}");
    if let Some(h) = handle {
        println!("handle  : @{h}");
    }
    println!("Registry save hui: {}", reg_path.display());
    println!("Ab is ID se receive kar sakte ho — jaise: but-node butpay-lookup {id}");
    Ok(())
}

/// BUT Pay ID/handle se address nikalo.
fn run_butpay_lookup(query: &str, chain_file: &str) -> Result<(), String> {
    let reg_path = butpay_path_for(Path::new(chain_file));
    let reg = load_butpay(&reg_path).map_err(|e| e.to_string())?;
    match reg.lookup(query) {
        Some(entry) => {
            println!("BUT Pay ID : {}", entry.id);
            if let Some(h) = &entry.handle {
                println!("handle    : @{h}");
            }
            println!("address   : {}", entry.address);
            Ok(())
        }
        None => Err(format!(
            "BUT Pay ID/handle '{query}' nahi mila — pehle register karo: but-node butpay-register <address> [handle]"
        )),
    }
}

fn run_demo() -> Result<(), String> {
    // Wallets (demo ke liye fixed entropy — real use me Wallet::generate() use karo).
    let validator = Wallet::from_entropy(&[9u8; 32]).map_err(|e| e.to_string())?;
    let alice = Wallet::from_entropy(&[1u8; 32]).map_err(|e| e.to_string())?;
    let bob = Wallet::from_entropy(&[2u8; 32]).map_err(|e| e.to_string())?;
    println!("Validator: {}", validator.address);
    println!("Alice    : {}", alice.address);
    println!("Bob      : {}", bob.address);

    // Genesis: Alice ko 1,000,000 smallest units.
    let t0 = now_secs();
    let mut chain = Chain::new_genesis(
        vec![(alice.address.clone(), 1_000_000)],
        &validator,
        t0,
    )
    .map_err(|e| e.to_string())?;
    println!("\nGenesis block: {}", chain.latest_hash());

    // Alice -> Bob 100 (fee 1), mempool se block 1.
    let mut pool = Mempool::new();
    let mut tx1 = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 1, 0);
    tx1.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    pool.add(tx1).map_err(|e| e.to_string())?;
    let block1 = Block::new(
        chain.height() as u64,
        t0 + 1,
        chain.latest_hash(),
        pool.take_for_block(10),
        validator.address.clone(),
        &validator.signing_key(),
    )
    .map_err(|e| e.to_string())?;
    chain.add_block(block1).map_err(|e| e.to_string())?;
    println!("Block 1: {} (Alice -> Bob 100, fee 1)", chain.latest_hash());

    // Bob -> Alice 25 (fee 0), block 2.
    let mut tx2 = Transaction::new(bob.address.clone(), alice.address.clone(), 25, 0, 0);
    tx2.sign(&bob.signing_key()).map_err(|e| e.to_string())?;
    let block2 = Block::new(
        chain.height() as u64,
        t0 + 2,
        chain.latest_hash(),
        vec![tx2],
        validator.address.clone(),
        &validator.signing_key(),
    )
    .map_err(|e| e.to_string())?;
    chain.add_block(block2).map_err(|e| e.to_string())?;
    println!("Block 2: {} (Bob -> Alice 25)", chain.latest_hash());

    // Verify + balances.
    chain.verify_chain().map_err(|e| e.to_string())?;
    println!("\nVerify: PASS ({} blocks)", chain.height());
    println!("Balances — Alice: {}, Bob: {}, Validator (fees): {}",
        chain.balance(&alice.address),
        chain.balance(&bob.address),
        chain.balance(&validator.address));

    // BUT Pay: Bob ko chhoti ID do, Alice ID se 10 bhejti hai.
    let mut reg = ButPayRegistry::new();
    let bob_id = reg.register(&bob.address, Some("bob")).map_err(|e| e.to_string())?;
    println!("\nBUT Pay: Bob ki ID {bob_id}, handle @bob");
    let mut tx3 = Transaction::new(alice.address.clone(), bob_id.to_string(), 10, 0, 1);
    tx3.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    let mut pool3 = Mempool::new();
    pool3.add_with_registry(tx3, &reg).map_err(|e| e.to_string())?;
    let block3 = Block::new(
        chain.height() as u64,
        t0 + 3,
        chain.latest_hash(),
        pool3.take_for_block(10),
        validator.address.clone(),
        &validator.signing_key(),
    )
    .map_err(|e| e.to_string())?;
    chain.add_block_with_registry(block3, &reg).map_err(|e| e.to_string())?;
    chain.verify_chain_with_registry(&reg).map_err(|e| e.to_string())?;
    println!("Block 3: {} (Alice -> BUT Pay ID {bob_id} = Bob, 10)", chain.latest_hash());
    println!("Balances — Alice: {}, Bob: {} (ID se bheja paisa Bob ke address par pahuncha)",
        chain.balance(&alice.address),
        chain.balance(&bob.address));

    // Disk par save karo (chain + registry), wapas load karke verify karo.
    let path = Path::new("data/chain.json");
    save_chain(&chain, path).map_err(|e| e.to_string())?;
    let reg_path = butpay_path_for(path);
    save_butpay(&reg_path, &reg).map_err(|e| e.to_string())?;
    println!("\nChain disk par save hui: {}", path.display());
    println!("BUT Pay registry save hui: {}", reg_path.display());
    let (loaded, loaded_reg) = load_chain_with_registry(path).map_err(|e| e.to_string())?;
    println!("Reload + verify: PASS ({} blocks)", loaded.height());
    println!("Reloaded registry: Bob ki ID {} → {}", bob_id, loaded_reg.resolve(&bob_id.to_string()).ok_or("reload ke baad registry me Bob ki ID nahi mili")?);
    Ok(())
}
