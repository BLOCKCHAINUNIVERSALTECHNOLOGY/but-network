# BUT Network — Demo se Real banane ka plan

> Sanjay ne saaf kaha hai: **ye testing nahi hai, ye seekhne ke liye network nahi hai — ye REAL network banega.**
> Is document me imaandari se likha hai: kya fake tha, is phase me kya real hua, aur kya abhi bhi baaki hai.
> Demo word sirf rename karke nahi hataya gaya — jo fake hai use real se badla gaya hai, ya saaf likha hai ki abhi baaki hai.
>
> Date: 7 November 2026. Repo commit: `49a4af7` (is phase me koi commit/push nahi hua — Sanjay ki approval ke baad hi push hoga).

---

## 1. Audit — repo me kya-kya fake/demo mila (7 Nov 2026)

`grep -rniE "demo|fake"` se **166 matches, 19 files** me mile. Poori list `/tmp/demo-audit.txt` me save hai (ye repo ke bahar sandbox file hai).

### Sabse badi fake cheezein (ye sabse pehle badalni hain)

| # | Kya fake hai | Kahan hai | Kyun fake hai |
|---|---|---|---|
| 1 | **Fake balance 4280.5** — koi real coin nahi, number screen par likha hai | `client/src/pages/Home.tsx` (line ~50) | Balance kisi chain se nahi aata |
| 2 | **Fake transactions** `demo-001`, `demo-002`, `demo-003` | `Home.tsx` (lines 59–79) | Transactions haath se likhi hain, chain par kabhi nahi gayi |
| 3 | **Fake receive address** `BUT-DEMO-7J2K-Q9LX` | `Home.tsx` (line 49) | Asli `but1...` address nahi hai |
| 4 | **localStorage ledger** — balance/transactions browser me save hote hain | `Home.tsx` (`but-demo-balance-v1`, `but-demo-transactions-v1`, `but-demo-ledger-enc-v1`) | Browser ka storage blockchain nahi hai; koi bhi padh/badala sakta hai |
| 5 | **Demo password** (12 akshar) — sirf screen lock hai | `Home.tsx` (lines 46–48, 232–234) | Ye wallet ki asli security nahi hai |
| 6 | **Demo keypair / demo wallet** — mnemonic aur keypair alag-alag bante hain | `shared/crypto-secure.ts` (`generateDemoKeypair`), `shared/wallet-secure.ts` (`generateWallet`) | Real wallet me mnemonic SE HI keypair banta hai; yahan mnemonic sirf dikhane ke liye hai |
| 7 | **"BUT Wallet Demo" title** | `README.md`, `client/index.html`, `client/public/manus-routes.json` | Poora app demo ke roop me pehchana jata hai |
| 8 | **"Demo network" pill + "TEST DATA" chip + demo modal** | `Home.tsx` (lines 466, 575, 718) | UI khud kehta hai ye demo hai |
| 9 | **Demo transaction hash** `0x...demo` | `Home.tsx` (line 355) | Hash haath se banaya gaya hai, kisi block se nahi aaya |
| 10 | **DEMO_MAP_ID** (Google Maps demo key) | `client/src/components/Map.tsx` (line 139) | Asli map key nahi hai |
| 11 | **ComponentShowcase demo AI chat** — nakli jawab dikhata hai | `client/src/pages/ComponentShowcase.tsx` | Real AI call nahi hai |
| 12 | **Android skeleton "DEMO ONLY"** — kabhi build nahi hua | `android/` (`MainActivity.kt`, `WalletCrypto.kt`, `README.md`) | Sirf code ka dhancha hai, APK nahi bana |

### Baaki files jinme "demo" likha hai (kam important, par list poori hai)

