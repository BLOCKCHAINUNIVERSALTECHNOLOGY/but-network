//! Dev tool: ek signed transaction ka JSON print karo.
//!
//! Usage: `cargo run --example make_tx -- <amount> <fee> <nonce>`
//! (default: 100, 0, 0). Sender hamesha Alice hai (entropy [1u8; 32]),
//! receiver Bob ([2u8; 32]) — wahi wallets jo `but-node demo` me hain.
//! Output ko `curl -X POST .../tx -d @tx.json` se node ko bheja ja sakta hai.
//!
//! Ye sirf local testing ke liye hai — real wallet me sender apni
//! private key se khud sign karega, aise fixed entropy se nahi.

use but_chain::{Transaction, Wallet};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let amount: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let fee: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let nonce: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
    let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
    let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), amount, fee, nonce);
    tx.sign(&alice.signing_key()).unwrap();
    println!("{}", serde_json::to_string_pretty(&tx).unwrap());
}
