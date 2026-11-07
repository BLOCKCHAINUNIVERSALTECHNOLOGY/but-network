//! BUT Pay — chhota ID / naam se send-receive.
//!
//! Sanjay ka sawal tha: Binance ki tarah har wallet ko ek chhoti unique
//! ID/numbering mile, jo wallet address ke saath judi ho — address lamba
//! lage to user chhote number se bhi send/receive kar sake. Ye system
//! **BUT Pay** naam se kaam karta hai.
//!
//! Kaise kaam karta hai:
//! - Har wallet ko ek **numeric ID** milti hai — 100001 se shuru, har naye
//!   wallet ke liye +1 (Binance UID ki tarah, sirf number).
//! - User chahe to apna **handle/naam** bhi le sakta hai — jaise `@sanjay`
//!   (UPI ID / ENS naam ki tarah, pehle aao pehle pao).
//! - ID aur handle dono **public** hote hain — receive karne ke liye safe
//!   hain, private key se inka koi lena-dena nahi hai.
//! - Send karte waqt user ID ya handle likhta hai; system use address me
//!   badal kar normal transaction bhejta hai.
//!
//! SABSE AHEM RULE — registry immutable hai:
//! Ek baar ID/handle kisi address se jud gaya to wo **kabhi nahi badlega**
//! — na re-point, na delete, na transfer. Isi liye ID par sign ki hui
//! transaction utni hi binding hoti hai jitni address par sign ki hui:
//! "100001" ka matlab hamesha ek hi address rahega, isliye signature ka
//! matlab kabhi nahi badlega. Agar ID re-point ho sakti to signature ka
//! matlab badal jata — isliye ye rule todna mana hai.
//!
//! Persistence:
//! Registry chain ke saath disk par save hoti hai — `butpay.json` file me,
//! chain file (`chain.json`) ke bagal me. Chain file + registry file
//! dono saath chalte hain: chain me ID wali transaction ho to chain
//! verify karne ke liye registry bhi chahiye hoti hai (kyunki receiver
//! ka address registry se hi nikalta hai). File na ho to khaali registry
//! mani jati hai (koi ID register nahi hui).
//!
//! SACH BATA RAHE HAIN — kya abhi bhi simplified hai:
//! - Registry **local file** me hai — har node ki apni copy. Nodes ke
//!   beech registry sync (gossip) abhi nahi hai; P2P sync me ID wali
//!   transaction wala block reject ho jayega (kyunki sync karne wale
//!   node ke paas registry nahi hoti). Testnet me registry file ko
//!   nodes par haath se copy karna hoga, ya Phase 3 me gossip jodega.
//! - Handle lena pehle-aao-pehle-pao hai — koi trademark/brand check nahi.
//!   Koi mashhoor naam (jaise kisi company ka naam) pehle le le to
//!   dispute suljhane ka koi tareeka abhi nahi hai.
//! - ID/handle public hain — koi bhi dekh sakta hai ki "100001" kis
//!   address ka hai. Privacy ke liye ye address jaisa hi hai (public).

use crate::error::{ButError, Result};
use crate::wallet::validate_address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pehli BUT Pay ID — 100001 se shuru (Binance UID ki tarah chhota number).
pub const BUTPAY_FIRST_ID: u64 = 100001;

/// Handle ke rules: 3–32 characters.
pub const HANDLE_MIN_LEN: usize = 3;
pub const HANDLE_MAX_LEN: usize = 32;

/// System ke naam — koi user ye handle nahi le sakta.
/// (Nakal rokne ke liye: koi "admin" ya "butpay" naam se dhokha na de.)
pub const RESERVED_HANDLES: &[&str] = &[
    "but", "butpay", "butcoin", "admin", "support", "official", "team", "help", "system",
    "root", "mod", "security",
];

/// Ek BUT Pay entry — lookup ka jawab: ID + address + handle (agar ho).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButPayEntry {
    /// Numeric ID (100001 se shuru).
    pub id: u64,
    /// Wallet address (56 chars, "but1" se shuru).
    pub address: String,
    /// Handle bina "@" ke (jaise "sanjay"), na ho to `None`.
    pub handle: Option<String>,
}