- `SECURITY.md` (24 matches) — poori policy "demo" ke hisaab se likhi hai; real network aane par dobara likhni hogi.
- `shared/crypto-secure.ts` (25 matches) — "DEMO ONLY" module; encryption ke tareeke sahi hain, par demo data ke liye hai.
- `shared/wallet-secure.ts` (16 matches) — "DEMO ONLY"; BIP39-style mnemonic hai, ab real BIP39 Rust core me hai (neeche dekho).
- `server/crypto-secure.test.ts` (17 matches) aur `server/wallet-secure.test.ts` (3 matches) — demo module ke tests.
- `docs/FDROID-CHECKLIST.md` (8 matches) — khud kehta hai "ye repo abhi web demo hai".
- `docs/DEVICE-LOCK-DESIGN.md` (1 match) — design hai, bani hui cheez nahi.
- `server/platform.test.ts` (3 matches) — `fakeDocument` test ka naam hai, koi fake data nahi.
- `client/src/components/ui/input-otp.tsx` (2 matches) — `hasFakeCaret` UI library ka naam hai, koi fake data nahi.

**Saaf kar den:** `input-otp.tsx` aur `platform.test.ts` wale "fake" sirf naam hain — unme koi nakli data nahi hai. Baaki sab real fake hain.

---

## 2. Is phase me kya REAL hua (7 Nov 2026)

Rust me asli blockchain core bana hai — `chain/` directory me. Ye foundation hai, mainnet-grade nahi.

| Cheez | File | Kya real hai |
|---|---|---|
| Real Wallet | `chain/src/wallet.rs` | Vetted `bip39` crate se **asli 24-word BIP39** mnemonic; mnemonic SE HI Ed25519 keypair banta hai (TS demo me mnemonic sirf dikhawa tha); fixed lengths — mnemonic 24 words, private key 128 chars, address 56 chars (`but1` + 52); BIP39 test vector "abandon"×23 + "art" pass hota hai |
| Real Transaction | `chain/src/transaction.rs` | Sender/receiver/amount/fee/nonce; **Ed25519 signature**; bina sahi signature ke transaction **reject**; amount badla to signature fail; public key transaction me hi hoti hai (koi bhi node verify kar sakta hai) |
| Real Block | `chain/src/block.rs` | Hash, prev hash, Merkle root, timestamp, validator signature; ek bhi field badli to hash/signature fail |
| Real Chain | `chain/src/chain.rs` | Genesis block, block add (poori validation — index, prev hash, timestamp, validator, har tx ki signature/nonce/balance), **chain verify** (genesis se rebuild karke check), account balances, nonce (replay attack rokta hai), fee validator ko |
| Real Storage | `chain/src/storage.rs` | Chain **disk par JSON file** me save hoti hai; load karte waqt poori verify hoti hai — file badli mili to load hi nahi hoga |
| Mempool | `chain/src/mempool.rs` | Pending transactions ki list; bina signature ke tx andar nahi aati; duplicate reject |
| Consensus (Phase 1) | `chain/src/consensus.rs` | **Proof-of-Authority (PoA)** — ek authorized validator. Kyun? Strategy me PoS+BFT recommend tha, par Phase 1 me ek node par PoS ka koi matlab nahi (stake karne wala koi nahi). PoA simple aur sahi hai: galat validator ka block reject hota hai. **PoS+BFT Phase 2 me aa gaya** (§3 dekho — staking, proposer, vote, quorum, slashing; BFT block-add opt-in hai) |
| Merkle Tree | `chain/src/merkle.rs` | Transactions ka Merkle root |
| CLI | `chain/src/main.rs` | `but-node new-wallet` (naya real wallet), `but-node demo` (genesis + 2 transfers + verify + save/load), `but-node verify <file>` |
| Tests | `chain/src/*.rs` (har module me) | **`cargo test`: 34/34 pass** — block validation, galat signature reject, tampered block reject, chain verify, double-spend reject, replay attack reject, BIP39 vector, fixed-length rejection, tampered file load reject |

### Demo run (asli output, 7 Nov 2026)

