//! Swap — AMM x·y=k, §11 liquidity rule, §14 routing rule (Phase 3b).
//!
//! Sanjay ke FINAL faisle (docs/TOKENOMICS-DRAFT.md):
//! - **§11 Liquidity rule:** Swap hamesha pool me available liquidity tak
//!   hi hoga. User ko swap se **PEHLE** bataya jayega ki use kitna milega
//!   (quote). User phir bhi poora swap kare to transaction **FAIL nahi
//!   hogi** — uske saare input coins pool me chale jayenge, use sirf
//!   available output milega, baad me kuch nahi milega (na IOU, na pending
//!   balance). User ki marzi — karwana hai to karwao, nahi to mat karwao.
//! - **§13 Sirf 2 coins:** BUT (main) aur BUT Coin (stablecoin). Dono ki
//!   liquidity **alag-alag** hogi — har coin ka apna alag pool.
//! - **§14 Routing rule:** Bitcoin (ya koi bhi bahar ka coin) seedha BUT
//!   Coin me swap nahi ho sakta — rasta hamesha BTC → BUT → BUT Coin.
//!
//! Kaise kaam karta hai:
//! - **Do alag pools:** BUT ka pool (`but_pool`, bits me) aur BUT Coin ka
//!   pool (`butcoin_pool`, smallest units me). Ek pool me dalna doosre ko
//!   nahi badalta — reserves hamesha alag-alag track hote hain.
//! - **AMM x·y=k pricing:** `out = reserve_out * amount_in /
//!   (reserve_in + amount_in)`. Output hamesha reserve se KAM hota hai —
//!   pool kabhi poora khaali nahi hota. Yehi mathematical cap §11 ka
//!   "sirf available liquidity tak" hai. Pool khaali ho (reserve_out 0)
//!   to output 0 — swap phir bhi fail nahi hoga (§11), user ko 0 milega.
//! - **Quote pehle:** `SwapState::quote` swap se pehle batata hai ki
//!   kitna milega, pool me kitna hai, aur note me saaf likha hota hai.
//!   Quote sirf snapshot hai — block bante waqt reserves badal sakte hain,
//!   isliye executed output quote se thoda alag ho sakta hai.
//! - **SwapOp signed hai** (StakeOp jaisa — Ed25519 + nonce, ek hi nonce
//!   sequence). Bina sahi signature ke reject, replay bhi nonce se rokta hai.
//! - **Koi fee nahi (3b):** swap fee ka faisla abhi pending hai — isliye
//!   abhi 0 fee hai. Fee aayegi to quote me dikhegi.
//! - **BUT Coin on-chain:** BUT Coin ke balances bhi chain state me hain
//!   (`Chain::butcoin_balances`) — 1 BUT Coin = 1,000 smallest units
//!   (BUT jaisa hi, 1 BUT = 1,000 bits). BUT Coin genesis allocations se
//!   aata hai; uski minting reserves se judi hai (reserves/audit/legal
//!   abhi pending — §13 — isliye yahan koi mint authority nahi hai).
//! - **Bahar ke coins on-chain nahi hain:** BTC/ETH ka balance chain par
//!   nahi hota — bridge abhi nahi bana hai. `SwapAsset::External` sirf
//!   routing check ke liye hai: seedha External → BUT Coin reject hota
//!   hai (§14), aur baaki external hops bhi "bridge abhi nahi bana" keh
//!   kar reject hote hain. On-chain sirf BUT ↔ BUT Coin swap hota hai.
//!
//! SACH BATA RAHE HAIN — kya abhi bhi simplified hai:
//! - **Liquidity nikaalna abhi nahi:** `AddLiquidity` se pool me coins
//!   daal sakte ho, par wapas nikaalne ka op abhi nahi hai (LP shares
//!   Phase 3c/testnet me). Testnet par liquidity dalna one-way hai.
//! - **Slippage protection nahi:** op me `min_out` nahi hai — §11 ke
//!   hisaab se swap kabhi fail nahi hota, isliye min_out ka matlab hi
//!   nahi hai. User ko quote dekh kar khud faisla karna hai.
//! - **Price oracle nahi:** pool ratio hi price hai (AMM). Bahar ke
//!   market se price link nahi hai — arbitrage se price theek hota hai
//!   (jaise har AMM me).