/// BUT Pay registry — ID/handle se address ka map.
/// Ek baar entry jud gayi to kabhi nahi badalti (immutable).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ButPayRegistry {
    /// Numeric ID (100001 se shuru) → wallet address.
    pub by_id: HashMap<u64, String>,
    /// Normalized handle ("sanjay") → wallet address.
    pub by_handle: HashMap<String, String>,
    /// Wallet address → numeric ID (ulta lookup, duplicate rokne ke liye).
    pub by_address: HashMap<String, u64>,
    /// Wallet address → handle (agar liya ho to; display ke liye).
    pub handles_by_address: HashMap<String, String>,
    /// Agli ID jo di jayegi (pehli baar 100001).
    pub next_id: u64,
}

impl ButPayRegistry {
    /// Khaali registry — agli ID 100001 hogi.
    pub fn new() -> Self {
        Self {
            by_id: HashMap::new(),
            by_handle: HashMap::new(),
            by_address: HashMap::new(),
            handles_by_address: HashMap::new(),
            next_id: BUTPAY_FIRST_ID,
        }
    }

    /// Registry me kitni IDs hain.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Khaali hai ya nahi.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Nayi ID register karo — address ko agla number do.
    /// `handle`: optional naam (jaise "sanjay" ya "@sanjay").
    /// Rules (koi ek bhi toota to reject):
    /// 1. Address valid `but1...` (56 chars) hona chahiye.
    /// 2. Is address ki ID pehle se nahi honi chahiye — ek address ki
    ///    ek hi ID hoti hai (dobara register karne par error me pehli
    ///    wali ID bata di jati hai).
    /// 3. Handle diya ho to wo valid hona chahiye aur kisi aur ka nahi
    ///    hona chahiye (pehle aao pehle pao).
    /// Return: nayi ID (100001 se shuru).
    pub fn register(&mut self, address: &str, handle: Option<&str>) -> Result<u64> {
        validate_address(address)?;
        if let Some(existing) = self.by_address.get(address) {
            return Err(ButError::InvalidTransaction(format!(
                "is address ki BUT Pay ID pehle se hai: {existing} — ek address ki ek hi ID hoti hai, dobara nahi milegi"
            )));
        }
        let normalized_handle: Option<String> = match handle {
            Some(h) => {
                let h = normalize_handle(h);
                validate_handle(&h)?;
                if self.by_handle.contains_key(&h) {
                    return Err(ButError::InvalidTransaction(format!(
                        "ye naam pehle se liya gaya hai: @{h} — koi aur naam chuno"
                    )));
                }
                Some(h)
            }
            None => None,
        };
        let id = self.next_id;
        self.next_id += 1;
        self.by_id.insert(id, address.to_string());
        self.by_address.insert(address.to_string(), id);
        if let Some(h) = normalized_handle {
            self.by_handle.insert(h.clone(), address.to_string());
            self.handles_by_address.insert(address.to_string(), h);
        }
        Ok(id)
    }

    /// ID/handle/address se poori entry nikalo (lookup ke liye).
    /// Jaise `lookup("100001")`, `lookup("@sanjay")` ya
    /// `lookup("but1...")` — teeno se wahi entry milti hai.
    /// Address register nahi hua ho (sirf pass-through address) to `None`.
    pub fn lookup(&self, query: &str) -> Option<ButPayEntry> {
        let address = self.resolve(query)?;
        let id = *self.by_address.get(&address)?;
        Some(ButPayEntry {
            id,
            address: address.clone(),
            handle: self.handles_by_address.get(&address).cloned(),
        })
    }

    /// ID, handle ya address se address nikalo.
    /// - Sirf digits hon (jaise "100001") to numeric ID samjho.
    /// - Valid `but1...` address ho to wahi wapas do (pass-through).
    /// - Warna handle samjho ("@sanjay" ya "sanjay" — dono chalenge).
    /// Na mile to `None`.
    pub fn resolve(&self, input: &str) -> Option<String> {
        let s = input.trim();
        if s.is_empty() {
            return None;
        }
        if let Ok(n) = s.parse::<u64>() {
            return self.by_id.get(&n).cloned();
        }
        if validate_address(s).is_ok() {
            return Some(s.to_string());
        }
        let h = normalize_handle(s);
        self.by_handle.get(&h).cloned()
    }