```
Validator: but1f93bcb5032a46d5de8d0f6e234a2aa349f508239345c393086cd
Alice    : but1071e3bb9bb2e5acc7491f94c7526debeaaef09ee6126312824a0
Bob      : but1af50f1cae2556c6a78024ca7b2113fc9facf59318c78303a1037

Genesis block: bf35f7042d73f3c2772ba517ccc22bb917ff66b40894605a121484711caa3bea
Block 1: 99d844531cd0b2f3ab738bac699af96639b9f75f194002aa939487f5d29f152f (Alice -> Bob 100, fee 1)
Block 2: 05788362edf65b09bd11e3d0c55d291e0342fbb2073107bc44110c55633555e6 (Bob -> Alice 25)

Verify: PASS (3 blocks)
Balances — Alice: 999924, Bob: 75, Validator (fees): 1
Chain disk par save hui: data/chain.json
Reload + verify: PASS (3 blocks)
```

### Kaise chalaye

```bash
cd chain
cargo test            # 34 tests
cargo run -- demo     # chhoti chain banao + verify + save
cargo run -- new-wallet   # naya real wallet (mnemonic/private key KABHI share mat karo)
cargo run -- verify data/chain.json
```

### Web demo ke tests bhi pass hain

`npx tsc --noEmit` (exit 0) aur `npx vitest run` (**33/33 pass**) — web demo toota nahi hai.

---

## 3. Phase 2 me kya REAL hua (7 Nov 2026)

Is phase me koi commit/push nahi hua — Sanjay ki approval ke baad hi push hoga.

| Cheez | File | Kya real hai |
|---|---|---|
| P2P network | `chain/src/p2p.rs` | **Plain-TCP P2P (std-only, koi bhaari dependency nahi)**. Node doosre node se height/hash poochh sakta hai (`Hello`/`Height`), missing blocks maang kar sync kar sakta hai (`GetBlocks`/`Blocks` — har block `Chain::add_block` se poori tarah verify hokar hi judta hai), aur pending transactions bhej sakta hai (`AnnounceTx` — mempool me jaane se pehle signature verify hoti hai). **Test: 2 nodes, alag ports** — `cargo test` me Node A (genesis + 2 blocks) se Node B (sirf genesis) sync karta hai, height 1 → 3, balances match, verify PASS. CLI demo: `but-node p2p-demo` |
| Consensus PoS+BFT | `chain/src/consensus.rs`, `chain/src/chain.rs` (`add_block_bft`) | **Validator set + staking** — validator banne ke liye `MIN_STAKE` (proposal: 100 BUT) lock; kam stake reject. **Proposer chunav** — stake-weighted deterministic (round ka hash → stake ke hisaab se chunav). **BFT vote** — Ed25519 sign wala `Vote`, `VoteTally` me 2/3 quorum (approvals ka stake kul active stake ke 2/3 se zyada). **Slashing** — equivocation (ek height par do blocks ko vote) pakda jata hai, stake ka 10% kat ta hai, stake `MIN_STAKE` se neeche gira to jail; `restake` se unjail. **`Chain::add_block_bft`** — block tabhi judta hai jab proposer sahi ho aur quorum poora ho (tests: quorum ke saath judta hai, 2/4 votes par reject, galat proposer reject) |
| Local HTTP/JSON API | `chain/src/api.rs`, `chain/src/main.rs` (`serve` / `node`) | **`but-node serve`** localhost par HTTP API chalata hai (std-only). Endpoints: `GET /`, `/chain/height`, `/chain`, `/block/<n>`, `/balance/<address>` (balance **chain se** aata hai), `/mempool`, `/validators`, `POST /tx` (signature verify hokar mempool me), `POST /mine` (mempool se block banao + chain disk par save). **`but-node node`** P2P + HTTP API ek saath chalata hai. Live test kiya gaya: tx submit → mine → Bob ka balance 0 → 100 (chain se, fake nahi) |
| Tokenomics draft | `docs/TOKENOMICS-DRAFT.md` | **PROPOSAL ONLY** — total supply 1 billion BUT (proposal), 1 BUT = 1,000,000 anu (proposal), fee model, validator rewards (5 BUT/block, halving — proposal), staking rules. Har jagah "proposal" likha hai — final faisla Sanjay ka hoga |
| Tests | `chain/src/*.rs` | **`cargo test`: 55/55 pass** (Phase 1 ke 34 + Phase 2 ke 21 naye: consensus 8, BFT chain 3, P2P 6, API 4). CLI demo + p2p-demo + live API smoke test — sab chale |

