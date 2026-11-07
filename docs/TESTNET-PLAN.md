# BUT Network — Testnet Plan (taiyari, 7 Nov 2026)

> **Testnet abhi launch nahi hua hai.** Ye document testnet ki taiyari hai —
> faucet code + plan. Asli testnet ke liye hosted node + HTTPS chahiye,
> jo abhi nahi hai. Neeche imaandari se likha hai ki kya bana hai aur
> kya baaki hai.

---

## 1. Testnet kya hai aur kyun chahiye

**Testnet = nakli coins wala test network.** Is par asli paisa nahi hota —
coins free milte hain (faucet se), aur kuch bhi toot jaye to koi nuksaan
nahi hota.

Kyun chahiye:
- **Wallet, BUT Pay, swap, staking** — sab kuch asli users se pehle
  testnet par try hota hai, bina asli paisa lagaye.
- **Bugs milte hain** — mainnet par bug mile to asli paisa doob sakta hai;
  testnet par bug mile to sirf test coins doobte hain.
- **Red Team / Blue Team testing** — ek team todne ki koshish karti hai,
  doosri team bachati hai. Ye testnet par hota hai, mainnet par nahi.
- **Community seekhti hai** — log testnet par wallet chalana, swap karna,
  stake karna seekhte hain, phir mainnet par aate hain.

**Testnet ≠ mainnet:** testnet ke coins ki koi value nahi hai. Jo testnet
par kamao, wo mainnet par nahi milega. Ye baat har jagah saaf likhni hogi.

---

## 2. Testnet node kaise chalaye (local, abhi ke liye)

```bash
cd chain
# Testnet node chalao (faucet ON) — default port 3030, default file data/chain.json
cargo run -- testnet [port] [chain-file]

# Jaise:
cargo run -- testnet 3031 data/testnet-chain.json
```

Pehli baar chalane par **testnet genesis** banta hai:
- Alice (demo wallet) ko 1,000,000 bits (= 1,000 BUT)
- **Faucet wallet ko 100,000,000 BUT test coins** (faucet se baantne ke liye)

Normal node (`but-node serve`) se fark: us par faucet **band** rehta hai.
Testnet node par hi faucet chalta hai.

> **Node band karne ke baad chain file + `faucet.json` dono save rehte hain.**
> Dobara chalane par wahi chain + wahi faucet state load hoti hai —
> rate limit restart se reset nahi hoti.

---

## 3. Faucet — free test coins kaise milte hain

Faucet testnet par free test coins dene wala hai.

**Rules (FINAL — testnet ke liye):**
- **Ek drip = 1,000 BUT test coins** (= 1,000,000 bits). Ye raqam code me
  configurable hai (`FaucetConfig.drip_amount_bits`), par default yahi hai.
- **Ek address din me sirf 1 baar** claim kar sakta hai (24 ghante ka cooldown).
- **Ek IP se din me zyada se zyada 10 alag addresses** claim kar sakte hain.
- **Ye test coins hain — inki koi real value nahi hai.** Inhe becha,
  khareeda ya asli paisa mana nahi ja sakta.
- **Mainnet par faucet kabhi nahi chalega.**

**Kaise milega (API):**
```bash
# Test coins maango
curl -X POST http://127.0.0.1:3031/faucet \
  -H 'Content-Type: application/json' \
  -d '{"address": "but1..."}'
# Jawab: {"ok": true, "tx_id": "...", "amount_but": "1000", ...}

# Coins agle block mine hone par milenge:
curl -X POST http://127.0.0.1:3031/mine

# Balance dekho:
curl http://127.0.0.1:3031/balance/but1...

# Faucet ki status dekho:
curl http://127.0.0.1:3031/faucet/status
```

**Kaise milega (CLI — seedha mine karke):**
```bash
cargo run -- faucet <address> [chain-file]
# Jaise: cargo run -- faucet but1af50... data/testnet-chain.json
# Coins turant block me jud jate hain (dev validator proposer hona chahiye).
```

**Faucet kaise kaam karta hai (imaandari se):**
- Faucet seedha balance nahi badalta — wo verify tod dega. Faucet ek
  wallet hai (testnet-only dev key, entropy `[7u8; 32]`) jo testnet
  genesis me fund hota hai; wo asli signed transaction bhejta hai.
- Rate limit `faucet.json` me save hoti hai (chain file ke saath) —
  node restart hone par bhi yaad rehti hai.
- **Localhost dev me sab 127.0.0.1 se aate hain** — isliye IP limit wahan
  spam rokne ke liye hai, per-person limit nahi. Asli testnet par node
  apne proxy ke peeche `X-Forwarded-For` se real IP lega. Us header par
  sirf tab bharosa karo jab proxy tumhara ho — koi bhi client use jhooth
  bhi likh sakta hai.
- **CLI faucet aur chalta hua node ek hi chain file share na karein** —
  CLI seedha file se block mine karta hai, node ke paas purani state
  memory me rehti hai. Node chal raha ho to API `POST /faucet` use karo.

---

## 4. Testnet par kya-kya test karna hai

