//! but-node — BUT chain ka CLI (Phase 3b).
//!
//! Commands:
//! - `but-node new-wallet` — naya real wallet banao (24-word mnemonic).
//! - `but-node demo` — ek chhoti chain banao: genesis, transfers, BUT Pay,
//!   stake + validator register, swap (liquidity + AMM) + slash demo,
//!   verify, disk par save, wapas load karke verify.
//! - `but-node verify <chain.json>` — chain file load karke verify karo.
//! - `but-node serve [http-port] [chain-file]` — local HTTP/JSON API chalao
//!   (default port 3030, default file data/chain.json). Endpoints: /,
//!   /chain/height, /chain, /block/<n>, /balance/<addr>, /mempool,
//!   /validators, /staking, /stake/<addr>, POST /tx, POST /stake,
//!   POST /validators/register, POST /mine, GET /swap/quote, GET /swap/pools,
//!   POST /swap, POST /slash.
//! - `but-node node [p2p-port] [http-port] [chain-file]` — poora node chalao:
//!   P2P listener + HTTP API, ek saath (default 9001 / 3030).
//! - `but-node p2p-demo` — do nodes (alag ports) banao aur sync karke dikhao
//!   (registry + ID-tx block bhi sync hote hain).
//! - `but-node convert <bits>` — bits ko BUT me dikhao (1 BUT = 1,000 bits).
//!   Decimal (jaise `1.5`) do to BUT ko bits me badlo.
//! - `but-node butpay-register <address> [handle] [chain-file]` — wallet ko
//!   BUT Pay ID do (100001 se shuru). Registry `data/butpay.json` me save hoti hai.
//! - `but-node butpay-lookup <id-ya-handle> [chain-file]` — ID/handle se address nikalo.
//! - `but-node swap-quote <from> <to> <amount> [chain-file]` — swap se PEHLE
//!   quote dekho: kitna output milega, pool me kitna available hai (§11).
//!   Jaise: `but-node swap-quote BUT BUTCOIN 50000`.
//! - `but-node swap <from> <to> <amount> <private-key-hex> [chain-file]` —
//!   swap karo: signed SwapOp banao aur dev validator se seedha block me
//!   mine karo. Dev validator us height ka proposer hona chahiye, warna
//!   saaf error milega.
//!
//! WARNING: `new-wallet` mnemonic aur private key screen par dikhata hai.
//! Inhe kabhi kisi ko mat dikhao, kabhi screenshot me mat rakho.