### Web wallet ko API se jodne ki wiring (Phase 3 — Wallet v1 me poori hui, 7 Nov 2026)

**UPDATE 7 Nov 2026 — Wallet v1 ban gaya** (docs/WALLET-V1-SCOPE.md dekho).
Neeche ki wiring ab poori hai — web wallet ab Rust API se juda hua real
wallet hai, demo nahi:

1. **API base URL**: `shared/but-node.ts` me `VITE_BUT_NODE_URL` (default
   `http://127.0.0.1:3030`). Rust API me CORS headers bhi jode gaye
   (`chain/src/api.rs` — OPTIONS preflight + `Access-Control-Allow-Origin: *`,
   sirf local dev ke liye).
2. **Balance**: `GET /balance/<address>` se aata hai — fake `4280.5` hata
   diya gaya. Address real wallet (`but1...`) se aata hai.
3. **Transaction bhejhna**: browser me sign hoti hai (tweetnacl Ed25519,
   `shared/but-tx.ts` — Rust core ke saath same payload + signature format;
   live E2E se verify hua), phir `POST /tx` par jati hai. Private key kabhi
   server ko nahi bheji jati. Mnemonic se keypair banta hai — wahi derivation
   jo Rust core me hai (`shared/wallet-secure.ts` — purani demo galti theek hui).
4. **History**: `GET /chain` se padhi jati hai — `demo-001/002/003` hata diye gaye.
5. **Mine / confirm**: transaction submit hone ke baad UI khud `POST /mine`
   chalata hai (dev node), phir balance/history refresh karke confirmation
   dikhata hai. Production me validator network khud mine karega.
6. **localStorage ledger hataya**: balance/transactions browser storage se
   nahi, chain se aate hain. Browser me sirf encrypted vault (AES-GCM)
   rehta hai — purane `but-demo-*` keys pehli baar me delete ho jate hain.
7. **Production API**: ye API localhost-only hai — asli users ke liye hosted
   node + HTTPS + auth chahiye hoga, jo abhi nahi hai.

### Phase 2 me kya abhi bhi SIMPLIFIED hai (imaandari se)

- **P2P**: no encryption, no peer discovery (peers haath se dene padte hain), no fork-choice/reorg (diverged chain apnayi nahi jati, sync reject hota hai), no vote gossip, no rate limiting (spam possible), ek connection = ek request.
- **Slashing bookkeeping hai**: stake validator set me darj hai, par coins chain-state me lock nahi hote — staked coins abhi bhi kharch kiye ja sakte hain. Asli stake-locking + unbonding Phase 3 me.
- **BFT opt-in hai**: `add_block_bft` quorum maangta hai, par purana `add_block` abhi bhi PoA hai. Har block ke liye vote zaroori karna (BFT-only chain) baaki hai.
- **Single-machine BFT test**: vote/quorum ek hi machine par test hue hain — alag machines, network delay, byzantine node abhi test nahi hue.
- **API dev-grade hai**: localhost-only, koi auth nahi, HTTPS nahi, `/mine` fixed dev validator se sign karta hai, mempool disk par save nahi hoti.
- **Tokenomics proposal hai** — koi bhi number chain me hardcode nahi hua.
- **Block reward nahi hai** — validator ko sirf fees milti hain.

## 4. Phase 3a me kya REAL hua (7 Nov 2026)

Sanjay ne kaha "Okay start" — Phase 3 shuru. Phase 3a me Rust core ke
5 kaam hue (koi commit/push nahi — local hi hai):

