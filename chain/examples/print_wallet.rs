//! Dev tool: mnemonic ya entropy se wallet ka address print karo.
//!
//! Usage:
//!   `cargo run --example print_wallet -- "<24 words>"`
//!   `cargo run --example print_wallet` — bina args ke Alice ([1u8; 32])
//!   aur Bob ([2u8; 32]) dikhata hai (wahi wallets jo `but-node demo` me hain).
//!
//! Ye sirf local testing / cross-check ke liye hai — TypeScript wallet ka
//! derivation (BIP39 seed -> Ed25519) Rust se match karta hai ya nahi,
//! ye isse verify hota hai. Mnemonic yahan terminal me dikhega, isliye
//! is tool ko kabhi asli seed phrase ke saath mat chalao.

use but_chain::Wallet;

fn print_wallet(label: &str, wallet: &Wallet) {
    println!("{label}:");
    println!("  mnemonic: {}", wallet.mnemonic);
    println!("  address: {}", wallet.address);
    println!("  private_key_hex: {}", wallet.private_key_hex);
    println!("  public_key_hex: {}", hex::encode(wallet.public_key));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let wallet = Wallet::from_mnemonic(&args[1]).expect("mnemonic sahi nahi hai");
        print_wallet("wallet", &wallet);
    } else {
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        print_wallet("Alice", &alice);
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        print_wallet("Bob", &bob);
    }
}