use but_chain::api::{serve_forever as serve_api, ApiState};
use but_chain::butpay::ButPayRegistry;
use but_chain::consensus::{Vote, VoteTally};
use but_chain::p2p::Node;
use but_chain::slash::SlashOp;
use but_chain::staking::{LockTier, StakeOp};
use but_chain::storage::{
    butpay_path_for, load_butpay, load_chain, load_chain_with_registry, save_butpay, save_chain,
};
use but_chain::swap::{format_butcoin, SwapAsset, SwapOp};
use but_chain::units::{format_but, format_but_with_unit, parse_but};
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
        "convert" => {
            let arg = args.get(2).map(|s| s.as_str()).unwrap_or("");
            if let Ok(bits) = arg.parse::<u64>() {
                println!("{bits} bits = {} BUT", format_but(bits));
            } else if let Ok(bits) = parse_but(arg) {
                println!("{arg} BUT = {bits} bits");
            } else {
                eprintln!("convert samajh nahi aaya: '{arg}' — bits (jaise 1500) ya BUT (jaise 1.5) do");
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
        "swap-quote" => {
            let from = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let to = args.get(3).map(|s| s.as_str()).unwrap_or("");
            let amount = args.get(4).map(|s| s.as_str()).unwrap_or("");
            let chain_file = args.get(5).map(|s| s.as_str()).unwrap_or("data/chain.json");
            if let Err(e) = run_swap_quote(from, to, amount, chain_file) {
                eprintln!("swap-quote fail: {e}");
                std::process::exit(1);
            }
        }
        "swap" => {
            let from = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let to = args.get(3).map(|s| s.as_str()).unwrap_or("");
            let amount = args.get(4).map(|s| s.as_str()).unwrap_or("");
            let private_key = args.get(5).map(|s| s.as_str()).unwrap_or("");
            let chain_file = args.get(6).map(|s| s.as_str()).unwrap_or("data/chain.json");
            if let Err(e) = run_swap(from, to, amount, private_key, chain_file) {
                eprintln!("swap fail: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            println!("BUT node — real chain core (Phase 3b)");
            println!("Usage:");
            println!("  but-node new-wallet        naya real wallet banao");
            println!("  but-node demo              chhoti chain banao + verify + save");
            println!("  but-node verify <file>     chain file verify karo");
            println!("  but-node serve [port] [chain-file]   local HTTP API (default 3030, data/chain.json)");
            println!("  but-node node [p2p-port] [http-port] [chain-file]   poora node: P2P + HTTP API");
            println!("  but-node p2p-demo          do nodes banao aur sync karke dikhao");
            println!("  but-node convert <bits|but>   bits <-> BUT badlo (1 BUT = 1,000 bits)");
            println!("  but-node butpay-register <address> [handle] [chain-file]   BUT Pay ID do");
            println!("  but-node butpay-lookup <id-ya-handle> [chain-file]   ID/handle se address nikalo");
            println!("  but-node swap-quote <from> <to> <amount> [chain-file]   swap se pehle quote dekho");
            println!("  but-node swap <from> <to> <amount> <private-key-hex> [chain-file]   swap karo + mine karo");
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
    println!("Endpoints: /chain/height, /balance/<address>, /staking, POST /tx, POST /stake, POST /mine");
    println!("Band karne ke liye Ctrl-C dabao.");
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("port {port} par sun nahi paya: {e}"))?;
    serve_api(listener, &state).map_err(|e| e.to_string())?;
    Ok(())
}

/// Poora node chalao: P2P listener (alag thread) + HTTP API (is thread).
fn run_node(p2p_port: u16, http_port: u16, chain_file: &str) -> Result<(), String> {
    let state = ApiState::load_or_create(Path::new(chain_file)).map_err(|e| e.to_string())?;
    // P2P node aur API wahi chain/mempool/registry share karte hain.
    let node = Node {
        chain: state.chain.clone(),
        mempool: state.mempool.clone(),
        peers: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        tallies: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        registry: state.butpay.clone(),
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

/// Block banao (sahi proposer se) aur uske self-vote ke saath BFT se
/// chain me jodo. Demo/CLI ke liye hai.
/// Proposer `proposer_for_round` se chuna jata hai — `validators` slice me
/// saare validator wallets hone chahiye. Demo me Alice register (block 5)
/// ke baad proposer hamesha Alice hoti hai (uske paas 100% stake hai),
/// isliye ek fixed validator se block banana galat hoga.
fn bft_add(
    chain: &mut Chain,
    validators: &[Wallet],
    txs: Vec<Transaction>,
    stake_ops: Vec<StakeOp>,
    timestamp: u64,
    reg: &ButPayRegistry,
) -> Result<(), String> {
    bft_add_ops(
        chain,
        validators,
        txs,
        stake_ops,
        Vec::new(),
        Vec::new(),
        timestamp,
        reg,
    )
}

/// BFT block banao — txs + stake ops + swap ops + slash ops ke saath.
/// Proposer hamesha `proposer_for_round` se chuna jata hai (stake-weighted).
fn bft_add_ops(
    chain: &mut Chain,
    validators: &[Wallet],
    txs: Vec<Transaction>,
    stake_ops: Vec<StakeOp>,
    swap_ops: Vec<SwapOp>,
    slash_ops: Vec<SlashOp>,
    timestamp: u64,
    reg: &ButPayRegistry,
) -> Result<(), String> {
    let height = chain.height() as u64;
    let proposer_addr = chain
        .validators
        .proposer_for_round(height)
        .ok_or_else(|| "koi active validator nahi hai — proposer nahi chuna ja sakta".to_string())?;
    let proposer = validators
        .iter()
        .find(|w| w.address == proposer_addr)
        .ok_or_else(|| format!("proposer {proposer_addr} ka wallet demo/CLI me nahi hai — uska wallet chahiye"))?;
    let block = Block::new_with_ops(
        height,
        timestamp,
        chain.latest_hash(),
        txs,
        stake_ops,
        swap_ops,
        slash_ops,
        proposer.address.clone(),
        &proposer.signing_key(),
    )
    .map_err(|e| e.to_string())?;
    let mut tally = VoteTally::new(height);
    let vote = Vote::new(
        block.hash.clone(),
        height,
        true,
        proposer.address.clone(),
        &proposer.signing_key(),
    )
    .map_err(|e| e.to_string())?;
    tally
        .add_vote(vote, &chain.validators)
        .map_err(|e| e.to_string())?;
    chain
        .add_block_bft_with_registry(block, &tally, reg)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Do nodes, alag-alag ports, sync karke dikhao — registry + ID-tx block bhi.
fn run_p2p_demo() -> Result<(), String> {
    let validator = Wallet::from_entropy(&[9u8; 32]).map_err(|e| e.to_string())?;
    let alice = Wallet::from_entropy(&[1u8; 32]).map_err(|e| e.to_string())?;
    let bob = Wallet::from_entropy(&[2u8; 32]).map_err(|e| e.to_string())?;
    let t0 = now_secs();
    let mut reg = ButPayRegistry::new();
    // Node A par Bob ki BUT Pay ID register karo (100001, @bob).
    let bob_id = reg.register(&bob.address, Some("bob")).map_err(|e| e.to_string())?;
    println!("Node A: Bob ki BUT Pay ID {bob_id}, handle @bob");

    // Node A: genesis + 3 blocks (poori chain).
    let mut chain_a = Chain::new_genesis(
        vec![(alice.address.clone(), 1_000_000)],
        &validator,
        t0,
    )
    .map_err(|e| e.to_string())?;
    let validators = [validator.clone()];
    for (i, (amount, nonce)) in [(100u64, 0u64), (25, 1)].iter().enumerate() {
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), *amount, 0, *nonce);
        tx.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
        bft_add(&mut chain_a, &validators, vec![tx], Vec::new(), t0 + 1 + i as u64, &reg)?;
    }
    // Block 3: Alice → BUT Pay ID 100001 (Bob) 10 — ID-tx block.
    let mut tx_id = Transaction::new(alice.address.clone(), bob_id.to_string(), 10, 0, 2);
    tx_id.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    bft_add(&mut chain_a, &validators, vec![tx_id], Vec::new(), t0 + 3, &reg)?;
    println!("Node A: block 3 me ID-tx hai (Alice -> ID {bob_id} = Bob, 10)");
    // Node B: sirf genesis (wahi params, isliye genesis hash ek hai).
    let chain_b = Chain::new_genesis(
        vec![(alice.address.clone(), 1_000_000)],
        &validator,
        t0,
    )
    .map_err(|e| e.to_string())?;

    let node_a = Node::with_registry(chain_a, reg);
    let node_b = Node::new(chain_b);
    println!("Node A (port 9001): height {}", node_a.height_and_hash().0);
    println!("Node B (port 9002): height {} (sync se pehle), registry khaali: {}",
        node_b.height_and_hash().0,
        node_b.registry.lock().unwrap().is_empty());

    // Node A ko port 9001 par sunao (background thread, hamesha chalega).
    let listener = TcpListener::bind(("127.0.0.1", 9001))
        .map_err(|e| format!("port 9001 par sun nahi paya: {e}"))?;
    let node_a_server = node_a.clone();
    std::thread::spawn(move || {
        let _ = node_a_server.serve_forever(listener);
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Node B, Node A se sync kare — pehle registry, phir blocks.
    let (peer_height, _) = Node::request_height("127.0.0.1:9001").map_err(|e| e.to_string())?;
    println!("Node B ne Node A se height poochhi: {peer_height}");
    let added = node_b.sync_from_peer("127.0.0.1:9001").map_err(|e| e.to_string())?;
    println!("Node B ne {added} blocks sync kiye.");
    println!("Node B (sync ke baad): height {}", node_b.height_and_hash().0);
    // Registry bhi sync hui — Bob ki ID Node B par bhi milti hai.
    let reg_b = node_b.registry.lock().unwrap().clone();
    let bob_via_id = reg_b.resolve(&bob_id.to_string()).ok_or("sync ke baad Node B ki registry me Bob ki ID nahi mili")?;
    println!("Node B registry: ID {bob_id} → {bob_via_id} (handle @bob bhi: {})",
        reg_b.resolve("@bob").ok_or("sync ke baad @bob handle nahi mila")?);
    let chain_b = node_b.chain.lock().unwrap();
    chain_b.verify_chain_with_registry(&reg_b).map_err(|e| e.to_string())?;
    println!("Verify: PASS ({} blocks, registry ke saath)", chain_b.height());
    println!(
        "Balances — Alice: {}, Bob: {} (ID-tx wale 10 bhi mile)",
        chain_b.balance(&alice.address),
        chain_b.balance(&bob.address)
    );
    println!("Sync kaam kar gaya — Node B ki chain aur registry ab Node A jaisi hain.");
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

/// Swap se PEHLE quote dekho (§11) — kitna output milega, pool me kitna
/// available hai. Koi transaction nahi banti, sirf jaankari milti hai.
fn run_swap_quote(from: &str, to: &str, amount: &str, chain_file: &str) -> Result<(), String> {
    let (chain, _) = load_chain_with_registry(Path::new(chain_file)).map_err(|e| e.to_string())?;
    let from_asset = SwapAsset::parse(from).map_err(|e| e.to_string())?;
    let to_asset = SwapAsset::parse(to).map_err(|e| e.to_string())?;
    let amount_in: u64 = amount.parse().map_err(|_| {
        format!("amount number hona chahiye (smallest units me), mila: '{amount}'")
    })?;
    let quote = chain
        .swap
        .quote(&from_asset, &to_asset, amount_in)
        .map_err(|e| e.to_string())?;
    let fmt = |asset: &SwapAsset, n: u64| match asset {
        SwapAsset::But => format_but_with_unit(n),
        SwapAsset::ButCoin => format!("{} BUT Coin", format_butcoin(n)),
        SwapAsset::External(name) => format!("{n} {name}"),
    };
    println!("Swap quote (swap karne se PEHLE — §11):");
    println!("  {} → {}", fmt(&from_asset, amount_in), to_asset.label());
    println!("  Milega (lagbhag): {}", fmt(&to_asset, quote.expected_out));
    println!("  Pool me available: {}", fmt(&to_asset, quote.available_out));
    println!("  Pool reserves — input: {}, output: {}",
        fmt(&from_asset, quote.reserve_in), fmt(&to_asset, quote.reserve_out));
    println!("  Note: {}", quote.note);
    Ok(())
}

/// Swap karo: signed SwapOp banao aur dev validator se seedha block me mine karo.
/// Dev validator us height ka proposer hona chahiye — warna saaf error milega
/// (tab API `POST /swap` se mempool me bhejo, proposer mine karega).
/// WARNING: private key command line par likhne se shell history me save ho
/// sakti hai — ye command sirf apni machine par, dev/testing ke liye hai.
fn run_swap(
    from: &str,
    to: &str,
    amount: &str,
    private_key_hex: &str,
    chain_file: &str,
) -> Result<(), String> {
    let (mut chain, reg) =
        load_chain_with_registry(Path::new(chain_file)).map_err(|e| e.to_string())?;
    let from_asset = SwapAsset::parse(from).map_err(|e| e.to_string())?;
    let to_asset = SwapAsset::parse(to).map_err(|e| e.to_string())?;
    let amount_in: u64 = amount.parse().map_err(|_| {
        format!("amount number hona chahiye (smallest units me), mila: '{amount}'")
    })?;
    let wallet = Wallet::from_private_key_hex(private_key_hex).map_err(|e| e.to_string())?;
    // Pehle quote dikhao — user ko pata chale kitna milega (§11).
    let quote = chain
        .swap
        .quote(&from_asset, &to_asset, amount_in)
        .map_err(|e| e.to_string())?;
    println!("Quote: {} {} → lagbhag {} {} milega (pool me {} available)",
        amount_in, from_asset.label(), quote.expected_out, to_asset.label(), quote.available_out);
    println!("Note: {}", quote.note);
    // SwapOp banao aur sign karo (nonce chain se).
    let nonce = chain.next_nonce(&wallet.address);
    let mut op = SwapOp::swap(from_asset.clone(), to_asset.clone(), amount_in, nonce, wallet.address.clone());
    op.sign(&wallet.signing_key()).map_err(|e| e.to_string())?;
    // Direct mine: dev validator hi proposer hona chahiye.
    let dev = but_chain::api::dev_validator();
    let height = chain.height() as u64;
    match chain.validators.proposer_for_round(height) {
        Some(p) if p == dev.address => {}
        Some(p) => {
            return Err(format!(
                "is height ({height}) ka proposer {p} hai, dev validator nahi — seedha mine nahi ho sakta. Swap op ko API se mempool me bhejo (POST /swap), proposer mine karega."
            ));
        }
        None => {
            return Err("koi active validator nahi hai — proposer nahi chuna ja sakta".to_string());
        }
    }
    let block = Block::new_with_ops(
        height,
        now_secs(),
        chain.latest_hash(),
        Vec::new(),
        Vec::new(),
        vec![op],
        Vec::new(),
        dev.address.clone(),
        &dev.signing_key(),
    )
    .map_err(|e| e.to_string())?;
    let mut tally = VoteTally::new(height);
    let vote = Vote::new(
        block.hash.clone(),
        height,
        true,
        dev.address.clone(),
        &dev.signing_key(),
    )
    .map_err(|e| e.to_string())?;
    tally
        .add_vote(vote, &chain.validators)
        .map_err(|e| e.to_string())?;
    chain
        .add_block_bft_with_registry(block, &tally, &reg)
        .map_err(|e| e.to_string())?;
    let path = Path::new(chain_file);
    save_chain(&chain, path).map_err(|e| e.to_string())?;
    save_butpay(&butpay_path_for(path), &reg).map_err(|e| e.to_string())?;
    println!("Swap ho gaya — block {} me jud gaya (hash {})", height, chain.latest_hash());
    println!("Tumhara balance ab — BUT: {} bits ({}), BUT Coin: {} smallest units ({} BUT Coin)",
        chain.balance(&wallet.address),
        format_but_with_unit(chain.balance(&wallet.address)),
        chain.butcoin_balance(&wallet.address),
        format_butcoin(chain.butcoin_balance(&wallet.address)));
    println!("Pools ab — BUT: {} bits, BUT Coin: {} smallest units",
        chain.swap.but_pool, chain.swap.butcoin_pool);
    Ok(())
}

fn run_demo() -> Result<(), String> {
    // Wallets (demo ke liye fixed entropy — real use me Wallet::generate() use karo).
    let validator = Wallet::from_entropy(&[9u8; 32]).map_err(|e| e.to_string())?;
    let alice = Wallet::from_entropy(&[1u8; 32]).map_err(|e| e.to_string())?;
    let bob = Wallet::from_entropy(&[2u8; 32]).map_err(|e| e.to_string())?;
    println!("Validator: {}", validator.address);
    println!("Alice    : {}", alice.address);
    println!("Bob      : {}", bob.address);

    // Genesis: Alice ko 1,000,000 bits (= 1,000 BUT) + 1,000,000 BUT Coin
    // smallest units (= 1,000 BUT Coin). Pools genesis par khaali hote hain.
    let t0 = now_secs();
    let mut chain = Chain::new_genesis_full(
        vec![(alice.address.clone(), 1_000_000)],
        vec![(alice.address.clone(), 1_000_000)],
        &validator,
        t0,
        None,
    )
    .map_err(|e| e.to_string())?;
    println!("\nGenesis block: {}", chain.latest_hash());
    println!("Genesis: Alice ko 1,000,000 bits = {} BUT + 1,000,000 BUT Coin smallest units = {} BUT Coin",
        format_but(1_000_000), format_butcoin(1_000_000));
    // Block 5 ke baad proposer hamesha Alice hoti hai (100% stake), isliye
    // bft_add ko dono wallets do — wo khud sahi proposer chunega.
    let validators = [validator.clone(), alice.clone()];

    // Alice -> Bob 100 (fee 1), mempool se block 1.
    let mut pool = Mempool::new();
    let mut tx1 = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 1, 0);
    tx1.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    pool.add(tx1).map_err(|e| e.to_string())?;
    let txs1 = pool.take_for_block(10);
    bft_add(&mut chain, &validators, txs1, Vec::new(), t0 + 1, &ButPayRegistry::new())?;
    println!("Block 1: {} (Alice -> Bob 100, fee 1)", chain.latest_hash());

    // Bob -> Alice 25 (fee 0), block 2.
    let mut tx2 = Transaction::new(bob.address.clone(), alice.address.clone(), 25, 0, 0);
    tx2.sign(&bob.signing_key()).map_err(|e| e.to_string())?;
    bft_add(&mut chain, &validators, vec![tx2], Vec::new(), t0 + 2, &ButPayRegistry::new())?;
    println!("Block 2: {} (Bob -> Alice 25)", chain.latest_hash());

    // BUT Pay: Bob ko chhoti ID do, Alice ID se 10 bhejti hai (block 3).
    let mut reg = ButPayRegistry::new();
    let bob_id = reg.register(&bob.address, Some("bob")).map_err(|e| e.to_string())?;
    println!("\nBUT Pay: Bob ki ID {bob_id}, handle @bob");
    let mut tx3 = Transaction::new(alice.address.clone(), bob_id.to_string(), 10, 0, 1);
    tx3.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    let mut pool3 = Mempool::new();
    pool3.add_with_registry(tx3, &reg).map_err(|e| e.to_string())?;
    let txs3 = pool3.take_for_block(10);
    bft_add(&mut chain, &validators, txs3, Vec::new(), t0 + 3, &reg)?;
    println!("Block 3: {} (Alice -> BUT Pay ID {bob_id} = Bob, 10)", chain.latest_hash());

    // Stake: Alice 100,000 bits stake karti hai (tier 90 din), nonce 2 (block 4).
    let mut stake_op = StakeOp::stake(alice.address.clone(), 100_000, LockTier::Days90, 2);
    stake_op.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    bft_add(&mut chain, &validators, Vec::new(), vec![stake_op], t0 + 4, &reg)?;
    println!("Block 4: {} (Alice ne 100,000 bits stake kiye — 90 din lock)", chain.latest_hash());
    println!("  Alice spendable: {} bits ({}), locked: {} bits ({})",
        chain.balance(&alice.address),
        format_but_with_unit(chain.balance(&alice.address)),
        chain.locked_balance(&alice.address),
        format_but_with_unit(chain.locked_balance(&alice.address)));

    // Register: Alice validator banti hai (block 5, nonce 3).
    let mut reg_op = StakeOp::register(alice.address.clone(), 3);
    reg_op.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    bft_add(&mut chain, &validators, Vec::new(), vec![reg_op], t0 + 5, &reg)?;
    println!("Block 5: {} (Alice validator ban gayi — locked stake {} bits)",
        chain.latest_hash(), chain.validators.stake_of(&alice.address));
    println!("  (Ab se proposer hamesha Alice hai — uske paas 100% stake hai.)");

    // Swap: Alice BUT liquidity 100,000 dalti hai (nonce 4, block 6).
    let mut liq_but = SwapOp::add_liquidity(SwapAsset::But, 100_000, 4, alice.address.clone());
    liq_but.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    bft_add_ops(&mut chain, &validators, Vec::new(), Vec::new(), vec![liq_but], Vec::new(), t0 + 6, &reg)?;
    println!("Block 6: {} (Alice ne BUT pool me 100,000 bits liquidity dali)", chain.latest_hash());
    // Swap: Alice BUT Coin liquidity 500,000 dalti hai (nonce 5, block 7).
    let mut liq_coin = SwapOp::add_liquidity(SwapAsset::ButCoin, 500_000, 5, alice.address.clone());
    liq_coin.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    bft_add_ops(&mut chain, &validators, Vec::new(), Vec::new(), vec![liq_coin], Vec::new(), t0 + 7, &reg)?;
    println!("Block 7: {} (Alice ne BUT Coin pool me 500,000 smallest units liquidity dali)", chain.latest_hash());
    println!("  Pools — BUT: {} bits ({}), BUT Coin: {} smallest units ({} BUT Coin)",
        chain.swap.but_pool, format_but_with_unit(chain.swap.but_pool),
        chain.swap.butcoin_pool, format_butcoin(chain.swap.butcoin_pool));

    // Quote: swap se PEHLE user ko bataya jata hai kitna milega (§11).
    let quote = chain
        .swap
        .quote(&SwapAsset::But, &SwapAsset::ButCoin, 50_000)
        .map_err(|e| e.to_string())?;
    println!("  Quote (swap se pehle): 50,000 bits BUT → {} BUT Coin smallest units milega", quote.expected_out);
    println!("  Quote note: {}", quote.note);

    // Swap: Alice 50,000 BUT → BUT Coin (nonce 6, block 8). Proposer Alice.
    // (Bob ke paas sirf 85 BUT hain — bada swap wo nahi kar sakta.)
    let mut sw = SwapOp::swap(SwapAsset::But, SwapAsset::ButCoin, 50_000, 6, alice.address.clone());
    sw.sign(&alice.signing_key()).map_err(|e| e.to_string())?;
    bft_add_ops(&mut chain, &validators, Vec::new(), Vec::new(), vec![sw], Vec::new(), t0 + 8, &reg)?;
    println!("Block 8: {} (Alice ne 50,000 bits BUT swap kiye → use {} BUT Coin smallest units mile)",
        chain.latest_hash(), chain.butcoin_balance(&alice.address));
    println!("  Pools ab — BUT: {} bits, BUT Coin: {} smallest units",
        chain.swap.but_pool, chain.swap.butcoin_pool);

    // Slash demo: Alice height 8 par do alag blocks ko vote deti hai
    // (ek asli block 8 ka hash, ek nakli hash) — equivocation.
    // Bob report karta hai (nonce 1 — uski pehli tx nonce 0 thi) —
    // SlashOp block 9 me jata hai.
    let block8_hash = chain.blocks[8].hash.clone();
    let v1 = Vote::new(block8_hash, 8, true, alice.address.clone(), &alice.signing_key())
        .map_err(|e| e.to_string())?;
    let v2 = Vote::new("b".repeat(64), 8, true, alice.address.clone(), &alice.signing_key())
        .map_err(|e| e.to_string())?;
    let mut slash = SlashOp::new(
        alice.address.clone(),
        v1,
        v2,
        "demo: Alice ne height 8 par do alag blocks ko vote diya".to_string(),
        bob.address.clone(),
        1,
    );
    slash.sign(&bob.signing_key()).map_err(|e| e.to_string())?;
    bft_add_ops(&mut chain, &validators, Vec::new(), Vec::new(), Vec::new(), vec![slash], t0 + 9, &reg)?;
    println!("Block 9: {} (Bob ne Alice ki equivocation report ki — Alice slash hui)", chain.latest_hash());
    println!("  Alice ka stake ab: {} bits (10% kata), jail me: {}",
        chain.validators.stake_of(&alice.address),
        chain.validators.is_jailed(&alice.address));

    // Verify + balances + staking/reward summary.
    chain.verify_chain_with_registry(&reg).map_err(|e| e.to_string())?;
    println!("\nVerify: PASS ({} blocks)", chain.height());
    println!("Balances — Alice: {} bits ({}), Bob: {} bits ({}), Validator: {} bits ({})",
        chain.balance(&alice.address),
        format_but_with_unit(chain.balance(&alice.address)),
        chain.balance(&bob.address),
        format_but_with_unit(chain.balance(&bob.address)),
        chain.balance(&validator.address),
        format_but_with_unit(chain.balance(&validator.address)));
    println!("BUT Coin — Alice: {} smallest units ({} BUT Coin), Bob: {} smallest units ({} BUT Coin)",
        chain.butcoin_balance(&alice.address),
        format_butcoin(chain.butcoin_balance(&alice.address)),
        chain.butcoin_balance(&bob.address),
        format_butcoin(chain.butcoin_balance(&bob.address)));
    println!("Swap pools — BUT: {} bits ({}), BUT Coin: {} smallest units ({} BUT Coin)",
        chain.swap.but_pool,
        format_but_with_unit(chain.swap.but_pool),
        chain.swap.butcoin_pool,
        format_butcoin(chain.swap.butcoin_pool));
    println!("Staking — released supply: {} bits ({}), min stake: {} bits ({}), next block reward: {} bits ({})",
        chain.released_supply(),
        format_but_with_unit(chain.released_supply()),
        chain.min_stake(),
        format_but_with_unit(chain.min_stake()),
        chain.next_block_reward(),
        format_but_with_unit(chain.next_block_reward()));
    println!("Team share accrued (abhi tak): {} bits ({}), team account: {:?}",
        chain.team_accrued,
        format_but_with_unit(chain.team_accrued),
        chain.team_account);

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