| Kaam | File | Kya hua |
|---|---|---|
| Stake-locking (on-chain) | `chain/src/staking.rs` (naya), `chain/src/chain.rs` | Stake karte hi coins lock hote hain — kharch nahi ho sakte (`balances` map me sirf spendable hai). Lock tiers: 90 din → 180 din → 1 saal → 2 saal → 3 saal → 5 saal (Sanjay ke FINAL, doc §6). Unstake par release 72 ghante me dheere-dheere (linear), ek saath nahi. Lock khatm hone se pehle unstake reject |
| Released-supply + min-stake | `chain/src/consensus.rs`, `chain/src/chain.rs` | Chain released supply track karti hai (genesis + har block ka reward). Minimum stake = released supply ka 1% (Sanjay ka FINAL). Purana 100 BUT placeholder hata diya. Min stake har block ke saath badhta hai (jaise released 1,025,000 → min 10,250) |
| BFT-only chain | `chain/src/chain.rs` | Purana PoA `add_block` **hata diya** — ab sirf `add_block_bft` hai: sahi proposer + 2/3 quorum zaroori. Bina votes wala block reject hota hai |
| Vote gossip | `chain/src/p2p.rs` | Naye messages `AnnounceVote` / `GetVotes` / `Votes`. Validator vote bana kar peers ko bhejta hai; peer verify karke apni tally me rakhta hai; proposer votes maang kar quorum bana sakta hai (`collect_votes_for_block`). Votes block me embedded hote hain — sync karne wala node bhi quorum check karta hai |
| Block reward + 0.1% team share | `chain/src/consensus.rs`, `chain/src/chain.rs` | Har block par reward mint hota hai: proposer ko 99.9%, team ko 0.1% (Sanjay ka FINAL). Team account genesis me ek baar set hota hai; set na ho to team share `team_accrued` me jama hota hai. **Reward amount PLACEHOLDER hai** — 5 BUT/block + ~4 saal me halving, sirf mechanism ke liye. Sanjay ka FINAL demand-based formula aate hi badlega |
| Display conversion bits ↔ BUT | `chain/src/units.rs` (naya), `chain/src/api.rs`, `chain/src/main.rs` | 1 BUT = 1,000 bits, integer-only (float nahi). API har balance bits + BUT dono me deti hai; CLI `but-node convert 1500` → `1.5 BUT` |

**Verify (7 Nov 2026):** `cargo test` **104/104 pass** (Phase 2 ke 55 + Phase 3a ke 49 naye: staking 13, units 7, chain 13, p2p 4, api 3, error 1, consensus 8). `cargo run -- demo` verify PASS (6 blocks: transfers + BUT Pay + stake + validator register, reload bhi PASS). `cargo run -- p2p-demo` sync PASS (Node B height 1→3, verify PASS). Live API: `/staking`, `/stake/<addr>`, `/validators` (min stake + released supply), `/balance` (bits + BUT + locked/total) — sab curl se check hue. `npx tsc --noEmit` pass, `npx vitest run` 58/58 pass (web code nahi toota).

**Phase 3a me ek asli bug bhi mila aur theek hua:** `validate_block_against` block ke validator ko `self.validators` (poori chain ki final state) se check karta tha — verify-rebuild me jo validator baad me jail hua ho, uska purana block galat fail hota tha. Ab block se pehle wali state (`validators` parameter) se check hota hai.

**Purani chain files nahi chalengi:** block hash me ab stake op IDs bhi hain — purani `data/chain.json` delete karke `demo` dobara chalao.

### Phase 3a ke baad bhi SIMPLIFIED hai (imaandari se)

- **Block reward placeholder hai** — demand-based formula pending hai (Sanjay ka faisla baki). 5 BUT + halving FINAL nahi hai.
- **Slashing abhi bhi block-level op nahi hai** — `slash_validator` seedha state badalta hai (sirf tests/admin), uske baad verify fail karega jab tak slash block me darj na ho. Equivocation P2P par sirf log hoti hai, us par slash nahi hota.
- **BUT Pay registry P2P sync me nahi hai** — sync karte waqt registry khaali hoti hai, ID/handle wale txs wale blocks sync nahi honge.
- **P2P**: no encryption, no peer discovery, no fork-choice/reorg, no rate limiting — ye sab abhi bhi baaki hai.
- **API dev-grade hai** — localhost-only, koi auth nahi, HTTPS nahi, mempool disk par save nahi hoti.