use crate::error::{ButError, Result};
use crate::hash::sha256_hex;
use crate::wallet::validate_address;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// 1 BUT Coin = kitne smallest units (BUT jaisa hi: 1 BUT = 1,000 bits).
pub const BUTCOIN_PER_COIN: u64 = 1_000;

/// Swap me kaun se coins hain — poori chain me sirf 2 coins hain (§13 FINAL).
/// Bahar ke coins (BTC/ETH) sirf routing check ke liye hain — unka balance
/// chain par nahi hota.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SwapAsset {
    /// BUT (main coin, native).
    But,
    /// BUT Coin (stablecoin).
    ButCoin,
    /// Bahar ka coin — "BTC", "ETH" (uppercase me normalize hota hai).
    /// Sirf routing check ke liye; on-chain iska balance nahi hota.
    External(String),
}

impl SwapAsset {
    /// Asset ka label — op payload aur display ke liye.
    /// BUT → "BUT", BUT Coin → "BUTCOIN", BTC → "EXT:BTC".
    pub fn label(&self) -> String {
        match self {
            SwapAsset::But => "BUT".to_string(),
            SwapAsset::ButCoin => "BUTCOIN".to_string(),
            SwapAsset::External(name) => format!("EXT:{name}"),
        }
    }

    /// Kya ye bahar ka coin hai (BTC/ETH...)?
    pub fn is_external(&self) -> bool {
        matches!(self, SwapAsset::External(_))
    }

    /// Kya ye chain ka apna coin hai (BUT ya BUT Coin)?
    pub fn is_native(&self) -> bool {
        !self.is_external()
    }

    /// String se asset banao — API/CLI ke liye.
    /// "BUT" → But, "BUTCOIN"/"BUT_COIN"/"BUT COIN" → ButCoin,
    /// baaki (jaise "BTC", "ETH") → External (uppercase, A-Z0-9, 2–10 chars).
    pub fn parse(input: &str) -> Result<Self> {
        let s = input.trim().to_uppercase().replace([' ', '_'], "");
        match s.as_str() {
            "BUT" => Ok(SwapAsset::But),
            "BUTCOIN" => Ok(SwapAsset::ButCoin),
            "" => Err(ButError::InvalidTransaction(
                "coin ka naam khaali nahi ho sakta".to_string(),
            )),
            _ => {
                if s.len() < 2 || s.len() > 10
                    || !s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                {
                    return Err(ButError::InvalidTransaction(format!(
                        "bahar ke coin ka naam 2–10 characters (A-Z, 0-9) hona chahiye — mila: {input}"
                    )));
                }
                Ok(SwapAsset::External(s))
            }
        }
    }
}

/// Swap ke pools — har coin ka apna alag pool (§13).
/// Chain state me save hota hai, verify ke waqt rebuild hota hai.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwapState {
    /// BUT ka alag pool — kitne bits reserve me hain.
    pub but_pool: u64,
    /// BUT Coin ka alag pool — kitne smallest units reserve me hain.
    pub butcoin_pool: u64,
}

impl SwapState {
    /// Khaali pools (genesis par dono 0).
    pub fn new() -> Self {
        Self::default()
    }

    /// Kisi coin ki reserve nikalo.
    pub fn reserve_of(&self, asset: &SwapAsset) -> u64 {
        match asset {
            SwapAsset::But => self.but_pool,
            SwapAsset::ButCoin => self.butcoin_pool,
            SwapAsset::External(_) => 0, // bahar ke coin ka pool nahi hota
        }
    }

    /// AMM x·y=k: `amount_in` dalne par kitna output milega.
    /// out = reserve_out * amount_in / (reserve_in + amount_in).
    /// u128 se overflow nahi hota. Output hamesha reserve_out se KAM hota
    /// hai — pool kabhi poora khaali nahi hota. reserve_out 0 ho to 0.
    pub fn amm_out(amount_in: u64, reserve_in: u64, reserve_out: u64) -> u64 {
        if amount_in == 0 || reserve_out == 0 {
            return 0;
        }
        ((reserve_out as u128 * amount_in as u128)
            / (reserve_in as u128 + amount_in as u128)) as u64
    }