    /// Transaction ke receiver ke liye resolve karo — na mile to error.
    pub fn resolve_for_tx(&self, receiver: &str) -> Result<String> {
        self.resolve(receiver).ok_or_else(|| {
            ButError::InvalidTransaction(format!(
                "BUT Pay ID/naam nahi mila: {receiver} — pehle register karo ya sahi ID/naam likho"
            ))
        })
    }

    /// Address ki ID nikalo (na ho to None).
    pub fn id_for_address(&self, address: &str) -> Option<u64> {
        self.by_address.get(address).copied()
    }

    /// Address ka handle nikalo (na liya ho to None).
    pub fn handle_for_address(&self, address: &str) -> Option<&str> {
        self.handles_by_address.get(address).map(|s| s.as_str())
    }

    /// Saari entries (ID ke order me) — P2P sync ke liye.
    /// Entries `ButPayEntry` ki list hoti hain (id, address, handle).
    pub fn entries(&self) -> Vec<ButPayEntry> {
        let mut ids: Vec<u64> = self.by_id.keys().copied().collect();
        ids.sort_unstable();
        ids.iter()
            .filter_map(|id| {
                let address = self.by_id.get(id)?.clone();
                Some(ButPayEntry {
                    id: *id,
                    address: address.clone(),
                    handle: self.handles_by_address.get(&address).cloned(),
                })
            })
            .collect()
    }

    /// Doosre node ki registry ko apni registry me milao (P2P sync).
    /// Immutable-merge rules — entry ek baar jud gayi to kabhi nahi badalti:
    /// 1. Jo entry pehle se hai (wahi ID + wahi address) wo jaisi ki taisi.
    /// 2. **Conflict reject:** wahi ID kisi AUR address par ho, ya wahi
    ///    address kisi AUR ID par ho, ya wahi handle kisi AUR address par
    ///    ho — poora merge reject hota hai (kuch bhi nahi milta). Aisa tab
    ///    hota hai jab do nodes ne ek hi ID do alag logo ko de di ho —
    ///    uska hal manual hai (kis node ki chain sahi hai, wo dekhna).
    /// 3. `next_id` = dono me se bada (taaki agli ID kabhi takraye nahi).
    /// Returns: kitni nayi entries judi.
    pub fn merge(&mut self, other: &ButPayRegistry) -> Result<usize> {
        // Pehle saare conflict check karo — ek bhi conflict ho to kuch
        // bhi merge mat karo (all-or-nothing).
        for (id, address) in &other.by_id {
            if let Some(existing) = self.by_id.get(id) {
                if existing != address {
                    return Err(ButError::InvalidTransaction(format!(
                        "BUT Pay registry conflict: ID {id} is node par {existing} hai, doosre node par {address} — merge nahi hoga"
                    )));
                }
            }
            if let Some(existing_id) = self.by_address.get(address) {
                if existing_id != id {
                    return Err(ButError::InvalidTransaction(format!(
                        "BUT Pay registry conflict: {address} ki ID is node par {existing_id} hai, doosre node par {id} — merge nahi hoga"
                    )));
                }
            }
        }
        for (handle, address) in &other.by_handle {
            if let Some(existing) = self.by_handle.get(handle) {
                if existing != address {
                    return Err(ButError::InvalidTransaction(format!(
                        "BUT Pay registry conflict: naam @{handle} is node par {existing} hai, doosre node par {address} — merge nahi hoga"
                    )));
                }
            }
        }
        // Koi conflict nahi — nayi entries jodo.
        let mut added = 0;
        for (id, address) in &other.by_id {
            if self.by_id.contains_key(id) {
                continue;
            }
            self.by_id.insert(*id, address.clone());
            self.by_address.insert(address.clone(), *id);
            if let Some(h) = other.handles_by_address.get(address) {
                self.by_handle.insert(h.clone(), address.clone());
                self.handles_by_address.insert(address.clone(), h.clone());
            }
            added += 1;
        }
        // next_id bada wala rakho (dono me se).
        if other.next_id > self.next_id {
            self.next_id = other.next_id;
        }
        Ok(added)
    }
}

/// Handle normalize karo: aas-paas ki khaali jagah hatao, aage ka '@'
/// hatao, chhote akshar me karo. ("@Sanjay" → "sanjay")
pub fn normalize_handle(input: &str) -> String {
    input.trim().trim_start_matches('@').to_lowercase()
}