## 5. Har demo cheez kab real se replace hogi (replacement plan)

| Demo cheez | Kab replace hogi | Kaise |
|---|---|---|
| Fake balance 4280.5 | **Ho gaya — Wallet v1 (7 Nov 2026)** | Balance Rust chain se aata hai (API), localStorage se nahi |
| Fake transactions demo-001/002/003 | **Ho gaya — Wallet v1** | Transactions asli chain par jati hain, sign hokar; history chain se padhi jati hai |
| Fake address `BUT-DEMO-7J2K-Q9LX` | **Ho gaya — Wallet v1** | Real wallet se `but1...` address (Rust core jaisa derivation) |
| localStorage ledger | **Ho gaya — Wallet v1** | Ledger chain par hai; browser me sirf encrypted wallet vault rehta hai, ledger nahi; purane `but-demo-*` keys delete hote hain |
| Demo password / lock screen | **Ho gaya — Wallet v1 (web)** | Asli wallet unlock — password (12+ akshar, AES-GCM vault) + WebAuthn biometric (jahan support ho) + 5-min auto-lock. Hardware-backed (Android Keystore) abhi bhi Android app me baaki hai |
| Demo keypair / demo wallet (TS) | **Ho gaya — Wallet v1** | TS wallet ab mnemonic se keypair banata hai (Rust core jaisa derivation, tweetnacl signing) — purani demo galti theek hui |
| "BUT Wallet Demo" title | **Ho gaya — Wallet v1** | Title ab "BUT Wallet" hai (index.html, manus-routes.json, README.md) |
| Android skeleton | **Phase 4** | JDK 17 + Android SDK install karke asli app build karni hogi (~1.5–2 GB download, 30–60 min); RAM dhyaan rakhe (Gradle ko 2 GB free chahiye) |
| SECURITY.md "demo" policy | **Phase 3** | Real network aane par poori policy dobara likhni hogi |
| `shared/crypto-secure.ts` / `wallet-secure.ts` "DEMO ONLY" | **Phase 2–3** | Rust core ke saath judne par in modules ko "real" module se replace karo; purane demo module delete karo |

**Is phase me web demo ko chheda nahi gaya** — ye jaan-boojh kar hai. Web demo todne se pehle Rust core ka foundation poora hona chahiye tha. Ab foundation taiyar hai, isliye Phase 3 me web demo ko core se jodna safe hoga.

---

## 6. Kya abhi bhi REAL NAHI hai (imaandari se list)

1. **P2P basic hai, production-grade nahi** — nodes judte hain, sync karte hain aur votes gossip karte hain, par encryption, peer discovery, fork-choice/reorg aur rate limiting nahi hain.
2. **Slashing abhi bhi block-level op nahi hai** — stake-locking, BFT-only chain aur vote gossip Phase 3a me ho gaye, par slashing abhi seedha state badalti hai (verify toot jata hai) aur equivocation par slash nahi hota.
3. **Block reward placeholder hai** — demand-based formula pending hai (Sanjay ka faisla baki). 5 BUT + halving FINAL nahi.
4. **Web wallet v1 ban gaya hai (7 Nov 2026)** — fake balance, fake transactions, localStorage ledger sab hata diye gaye; UI ab Rust API se juda hai (wiring §3 dekho). Par web wallet abhi bhi **production-grade nahi** — local node par chalta hai, koi audit nahi, biometric real device par test nahi hua, aur UI ko real browser me chalana baaki hai.
5. **Koi independent security audit nahi hua** — code abhi bina audit ke hai. Asli paisa tabhi aayega jab audit ho.
6. **Tokenomics ke kuch faisle abhi pending hain** — demand-based reward formula, baaki 59% allocation, team vesting schedule, BUT Coin reserves/legal (docs/TOKENOMICS-DRAFT.md §8–§10 dekho). Jo FINAL hai wo chain me aa gaya hai (bits, 1B cap, 1% min stake, 0.1% team share, lock tiers).
7. **Swap feature nahi bana** — liquidity rule (§11) sirf doc me hai, AMM/swap code abhi nahi hai.
8. **Android app nahi bani** — skeleton hai, APK nahi. F-Droid tabhi possible hai jab asli app ho.
9. **Block explorer nahi hai** — transactions dekhne ka koi public tareeka nahi.
10. **Multi-chain (2–3 chains) abhi nahi hai** — pehle ek chain mazboot hogi, tabhi doosri.
11. **Device Lock / spend key abhi design hai** — code nahi bana.
12. **Ye core mainnet-grade nahi hai** — foundation hai. Production se pehle P2P, PoS+BFT, audit, aur testnet chahiye.