    /// Swap se PEHLE quote do — user ko batane ke liye ki use kitna milega
    /// (§11 upfront disclosure). Quote sirf snapshot hai — block bante
    /// waqt reserves badal sakte hain.
    /// External → BUT Coin seedha quote nahi milega (§14 routing rule).
    pub fn quote(&self, from: &SwapAsset, to: &SwapAsset, amount_in: u64) -> Result<SwapQuote> {
        if from == to {
            return Err(ButError::InvalidTransaction(
                "swap ke liye from aur to alag coins hone chahiye".to_string(),
            ));
        }
        if amount_in == 0 {
            return Err(ButError::InvalidTransaction(
                "swap amount 0 se zyada hona chahiye".to_string(),
            ));
        }
        // §14 routing rule: bahar ka coin seedha BUT Coin me nahi ja sakta.
        if to == &SwapAsset::ButCoin && from.is_external() {
            return Err(ButError::InvalidTransaction(format!(
                "seedha {} → BUT Coin swap nahi ho sakta (§14) — rasta hamesha {} → BUT → BUT Coin hai: pehle BUT me aao, phir BUT se BUT Coin lo",
                from.label(),
                from.label()
            )));
        }
        if from.is_external() || to.is_external() {
            return Err(ButError::InvalidTransaction(
                "bahar ke coin ka swap on-chain nahi hota — bridge abhi nahi bana hai; on-chain sirf BUT ↔ BUT Coin swap hota hai".to_string(),
            ));
        }
        let reserve_in = self.reserve_of(from);
        let reserve_out = self.reserve_of(to);
        let expected_out = Self::amm_out(amount_in, reserve_in, reserve_out);
        let note = if reserve_out == 0 {
            "Pool khaali hai — is swap se kuch nahi milega (0 output). Swap phir bhi fail nahi hoga (§11), par tumhare saare input coins pool me chale jayenge. Pehle liquidity badhne ka wait karo ya chhota swap karo.".to_string()
        } else if expected_out == 0 {
            format!("Output 0 hoga — itne chhote swap par AMM se kuch nahi milega (pool me {reserve_out} available hai). Bada amount swap karo ya liquidity badhne ka wait karo.")
        } else {
            format!("{amount_in} {} ke badle lagbhag {expected_out} {} milega. Pool me abhi {reserve_out} {} available hai — milega utna hi jitna pool me hai (§11).", from.label(), to.label(), to.label())
        };
        Ok(SwapQuote {
            from: from.clone(),
            to: to.clone(),
            amount_in,
            expected_out,
            reserve_in,
            reserve_out,
            available_out: reserve_out,
            note,
        })
    }
}

/// Quote ka jawab — user ko swap se PEHLE dikhane ke liye (§11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwapQuote {
    /// Kis coin se swap kar rahe ho.
    pub from: SwapAsset,
    /// Kis coin me ja rahe ho.
    pub to: SwapAsset,
    /// Kitna input (smallest units).
    pub amount_in: u64,
    /// Kitna output milega (is waqt ke reserves ke hisaab se).
    pub expected_out: u64,
    /// Input wale pool ki reserve.
    pub reserve_in: u64,
    /// Output wale pool ki reserve.
    pub reserve_out: u64,
    /// Pool me kul kitna output available hai (= reserve_out).
    pub available_out: u64,
    /// Upfront disclosure — screen par dikhane ke liye.
    pub note: String,
}

/// Swap op ka type — block me kya karna hai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapOpKind {
    /// Ek coin se doosre coin me badlo (AMM x·y=k).
    Swap,
    /// Pool me liquidity dalo (coins user ke balance se pool reserve me).
    /// `from` == `to` hota hai (jis pool me dalna hai).
    AddLiquidity,
}

