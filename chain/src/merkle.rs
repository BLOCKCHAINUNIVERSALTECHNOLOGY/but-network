//! Merkle tree — saare transactions ka ek root hash.
//!
//! Ek transaction badli to root badal jayega, block ka hash badal jayega,
//! aur chain verify fail ho jayegi. Isi se tampering pakdi jati hai.
//!
//! Rule: har level par akele bache hash ko khud se jodo (duplicate),
//! taaki tree hamesha binary rahe. Ye standard tareeka hai.

use crate::hash::sha256_hex;

/// Transaction IDs ki list se Merkle root banao (hex string).
/// Khaali list ka root khaali string hota hai (genesis jaisi khaali block).
pub fn merkle_root(tx_ids: &[String]) -> String {
    if tx_ids.is_empty() {
        return String::new();
    }
    let mut level: Vec<String> = tx_ids.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let left = &pair[0];
            let right = pair.get(1).unwrap_or(left); // akele ko khud se jodo
            next.push(sha256_hex(format!("{left}{right}").as_bytes()));
        }
        level = next;
    }
    level.into_iter().next().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn khaali_root_khaali() {
        assert_eq!(merkle_root(&[]), "");
    }

    #[test]
    fn ek_tx_root_tx_id() {
        let ids = vec!["abc".to_string()];
        assert_eq!(merkle_root(&ids), "abc");
    }

    #[test]
    fn tamper_se_root_badle() {
        let a = vec!["tx1".to_string(), "tx2".to_string()];
        let b = vec!["tx1".to_string(), "tx3".to_string()];
        assert_ne!(merkle_root(&a), merkle_root(&b));
    }

    #[test]
    fn order_badla_to_root_badle() {
        let a = vec!["tx1".to_string(), "tx2".to_string()];
        let b = vec!["tx2".to_string(), "tx1".to_string()];
        assert_ne!(merkle_root(&a), merkle_root(&b));
    }
}