| # | Kya test karna hai | Kaise | Status |
|---|---|---|---|
| 1 | **Wallet banao + seed backup** | Web wallet v1 (24 words, verify) | Code bana hai, real browser test baaki |
| 2 | **Faucet se test coins lo** | `POST /faucet` → `/mine` → balance | **Local par PASS** (Bob ko 1,000 BUT mile) |
| 3 | **Send / Receive** | Wallet se tx bhejo, history dekho | Code bana hai, real browser test baaki |
| 4 | **BUT Pay** | ID/handle se bhejo (100001, @bob) | Chain + API par PASS, app test baaki |
| 5 | **Swap** | Quote dekho → swap karo (§11 rule) | Chain + API par PASS, app me nahi juda |
| 6 | **Staking** | Stake karo, lock dekho, unstake (72h release) | Chain + API par PASS, app me nahi juda |
| 7 | **P2P sync** | 2 nodes, ek se doosre par sync | `p2p-demo` PASS (localhost par) |
| 8 | **Slashing** | Equivocation report → slash | Chain par PASS |
| 9 | **Spam / rate limit** | Faucet ko baar-baar maango, galat address do | Local par PASS (reject hota hai) |
| 10 | **Node restart** | Node band karo → chalao → chain + faucet state waisi ki waisi | Faucet state save/load test PASS; poora restart flow abhi manual check baaki |
| 11 | **Alag machines par nodes** | Do alag computers par node chalakar sync | **Baaki hai** — abhi sirf localhost test hua hai |
| 12 | **Red Team / Blue Team** | Todne ki koshish + bachav | **Baaki hai** |

---

## 5. Known limits (imaandari se — testnet launch se pehle pata hona chahiye)

- **API localhost-only hai** — koi auth nahi, HTTPS nahi. Internet par
  khula node koi bhi chala sakta hai (galat tx waise bhi signature check
  par reject hogi, par spam ho sakta hai).
- **P2P me encryption / peer discovery / fork-choice nahi hai** — peers
  haath se dene padte hain, NAT ke peeche ke nodes nahi jud sakte.
- **Mempool disk par save nahi hoti** — node band hua to pending
  transactions gayab.
- **Block reward placeholder hai** — demand-based formula pending hai
  (Sanjay ka faisla baki). 5 BUT + halving FINAL nahi hai.
- **Swap me koi fee nahi hai** — fee ka faisla pending hai. Liquidity
  nikaalna aur LP shares nahi hain. Slippage protection / min_out nahi hai.
- **Slashing sirf equivocation par hai** — downtime par slash nahi hai.
- **Koi independent audit nahi hua hai.**
- **Web wallet ka UI real browser me test nahi hua** (sandbox me browser
  nahi hai); biometric real device par test nahi hua.
- **BUT Coin ke reserves/legal abhi pending hain** — stablecoin tabhi
  chalega jab reserves + audit + legal taiyar ho.

---

## 6. Mainnet se pehle ki checklist

Testnet se mainnet jaane se pehle ye sab hona chahiye:

- [ ] **Independent audit** — kam se kam 2 firms se, saare critical bugs fix
- [ ] **Hosted node + HTTPS** — public testnet node, domain + TLS certificate
- [ ] **API auth + rate limiting** — public API par spam rokna
- [ ] **P2P mazboot karna** — encryption, peer discovery, fork-choice/reorg
- [ ] **Testnet par 2–4 hafte ka public testing** — alag machines, Red/Blue Team
- [ ] **Tokenomics FINAL** — demand-based reward formula, baaki 59% allocation,
      team vesting schedule (docs/TOKENOMICS-DRAFT.md §8–§10)
- [ ] **BUT Coin reserves + audit + legal** — stablecoin ke liye sabse zaroori
- [ ] **Android app (APK)** — asli app, F-Droid ke liye
- [ ] **Block explorer** — transactions dekhne ka public tareeka
- [ ] **SECURITY.md dobara likhna** — real network ke hisaab se
- [ ] **Mainnet genesis alag se** — testnet genesis mainnet par kabhi nahi chalega;
      faucet wallet mainnet genesis me fund nahi hoga

---

## 7. Files

| Cheez | File |
|---|---|
| Faucet module | `chain/src/faucet.rs` |
| Faucet save/load | `chain/src/storage.rs` (`save_faucet` / `load_faucet` / `faucet_path_for`) |
| API endpoints | `chain/src/api.rs` (`POST /faucet`, `GET /faucet/status`, `load_or_create_testnet`) |
| CLI commands | `chain/src/main.rs` (`testnet`, `faucet`) |
| Ye plan | `docs/TESTNET-PLAN.md` |

**Verify (7 Nov 2026):** `cargo test` 148/148 pass (faucet 7, storage 1,
api 2 naye). `cargo run -- demo` verify PASS (10 blocks). Live API:
testnet node par Bob ko faucet se 1,000 BUT mile (mine ke baad balance
1,000,000 bits), dobara claim reject, galat address reject, normal node
par faucet band. CLI `faucet` bhi PASS (Alice ko 1,000 BUT, balance
2,000,000; dobara reject). `npx tsc --noEmit` pass, `npx vitest run`
58/58 pass.