/// Block me jaane wala signed swap operation.
///
/// Transaction/StakeOp jaisa hi signed hota hai — Ed25519 signature + nonce.
/// Payload: `BUT-SWAP|<kind>|<from>|<to>|<amount>|<nonce>`.
/// - Swap: from != to, dono native (BUT/BUT Coin) ya routing-checked.
/// - AddLiquidity: from == to, native coin hi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwapOp {
    /// Swap / AddLiquidity.
    pub kind: SwapOpKind,
    /// Kis coin se (Swap: input coin; AddLiquidity: pool ka coin).
    pub from: SwapAsset,
    /// Kis coin me (Swap: output coin; AddLiquidity: `from` jaisa hi).
    pub to: SwapAsset,
    /// Kitne smallest units (input / deposit).
    pub amount: u64,
    /// Swap karne wale ka address ("but1...", 56 characters).
    pub swapper: String,
    /// Swapper ka nonce — transfer/stake op ke saath ek hi sequence.
    pub nonce: u64,
    /// Swapper ka Ed25519 public key (hex, 64 characters).
    pub swapper_public_key_hex: String,
    /// Ed25519 signature (hex, 128 characters) — op ID par sign.
    pub signature_hex: String,
}

impl SwapOp {
    /// Bina signature ke op banao (sign karne se pehle).
    pub fn new(kind: SwapOpKind, from: SwapAsset, to: SwapAsset, amount: u64, nonce: u64, swapper: String) -> Self {
        Self {
            kind,
            from,
            to,
            amount,
            swapper,
            nonce,
            swapper_public_key_hex: String::new(),
            signature_hex: String::new(),
        }
    }

    /// Swap ke liye shortcut.
    pub fn swap(from: SwapAsset, to: SwapAsset, amount: u64, nonce: u64, swapper: String) -> Self {
        Self::new(SwapOpKind::Swap, from, to, amount, nonce, swapper)
    }

    /// Liquidity dalne ke liye shortcut.
    pub fn add_liquidity(asset: SwapAsset, amount: u64, nonce: u64, swapper: String) -> Self {
        Self::new(SwapOpKind::AddLiquidity, asset.clone(), asset, amount, nonce, swapper)
    }

    /// Op ka ID — fields ka SHA-256 hash (signature ke bina).
    pub fn id(&self) -> String {
        sha256_hex(self.canonical_payload().as_bytes())
    }