---

## 7. Agla step (Phase 3b)

Phase 3a ho gaya (stake-locking, BFT-only, vote gossip, reward plumbing, bits↔BUT). Ab:

1. **Swap feature** — §11 liquidity rule ke saath AMM/swap (BUT ↔ BUT Coin, alag-alag liquidity, Bitcoin → BUT → BUT Coin routing).
2. **Slashing block-level op** — slash aur equivocation block me darj hon, verify na toote.
3. **BUT Pay registry P2P sync** — ID/handle wale blocks bhi sync hon.
4. **P2P mazboot karna** — encryption, peer discovery, fork-choice/reorg, rate limiting.
5. **Tokenomics faisle** — demand-based reward formula, baaki 59% allocation, team vesting schedule, BUT Coin reserves/legal (docs/TOKENOMICS-DRAFT.md §8–§10).
6. **Testnet** — kai machines par nodes, Red Team / Blue Team testing.
7. **Independent audit** — production se pehle zaroori.

## 8. Testnet taiyari (faucet + plan) — 7 Nov 2026

**Testnet abhi launch nahi hua hai** — ye sirf taiyari hai (local only).
Poora plan: `docs/TESTNET-PLAN.md`.

| Kaam | File | Kya hua |
|---|---|---|
| Faucet module | `chain/src/faucet.rs` (naya) | 1,000 BUT test coins per drip, ek address din me 1 baar, ek IP se din me 10 alag addresses, claims `faucet.json` me save hote hain (restart se reset nahi hote). **Sirf testnet mode me chalta hai** — normal node par band. Faucet seedha balance nahi badalta — wo asli signed transaction bhejta hai, coins agle block mine hone par milte hain |
| Faucet wallet | `chain/src/faucet.rs` | Testnet-only dev key (entropy `[7u8; 32]`), testnet genesis me 100,000,000 BUT fund hota hai. Test coins ki koi real value nahi |
| API endpoints | `chain/src/api.rs` | `POST /faucet`, `GET /faucet/status`, `ApiState::load_or_create_testnet` (testnet genesis + faucet ON) |
| CLI commands | `chain/src/main.rs` | `but-node testnet [port] [chain-file]` (faucet ON node), `but-node faucet <address> [chain-file]` (seedha mine karke drip) |
| Testnet plan doc | `docs/TESTNET-PLAN.md` (naya) | Testnet kya hai, node kaise chalaye, faucet rules, test checklist, known limits, mainnet checklist |

**Verify (7 Nov 2026):** `cargo test` **148/148 pass** (faucet 7, storage 1,
api 2 naye). `cargo run -- demo` verify PASS (10 blocks). Live API: testnet
node par Bob ko faucet se 1,000 BUT mile (mine ke baad balance 1,000,000
bits), dobara claim reject, galat address reject, normal node par faucet
band. CLI `faucet` bhi PASS (Alice ko 1,000 BUT, balance 2,000,000; dobara
reject). `npx tsc --noEmit` pass, `npx vitest run` 58/58 pass.

**Abhi bhi baaki hai (testnet launch ke liye):** hosted node + HTTPS,
API auth + rate limiting, P2P mazboot karna, alag machines par testing,
Red Team / Blue Team testing, independent audit, tokenomics FINAL faisle,
BUT Coin reserves/legal, Android APK, block explorer.

---

*Ye document jhooth nahi bolta. Jo real hai, wo real likha hai. Jo baaki hai, wo baaki likha hai.*
