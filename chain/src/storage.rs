//! Disk storage — chain JSON file me save hoti hai.
//!
//! Ye REAL persistence hai: node band ho jaye to bhi chain disk par
//! rehti hai. Browser ka localStorage yahan kahin use nahi hota.
//! Load karte waqt chain poori verify hoti hai — file badli hui mili to
//! load hi nahi hoga (tamper-proof storage).

use crate::butpay::ButPayRegistry;
use crate::chain::Chain;
use crate::error::{ButError, Result};
use std::fs;
use std::path::Path;

/// Chain ko disk par JSON file me save karo.
/// Parent directory na ho to bana di jayegi.
pub fn save_chain(chain: &Chain, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(chain)
        .map_err(|e| crate::error::ButError::Storage(format!("chain JSON nahi bana: {e}")))?;
    fs::write(path, json)?;
    Ok(())
}

/// Chain ko disk se load karo — aur load karte hi poori verify karo.
/// File badli hui ya galat hui to error milega, chain nahi milegi.
pub fn load_chain(path: &Path) -> Result<Chain> {
    let (chain, _) = load_chain_with_registry(path)?;
    Ok(chain)
}

/// Chain + BUT Pay registry dono load karo — registry file hamesha
/// chain file ke saath hi rehti hai (wahi directory, naam `butpay.json`).
/// Registry na mile to khaali registry milti hai (pehli baar chalane par
/// aisa hi hota hai). Chain verify registry ke saath hoti hai, taaki
/// ID/handle wali transaction bhi check ho.
pub fn load_chain_with_registry(path: &Path) -> Result<(Chain, ButPayRegistry)> {
    let json = fs::read_to_string(path)?;
    let chain: Chain = serde_json::from_str(&json)?;
    let registry = load_butpay(&butpay_path_for(path))?;
    chain.verify_chain_with_registry(&registry)?;
    Ok((chain, registry))
}

/// BUT Pay registry save karo — file `data/butpay.json`
/// (chain file ke saath wahi directory me). Pehle temp file me likho,
/// phir rename karo — crash hone par bhi file aadhi nahi likhi jayegi.
pub fn save_butpay(path: &Path, registry: &ButPayRegistry) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(registry)
        .map_err(|e| ButError::Storage(format!("butpay registry JSON me nahi badli: {e}")))?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// BUT Pay registry load karo. File na mile to khaali registry milti hai
/// (pehli baar chalane par normal hai). File mili lekin data kharab hua
/// to error — galat registry se chain verify nahi hogi.
pub fn load_butpay(path: &Path) -> Result<ButPayRegistry> {
    match fs::read_to_string(path) {
        Ok(json) => {
            let registry: ButPayRegistry = serde_json::from_str(&json).map_err(|e| {
                ButError::Storage(format!(
                    "butpay file '{}' ka data kharab hai — JSON parse nahi hua: {e}",
                    path.display()
                ))
            })?;
            Ok(registry)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ButPayRegistry::new()),
        Err(e) => Err(ButError::Storage(format!(
            "butpay file '{}' padhi nahi gayi: {e}",
            path.display()
        ))),
    }
}

/// Chain file ke path se registry file ka path banao.
/// Jaise `data/chain.json` → `data/butpay.json`.
pub fn butpay_path_for(chain_path: &Path) -> std::path::PathBuf {
    match chain_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join("butpay.json"),
        _ => std::path::PathBuf::from("butpay.json"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Transaction;
    use crate::wallet::Wallet;

    #[test]
    fn save_load_roundtrip_verify() {
        let validator = Wallet::from_entropy(&[9u8; 32]).unwrap();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let mut chain = Chain::new_genesis(
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
        )
        .unwrap();
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = crate::block::Block::new(
            1,
            1_700_000_001,
            chain.latest_hash(),
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        chain.add_block(block).unwrap();

        let dir = std::env::temp_dir().join("but_chain_test_storage");
        let path = dir.join("chain.json");
        save_chain(&chain, &path).unwrap();
        let loaded = load_chain(&path).unwrap();
        assert_eq!(loaded.height(), chain.height());
        assert_eq!(loaded.balance(&bob.address), 100);
        assert_eq!(loaded.latest_hash(), chain.latest_hash());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_file_load_reject() {
        let validator = Wallet::from_entropy(&[9u8; 32]).unwrap();
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let chain = Chain::new_genesis(
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
        )
        .unwrap();
        let dir = std::env::temp_dir().join("but_chain_test_tamper");
        let path = dir.join("chain.json");
        save_chain(&chain, &path).unwrap();
        // File me SIRF stored balance badlo (genesis allocation nahi) —
        // load karte waqt verify fail hona chahiye, kyunki rebuilt state
        // stored state se match nahi karegi.
        let json = fs::read_to_string(&path).unwrap();
        let balance_entry = format!("\"{}\": 1000000", alice.address);
        assert!(json.contains(&balance_entry), "balance entry JSON me nahi mili");
        let tampered = json.replacen(
            &balance_entry,
            &format!("\"{}\": 9999999", alice.address),
            1,
        );
        fs::write(&path, tampered).unwrap();
        assert!(load_chain(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn butpay_save_load_roundtrip() {
        use crate::butpay::ButPayRegistry;
        let alice = Wallet::from_entropy(&[1u8; 32]).unwrap();
        let bob = Wallet::from_entropy(&[2u8; 32]).unwrap();
        let mut reg = ButPayRegistry::new();
        reg.register(&alice.address, Some("alice")).unwrap();
        reg.register(&bob.address, None).unwrap();
        let dir = std::env::temp_dir().join("but_chain_test_butpay");
        let chain_path = dir.join("chain.json");
        let reg_path = butpay_path_for(&chain_path);
        assert_eq!(reg_path, dir.join("butpay.json"));
        save_butpay(&reg_path, &reg).unwrap();
        let loaded = load_butpay(&reg_path).unwrap();
        assert_eq!(loaded.resolve("100001").unwrap(), alice.address);
        assert_eq!(loaded.resolve("@alice").unwrap(), alice.address);
        assert_eq!(loaded.resolve("100002").unwrap(), bob.address);
        assert_eq!(loaded.next_id, 100003);
        // File na ho to khaali registry milti hai, error nahi.
        let missing = load_butpay(&dir.join("nahi-hai.json")).unwrap();
        assert_eq!(missing.len(), 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