    fn kind_str(&self) -> &'static str {
        match self.kind {
            SwapOpKind::Swap => "swap",
            SwapOpKind::AddLiquidity => "add-liquidity",
        }
    }

    /// Sign karne wala payload — hamesha ek hi format.
    /// Format badla to purani signature toot jayegi.
    fn canonical_payload(&self) -> String {
        format!(
            "BUT-SWAP|{}|{}|{}|{}|{}",
            self.kind_str(),
            self.from.label(),
            self.to.label(),
            self.amount,
            self.nonce
        )
    }

    /// Swapper ki key se op par sign karo.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        let public_key = signing_key.verifying_key().to_bytes();
        let expected = crate::wallet::address_from_public_key(&public_key);
        if expected != self.swapper {
            return Err(ButError::InvalidSignature(
                "sign karne wali key swapper address se match nahi karti".to_string(),
            ));
        }
        self.swapper_public_key_hex = hex::encode(public_key);
        let op_id = self.id();
        let signature = signing_key.sign(op_id.as_bytes());
        self.signature_hex = hex::encode(signature.to_bytes());
        Ok(())
    }

    /// Op verify karo — format + routing rules + signature.
    /// Nonce aur balance ki checking `Chain` karta hai (state janta hai).
    pub fn verify(&self) -> Result<()> {
        validate_address(&self.swapper)?;
        if self.amount == 0 {
            return Err(ButError::InvalidTransaction(
                "swap amount 0 se zyada hona chahiye".to_string(),
            ));
        }
        match self.kind {
            SwapOpKind::Swap => {
                if self.from == self.to {
                    return Err(ButError::InvalidTransaction(
                        "swap ke liye from aur to alag coins hone chahiye".to_string(),
                    ));
                }
                // §14 routing rule — bahar ka coin seedha BUT Coin me nahi.
                if self.to == SwapAsset::ButCoin && self.from.is_external() {
                    return Err(ButError::InvalidTransaction(format!(
                        "seedha {} → BUT Coin swap nahi ho sakta (§14) — rasta hamesha {} → BUT → BUT Coin hai: pehle BUT me aao, phir BUT se BUT Coin lo",
                        self.from.label(),
                        self.from.label()
                    )));
                }
                // Bridge abhi nahi bana — baaki external hops on-chain nahi.
                if self.from.is_external() || self.to.is_external() {
                    return Err(ButError::InvalidTransaction(
                        "bahar ke coin ka swap on-chain nahi hota — bridge abhi nahi bana hai; on-chain sirf BUT ↔ BUT Coin swap hota hai".to_string(),
                    ));
                }
            }
            SwapOpKind::AddLiquidity => {
                if self.from != self.to {
                    return Err(ButError::InvalidTransaction(
                        "liquidity dalne ke liye from aur to ek hi coin hone chahiye".to_string(),
                    ));
                }
                if self.from.is_external() {
                    return Err(ButError::InvalidTransaction(
                        "bahar ke coin ki liquidity on-chain pool me nahi dal sakte — bridge abhi nahi bana hai".to_string(),
                    ));
                }
            }
        }
        self.verify_signature()
    }

    fn verify_signature(&self) -> Result<()> {
        let pk_bytes = hex::decode(&self.swapper_public_key_hex).map_err(|_| {
            ButError::InvalidSignature("swapper public key hex format sahi nahi hai".to_string())
        })?;
        if pk_bytes.len() != 32 {
            return Err(ButError::InvalidSignature(
                "swapper public key 32 bytes ki honi chahiye".to_string(),
            ));
        }
        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&pk_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| {
            ButError::InvalidSignature("swapper public key valid Ed25519 key nahi hai".to_string())
        })?;
        if crate::wallet::address_from_public_key(&pk_array) != self.swapper {
            return Err(ButError::InvalidSignature(
                "public key swapper address se match nahi karti".to_string(),
            ));
        }
        let sig_bytes = hex::decode(&self.signature_hex).map_err(|_| {
            ButError::InvalidSignature("signature hex format sahi nahi hai".to_string())
        })?;
        if sig_bytes.len() != 64 {
            return Err(ButError::InvalidSignature(
                "signature 64 bytes (128 hex chars) ki honi chahiye".to_string(),
            ));
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_array);
        public_key
            .verify(self.id().as_bytes(), &signature)
            .map_err(|_| {
                ButError::InvalidSignature(
                    "swap op signature verify nahi hui — op badla gaya ya galat key se sign hua"
                        .to_string(),
                )
            })?;
        Ok(())
    }
}