/// Handle ke rules check karo:
/// - 3–32 characters.
/// - Sirf chhote a-z, 0-9, underscore.
/// - Pehla character letter hona chahiye (sirf number wala handle nahi —
///   warna numeric ID se takra jayega).
/// - Reserved naam nahi hona chahiye.
pub fn validate_handle(handle: &str) -> Result<()> {
    if handle.len() < HANDLE_MIN_LEN || handle.len() > HANDLE_MAX_LEN {
        return Err(ButError::InvalidTransaction(format!(
            "handle {HANDLE_MIN_LEN}–{HANDLE_MAX_LEN} characters ka hona chahiye, mila: {}",
            handle.len()
        )));
    }
    if !handle
        .chars()
        .next()
        .map(|c| c.is_ascii_lowercase())
        .unwrap_or(false)
    {
        return Err(ButError::InvalidTransaction(
            "handle ka pehla character a-z letter hona chahiye".to_string(),
        ));
    }
    if !handle
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(ButError::InvalidTransaction(
            "handle me sirf a-z, 0-9 aur underscore (_) chalenge — space ya khaas character nahi".to_string(),
        ));
    }
    if RESERVED_HANDLES.contains(&handle) {
        return Err(ButError::InvalidTransaction(format!(
            "ye naam system ke liye reserved hai: @{handle} — koi aur naam chuno"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn test_wallet(seed_byte: u8) -> Wallet {
        Wallet::from_entropy(&[seed_byte; 32]).unwrap()
    }

    #[test]
    fn pehli_id_100001_se_shuru() {
        let mut reg = ButPayRegistry::new();
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        assert_eq!(reg.register(&alice.address, None).unwrap(), 100001);
        assert_eq!(reg.register(&bob.address, None).unwrap(), 100002);
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn handle_register_aur_lookup() {
        let mut reg = ButPayRegistry::new();
        let alice = test_wallet(1);
        let id = reg.register(&alice.address, Some("@Sanjay")).unwrap();
        assert_eq!(id, 100001);
        // Handle normalize hota hai: @Sanjay → sanjay.
        assert_eq!(reg.resolve("sanjay").unwrap(), alice.address);
        assert_eq!(reg.resolve("@sanjay").unwrap(), alice.address);
        assert_eq!(reg.resolve("@SANJAY").unwrap(), alice.address);
        assert_eq!(reg.resolve("100001").unwrap(), alice.address);
        assert_eq!(reg.id_for_address(&alice.address), Some(100001));
        assert_eq!(reg.handle_for_address(&alice.address), Some("sanjay"));
    }

    #[test]
    fn duplicate_address_reject() {
        let mut reg = ButPayRegistry::new();
        let alice = test_wallet(1);
        reg.register(&alice.address, None).unwrap();
        let err = reg.register(&alice.address, Some("alice2")).unwrap_err();
        assert!(err.to_string().contains("100001"));
        // Dobara register karne par next_id nahi badhna chahiye.
        let bob = test_wallet(2);
        assert_eq!(reg.register(&bob.address, None).unwrap(), 100002);
    }

    #[test]
    fn duplicate_handle_reject() {
        let mut reg = ButPayRegistry::new();
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        reg.register(&alice.address, Some("sanjay")).unwrap();
        assert!(reg.register(&bob.address, Some("sanjay")).is_err());
        assert!(reg.register(&bob.address, Some("@SANJAY")).is_err());
        // Bob bina handle ke register ho sakta hai.
        assert_eq!(reg.register(&bob.address, None).unwrap(), 100002);
    }

    #[test]
    fn galat_address_reject() {
        let mut reg = ButPayRegistry::new();
        assert!(reg.register("but1abc", None).is_err());
        assert!(reg.register("not-an-address", Some("sanjay")).is_err());
    }

    #[test]
    fn galat_handle_reject() {
        let mut reg = ButPayRegistry::new();
        let alice = test_wallet(1);
        // Bahut chhota.
        assert!(reg.register(&alice.address, Some("ab")).is_err());
        // Pehla character digit.
        assert!(reg.register(&alice.address, Some("1sanjay")).is_err());
        // Khaas character / space.
        assert!(reg.register(&alice.address, Some("san jay")).is_err());
        assert!(reg.register(&alice.address, Some("sanjay!")).is_err());
        // Reserved naam.
        assert!(reg.register(&alice.address, Some("admin")).is_err());
        assert!(reg.register(&alice.address, Some("butpay")).is_err());
    }

    #[test]
    fn unknown_id_aur_handle_resolve_none() {
        let reg = ButPayRegistry::new();
        assert!(reg.resolve("999999").is_none());
        assert!(reg.resolve("koi-nahi-hai").is_none());
        assert!(reg.resolve("").is_none());
        assert!(reg.resolve("   ").is_none());
        assert!(reg.resolve_for_tx("999999").is_err());
    }

    #[test]
    fn address_passthrough() {
        let reg = ButPayRegistry::new();
        let alice = test_wallet(1);
        // Valid address seedha wapas milta hai — register kiye bina bhi.
        assert_eq!(reg.resolve(&alice.address).unwrap(), alice.address);
        assert_eq!(reg.resolve_for_tx(&alice.address).unwrap(), alice.address);
    }

    #[test]
    fn save_load_roundtrip_next_id_bachta_hai() {
        let mut reg = ButPayRegistry::new();
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        reg.register(&alice.address, Some("sanjay")).unwrap();
        reg.register(&bob.address, None).unwrap();
        let dir = std::env::temp_dir().join("but_butpay_test_save_load");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("butpay.json");
        crate::storage::save_butpay(&path, &reg).unwrap();
        let loaded = crate::storage::load_butpay(&path).unwrap();
        assert_eq!(loaded.resolve("100001").unwrap(), alice.address);
        assert_eq!(loaded.resolve("sanjay").unwrap(), alice.address);
        assert_eq!(loaded.resolve("100002").unwrap(), bob.address);
        assert_eq!(loaded.next_id, 100003);
        // Load ke baad agla register 100003 de.
        let mut loaded = loaded;
        let carol = test_wallet(3);
        assert_eq!(loaded.register(&carol.address, None).unwrap(), 100003);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_nayi_entries_jodta_hai_next_id_bada_rakhta_hai() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let carol = test_wallet(3);
        // Node A: Alice (100001, @sanjay) + Bob (100002).
        let mut a = ButPayRegistry::new();
        a.register(&alice.address, Some("sanjay")).unwrap();
        a.register(&bob.address, None).unwrap();
        // Node B: wahi Alice + Bob + Carol (100003).
        let mut b = ButPayRegistry::new();
        b.register(&alice.address, Some("sanjay")).unwrap();
        b.register(&bob.address, None).unwrap();
        b.register(&carol.address, Some("carol")).unwrap();
        // A me B merge karo — sirf Carol nayi hai.
        let added = a.merge(&b).unwrap();
        assert_eq!(added, 1);
        assert_eq!(a.len(), 3);
        assert_eq!(a.resolve("carol").unwrap(), carol.address);
        assert_eq!(a.resolve("100003").unwrap(), carol.address);
        // next_id bada wala (100004).
        assert_eq!(a.next_id, 100004);
        // Dobara merge — kuch naya nahi.
        assert_eq!(a.merge(&b).unwrap(), 0);
        // entries() ID order me aati hain.
        let entries = a.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, 100001);
        assert_eq!(entries[2].id, 100003);
    }

    #[test]
    fn merge_id_conflict_reject_poora_merge_nahi_hota() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let carol = test_wallet(3);
        // Node A: Alice ko 100001 di.
        let mut a = ButPayRegistry::new();
        a.register(&alice.address, None).unwrap();
        // Node B: Bob ko 100001 di (takraav!) + Carol ko 100002.
        let mut b = ButPayRegistry::new();
        b.register(&bob.address, None).unwrap();
        b.register(&carol.address, None).unwrap();
        // Merge reject hona chahiye — aur A me kuch bhi nahi judna chahiye.
        let err = a.merge(&b).unwrap_err();
        assert!(err.to_string().contains("conflict"));
        assert_eq!(a.len(), 1);
        assert_eq!(a.next_id, 100002);
        assert!(a.resolve("100002").is_none());
    }

    #[test]
    fn merge_handle_conflict_reject() {
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let mut a = ButPayRegistry::new();
        a.register(&alice.address, Some("sanjay")).unwrap();
        let mut b = ButPayRegistry::new();
        b.register(&bob.address, Some("sanjay")).unwrap();
        assert!(a.merge(&b).is_err());
        assert_eq!(a.len(), 1);
    }
}