/// BUT Coin smallest units ko dikhao — jaise 1500 → "1.5", 1000 → "1".
/// (BUT ke `units::format_but` jaisa hi — 1 BUT Coin = 1,000 units.)
pub fn format_butcoin(units: u64) -> String {
    let whole = units / BUTCOIN_PER_COIN;
    let frac = units % BUTCOIN_PER_COIN;
    if frac == 0 {
        return whole.to_string();
    }
    let frac_str = format!("{frac:03}").trim_end_matches('0').to_string();
    format!("{whole}.{frac_str}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn test_wallet(seed: u8) -> Wallet {
        Wallet::from_entropy(&[seed; 32]).unwrap()
    }

    #[test]
    fn amm_out_ganit_sahi_hai() {
        // Pool: x=100,000 BUT, y=100,000 BUT Coin. 10,000 BUT dalo:
        // out = 100,000 * 10,000 / 110,000 = 9,090.
        assert_eq!(SwapState::amm_out(10_000, 100_000, 100_000), 9_090);
        // Chhota swap: 100 dalo → 100,000*100/100,100 = 99.
        assert_eq!(SwapState::amm_out(100, 100_000, 100_000), 99);
        // Bahut bada swap: output reserve ke kareeb jata hai, par kabhi
        // reserve se zyada nahi (1,000,000 dalo → 90,909 < 100,000).
        assert_eq!(SwapState::amm_out(1_000_000, 100_000, 100_000), 90_909);
        // Pool khaali ho to output 0.
        assert_eq!(SwapState::amm_out(10_000, 100_000, 0), 0);
        // Input 0 ho to output 0.
        assert_eq!(SwapState::amm_out(0, 100_000, 100_000), 0);
        // Input pool (x) khaali ho to poora reserve milta hai (cap).
        assert_eq!(SwapState::amm_out(10_000, 0, 50_000), 50_000);
    }

    #[test]
    fn quote_pehle_batata_hai_kitna_milega() {
        let mut state = SwapState::new();
        state.but_pool = 100_000;
        state.butcoin_pool = 100_000;
        let q = state.quote(&SwapAsset::But, &SwapAsset::ButCoin, 10_000).unwrap();
        assert_eq!(q.expected_out, 9_090);
        assert_eq!(q.reserve_in, 100_000);
        assert_eq!(q.reserve_out, 100_000);
        assert_eq!(q.available_out, 100_000);
        assert!(q.note.contains("9,090") || q.note.contains("9090"));
        // Ulta quote bhi chalega.
        let q2 = state.quote(&SwapAsset::ButCoin, &SwapAsset::But, 10_000).unwrap();
        assert_eq!(q2.expected_out, 9_090);
    }

    #[test]
    fn quote_khaali_pool_par_zero_batata_hai() {
        let state = SwapState::new(); // dono pool 0
        let q = state.quote(&SwapAsset::But, &SwapAsset::ButCoin, 10_000).unwrap();
        assert_eq!(q.expected_out, 0);
        assert_eq!(q.available_out, 0);
        assert!(q.note.contains("khaali"));
    }

    #[test]
    fn quote_direct_btc_to_butcoin_reject() {
        let state = SwapState::new();
        // §14: BTC seedha BUT Coin me nahi ja sakta.
        let err = state
            .quote(&SwapAsset::External("BTC".to_string()), &SwapAsset::ButCoin, 10_000)
            .unwrap_err();
        assert!(err.to_string().contains("BTC → BUT"));
        // BTC → BUT ka quote bhi nahi (bridge abhi nahi bana).
        assert!(state
            .quote(&SwapAsset::External("BTC".to_string()), &SwapAsset::But, 10_000)
            .is_err());
        // Same coin quote reject.
        assert!(state
            .quote(&SwapAsset::But, &SwapAsset::But, 10_000)
            .is_err());
        // 0 amount quote reject.
        assert!(state
            .quote(&SwapAsset::But, &SwapAsset::ButCoin, 0)
            .is_err());
    }

    #[test]
    fn swap_op_sign_verify_aur_tamper() {
        let alice = test_wallet(1);
        let mut op = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            10_000,
            0,
            alice.address.clone(),
        );
        op.sign(&alice.signing_key()).unwrap();
        op.verify().unwrap();
        // Tamper: amount badla — signature toot gayi.
        let mut bad = op.clone();
        bad.amount = 999;
        assert!(bad.verify().is_err());
        // Bina signature reject.
        let unsigned = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            10_000,
            0,
            alice.address.clone(),
        );
        assert!(unsigned.verify().is_err());
        // Galat key se sign reject (sign() khud rokega).
        let bob = test_wallet(2);
        let mut op2 = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            10_000,
            0,
            alice.address.clone(),
        );
        assert!(op2.sign(&bob.signing_key()).is_err());
    }

    #[test]
    fn swap_op_routing_rule_verify_me_bhi_lagta_hai() {
        let alice = test_wallet(1);
        // Direct BTC → BUT Coin op — sign sahi hai, par routing rule reject karega.
        let mut op = SwapOp::swap(
            SwapAsset::External("BTC".to_string()),
            SwapAsset::ButCoin,
            10_000,
            0,
            alice.address.clone(),
        );
        op.sign(&alice.signing_key()).unwrap();
        let err = op.verify().unwrap_err();
        assert!(err.to_string().contains("BTC → BUT"));
        // ETH → BUT Coin bhi reject.
        let mut op2 = SwapOp::swap(
            SwapAsset::External("ETH".to_string()),
            SwapAsset::ButCoin,
            10_000,
            0,
            alice.address.clone(),
        );
        op2.sign(&alice.signing_key()).unwrap();
        assert!(op2.verify().is_err());
        // BTC → BUT bhi reject (bridge abhi nahi bana).
        let mut op3 = SwapOp::swap(
            SwapAsset::External("BTC".to_string()),
            SwapAsset::But,
            10_000,
            0,
            alice.address.clone(),
        );
        op3.sign(&alice.signing_key()).unwrap();
        assert!(op3.verify().is_err());
        // BUT → BUT Coin sahi hai.
        let mut op4 = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::ButCoin,
            10_000,
            0,
            alice.address.clone(),
        );
        op4.sign(&alice.signing_key()).unwrap();
        op4.verify().unwrap();
        // Same coin swap reject.
        let mut op5 = SwapOp::swap(
            SwapAsset::But,
            SwapAsset::But,
            10_000,
            0,
            alice.address.clone(),
        );
        op5.sign(&alice.signing_key()).unwrap();
        assert!(op5.verify().is_err());
    }

    #[test]
    fn add_liquidity_op_format_rules() {
        let alice = test_wallet(1);
        // Sahi: BUT pool me 5,000 dalo.
        let mut op = SwapOp::add_liquidity(SwapAsset::But, 5_000, 0, alice.address.clone());
        op.sign(&alice.signing_key()).unwrap();
        op.verify().unwrap();
        // Galat: from != to wala AddLiquidity reject.
        let mut bad = SwapOp::new(
            SwapOpKind::AddLiquidity,
            SwapAsset::But,
            SwapAsset::ButCoin,
            5_000,
            0,
            alice.address.clone(),
        );
        bad.sign(&alice.signing_key()).unwrap();
        assert!(bad.verify().is_err());
        // Galat: external coin ki liquidity reject.
        let mut bad2 = SwapOp::add_liquidity(
            SwapAsset::External("BTC".to_string()),
            5_000,
            0,
            alice.address.clone(),
        );
        bad2.sign(&alice.signing_key()).unwrap();
        assert!(bad2.verify().is_err());
        // Galat: 0 amount reject.
        let mut bad3 = SwapOp::add_liquidity(SwapAsset::But, 0, 0, alice.address.clone());
        bad3.sign(&alice.signing_key()).unwrap();
        assert!(bad3.verify().is_err());
    }

    #[test]
    fn asset_parse_sahi_hai() {
        assert_eq!(SwapAsset::parse("BUT").unwrap(), SwapAsset::But);
        assert_eq!(SwapAsset::parse("but").unwrap(), SwapAsset::But);
        assert_eq!(SwapAsset::parse("BUTCOIN").unwrap(), SwapAsset::ButCoin);
        assert_eq!(SwapAsset::parse("but coin").unwrap(), SwapAsset::ButCoin);
        assert_eq!(SwapAsset::parse("BUT_COIN").unwrap(), SwapAsset::ButCoin);
        assert_eq!(
            SwapAsset::parse("btc").unwrap(),
            SwapAsset::External("BTC".to_string())
        );
        assert_eq!(
            SwapAsset::parse("ETH").unwrap(),
            SwapAsset::External("ETH".to_string())
        );
        assert!(SwapAsset::parse("").is_err());
        assert!(SwapAsset::parse("B").is_err()); // bahut chhota
        assert!(SwapAsset::parse("VERYLONGCOINNAME").is_err()); // bahut bada
    }

    #[test]
    fn format_butcoin_sahi_hai() {
        assert_eq!(format_butcoin(0), "0");
        assert_eq!(format_butcoin(1), "0.001");
        assert_eq!(format_butcoin(1000), "1");
        assert_eq!(format_butcoin(1500), "1.5");
        assert_eq!(format_butcoin(1010), "1.01");
    }
}
