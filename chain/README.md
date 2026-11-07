# BUT Chain — Real Blockchain Core (Rust, Phase 3b)

Ye BUT Network ka **asli** blockchain core hai — web demo ka fake ledger nahi.
Foundation + P2P + BFT-only chain + stake-locking + block reward + local API
+ swap (AMM) + block-level slashing + BUT Pay registry sync.
Mainnet-grade nahi. Poora plan: `../docs/REAL-NETWORK-PLAN.md`.
Tokenomics: Sanjay ke FINAL decisions `../docs/TOKENOMICS-DRAFT.md` §1–§14 me
hain; §10 me Muse ki sifarishein (approval pending).

## Kya hai isme

| Module | Kaam |
|---|---|
| `src/wallet.rs` | Real BIP39 24-word mnemonic → Ed25519 keypair. Fixed lengths: mnemonic 24 words, private key 128 chars, address 56 chars (`but1` + 52) |
| `src/transaction.rs` | Ed25519 signature wali transaction. Bina sahi signature ke reject. Nonce replay rokta hai |
| `src/merkle.rs` | Transactions ka Merkle root |
| `src/block.rs` | Block: hash, prev hash, Merkle root, timestamp, validator signature, stake ops, **Phase 3b: swap ops + slash ops (op IDs hash me hain)**, embedded votes (votes hash me nahi hote) |
| `src/mempool.rs` | Pending transactions + pending stake ops + **Phase 3b: pending swap ops + pending slash ops** |
| `src/units.rs` | bits ↔ BUT conversion — 1 BUT = 1,000 bits, integer-only, float nahi (TS `shared/but-tx.ts` se match) |
| `src/staking.rs` | StakeLock (lock tiers 90d/180d/1y/2y/3y/5y), Unbonding (72 ghante linear gradual release), StakingState, signed StakeOp (Stake/Unstake/Claim/Register) |
| `src/swap.rs` | **Phase 3b (naya):** AMM x·y=k swap — do alag pools (BUT alag, BUT Coin alag), `quote()` (swap se pehle upfront disclosure, §11), signed SwapOp (Swap / AddLiquidity, nonce + Ed25519), §14 routing (BTC seedha BUT Coin me nahi — BTC → BUT → BUT Coin) |
| `src/slash.rs` | **Phase 3b (naya):** block-level SlashOp — equivocation saboot (do conflicting votes), order-independent evidence ID, reporter + nonce + signature, dedup evidence ID se |
| `src/consensus.rs` | PoS + BFT — minimum stake = released supply ka 1% (`min_stake_for`), stake-weighted proposer chunav, Ed25519 vote, 2/3 quorum, slashing (10%) + jail, bootstrap quorum (stake 0 ho to ek validator ka vote kaafi) |
| `src/chain.rs` | Genesis (+ team account option, **Phase 3b: BUT Coin allocations**), **BFT-only** block add (`add_block_bft`, quorum + proposer check — purana PoA `add_block` HATA DIYA GAYA), balances (sirf spendable) + **BUT Coin balances (alag map)**, locked/unbonding balances, released supply, block reward mint (proposer + 0.1% team), **Phase 3b: swap pools + applied slash ops (evidence IDs)**, full chain verify (balances + BUT Coin + staking + validators + pools + slash history + released supply + team accrued). **Purana `slash_validator` (seedha state badalne wala) HATA DIYA GAYA — slash ab sirf block-level SlashOp se** |
| `src/storage.rs` | Chain disk par JSON me save/load — load karte waqt poori verify |
| `src/p2p.rs` | Plain-TCP P2P — height handshake, block sync (BFT verify karke add, embedded votes se quorum check), tx gossip, vote gossip (AnnounceVote / GetVotes / Votes, tally per height), **Phase 3b: BUT Pay registry sync (GetRegistry / Registry, immutable-merge, conflict par reject) + `report_equivocation` (tally se SlashOp banakar mempool me)** |
| `src/api.rs` | Local HTTP/JSON API — `/balance/<address>` (bits + BUT + locked/total + **Phase 3b: BUT Coin balance**), `/chain/height`, `/tx`, `/mine` (BFT, proposer check, **Phase 3b: swap + slash ops bhi mine karta hai**), `/staking`, `/stake/<address>`, `POST /stake`, `POST /validators/register`, `/validators` me min_stake + released_supply, **Phase 3b: `GET /swap/quote/<from>/<to>/<amount>`, `GET /swap/pools`, `POST /swap`, `POST /slash`, `GET /mempool` me swap/slash ops bhi**, **testnet: `POST /faucet`, `GET /faucet/status`** |
| `src/faucet.rs` | **Testnet taiyari (naya):** testnet faucet — 1,000 BUT test coins per drip, ek address din me 1 baar, ek IP se din me 10 alag addresses, claims `faucet.json` me save hote hain. **Sirf testnet mode me chalta hai** — normal node par band. Faucet wallet testnet-only dev key (entropy `[7u8; 32]`), testnet genesis me 100,000,000 BUT fund hota hai. Test coins ki koi real value nahi. |
| `src/main.rs` | CLI: `but-node` — demo me stake + validator register + **Phase 3b: swap (liquidity + AMM) + slash demo** bhi dikhata hai; `convert <bits\|but>` bits ↔ BUT badalta hai; **Phase 3b: `swap-quote <from> <to> <amount>`, `swap <from> <to> <amount> <private-key-hex>`**; `Wallet::from_private_key_hex` (private key se wallet, sirf sign karne ke liye — mnemonic nahi hoti) |

## Chalana

```bash
cargo test                          # 138 tests
cargo run -- demo                   # genesis (BUT + BUT Coin) + transfers + BUT Pay + stake + register + swap (liquidity + AMM) + slash demo + verify + save (data/chain.json)
cargo run -- new-wallet             # naya real wallet (mnemonic/private key KABHI share mat karo)
cargo run -- verify data/chain.json  # chain file verify karo
cargo run -- p2p-demo               # do nodes (ports 9001/9002) — registry + ID-tx block bhi sync karke dikhao
cargo run -- convert 1500           # 1500 bits = 1.5 BUT
cargo run -- convert 1.5            # 1.5 BUT = 1500 bits
cargo run -- swap-quote BUT BUTCOIN 50000 [chain-file]  # swap se pehle quote (§11)
cargo run -- swap BUT BUTCOIN 1000 <private-key-hex> [chain-file]  # swap + direct mine (dev validator proposer hona chahiye)
cargo run -- serve [port] [chain-file]   # local HTTP API (default 3030, data/chain.json)
cargo run -- testnet [port] [chain-file]  # testnet node — faucet ON (free test coins)
cargo run -- faucet <address> [chain-file]  # 1,000 BUT test coins do (sirf testnet, seedha mine)
cargo run -- node [p2p-port] [http-port] [chain-file]  # poora node: P2P + HTTP API
cargo run --example make_tx -- <amount> <fee> <nonce>  # signed test tx ka JSON (Alice -> Bob)
```

API smoke test (do terminal chahiye — ek me `serve`, doosre me curl):

```bash
cargo run -- serve 3031 data/chain.json
curl http://127.0.0.1:3031/chain/height
curl http://127.0.0.1:3031/balance/but1071e3bb9bb2e5acc7491f94c7526debeaaef09ee6126312824a0
curl http://127.0.0.1:3031/staking
curl http://127.0.0.1:3031/swap/pools
curl http://127.0.0.1:3031/swap/quote/BUT/BUTCOIN/50000
curl -X POST http://127.0.0.1:3031/tx -d @tx.json   # tx.json: make_tx se
curl -X POST http://127.0.0.1:3031/swap -d @swapop.json  # signed SwapOp JSON
curl -X POST http://127.0.0.1:3031/slash -d @slashop.json  # signed SlashOp JSON
curl -X POST http://127.0.0.1:3031/mine
```

Testnet faucet (testnet node par — `cargo run -- testnet 3031 ...`):

```bash
curl -X POST http://127.0.0.1:3031/faucet -H 'Content-Type: application/json' -d '{"address": "but1..."}'
curl -X POST http://127.0.0.1:3031/mine   # coins agle block par milte hain
curl http://127.0.0.1:3031/faucet/status
```

## P2P: kyun plain TCP (libp2p kyun nahi)?

- **Std-only, koi bhaari dependency nahi** — libp2p/axum se build time,
  binary size aur attack surface badhta. Chhote network (kuch nodes,
  localhost/testnet) ke liye plain TCP + JSON kaafi hai.
- **Debuggable** — wire par sirf JSON lines hain; `nc`/`telnet` se bhi
  node se baat ho sakti hai.
- **Protocol aage bhi chalega** — messages JSON hain, isliye aage libp2p
  ya QUIC par bhi yahi format chal sakta hai; transport badlega,
  protocol nahi.
- **Limit pata hai** — plain TCP me encryption nahi, peer discovery
  nahi (peers haath se dene padte hain), NAT ke peeche ke nodes nahi jud
  sakte. Production P2P ke liye libp2p jaisa stack Phase 3+ me chahiye.

## Phase 3a me kya badla

- **Stake-locking on-chain:** stake karte hi coins lock hote hain (kharch
  nahi ho sakte — `balances` map me sirf spendable hai). Lock tiers
  90d → 180d → 1y → 2y → 3y → 5y. Unstake par 72 ghante me dheere-dheere
  release (linear), ek saath nahi. Lock khatm hone se pehle unstake reject.
- **Released-supply tracking + min-stake:** har block par reward mint
  hota hai, released supply badhti hai. Minimum stake = released supply
  ka 1% (Sanjay ka FINAL). Purana 100 BUT placeholder hata diya.
- **BFT-only chain:** purana PoA `add_block` hata diya — ab har block ko
  sahi proposer + 2/3 quorum chahiye. Votes block me embedded hote hain,
  sync karne wala node bhi quorum check karta hai.
- **Vote gossip:** validator vote bana kar peers ko bhejta hai
  (`AnnounceVote`), peer verify karke tally me rakhta hai, proposer
  votes maang sakta hai (`GetVotes` / `collect_votes_for_block`).
- **Block reward + 0.1% team share:** har block par reward mint hota hai
  (proposer ko 99.9%, team ko 0.1%). Team account genesis me ek baar set
  hota hai; set na ho to team share `team_accrued` me jama hota hai.
  **Reward amount PLACEHOLDER hai** (5 BUT/block, ~4 saal me halving) —
  Sanjay ka FINAL demand-based formula aate hi badlega.
- **Display conversion:** bits ↔ BUT har jagah (API, CLI `convert`,
  demo output). 1 BUT = 1,000 bits.
- **Purani chain files nahi chalengi:** block hash me ab stake op IDs
  bhi hain — purani `data/chain.json` delete karke `demo` dobara chalao.

## Phase 3b me kya badla

- **Swap (AMM x·y=k):** do alag pools — BUT ka pool aur BUT Coin ka pool
  kabhi mix nahi hote (§13). Swap sirf available liquidity tak hota hai
  (§11): `GET /swap/quote` swap se PEHLE batata hai kitna milega aur
  pool me kitna available hai; user phir bhi poora swap kare to
  transaction FAIL nahi hoti — saare input coins pool me jate hain,
  sirf available output milta hai, koi IOU/pending balance nahi.
  Routing (§14): BTC (ya koi bahar ka coin) seedha BUT Coin me nahi —
  rasta hamesha BTC → BUT → BUT Coin. Signed SwapOp (Swap/AddLiquidity,
  nonce + Ed25519). **Purani chain files nahi chalengi:** block hash me
  ab swap/slash op IDs bhi hain — purani `data/chain.json` delete karke
  `demo` dobara chalao.
- **Block-level slashing:** purana `slash_validator` (seedha state
  badalne wala) HATA DIYA GAYA. Ab slash sirf block me darj hone wale
  signed SlashOp se hota hai — saboot = do conflicting votes
  (equivocation). Ek saboot par ek hi baar slash (dedup evidence ID se).
  P2P par equivocation dekha jaye to `Node::report_equivocation`
  mempool me SlashOp dalta hai. Slash ke baad bhi `verify_chain` PASS
  hota hai (genesis check bhi fix hua — baad me jail hue validator ka
  purana block ab galat nahi mana jata).
- **BUT Pay registry P2P sync:** sync pehle registry karta hai, phir
  blocks — ID/handle wale txs wale blocks bhi sync hote hain.
  Merge immutable hai (entry kabhi nahi badalti); conflict ho to poora
  merge reject (manual hal).
- **BUT Coin balances:** har address ka BUT Coin balance alag map me
  hai; API `/balance` dono dikhata hai; genesis me BUT Coin allocations
  bhi ho sakti hain (demo me Alice ko 1,000 BUT Coin).
- **CLI:** `swap-quote` aur `swap` (direct-mine, dev validator
  proposer hona chahiye — warna saaf error + `POST /swap` ka rasta).
  `Wallet::from_private_key_hex` (private key se wallet — mnemonic
  nahi hoti, sirf sign karne ke liye).
- **Demo:** ab 10 blocks — swap (liquidity + AMM + quote) aur slash
  demo bhi; `p2p-demo` me registry + ID-tx block sync bhi.

## Sach (honest limits)

- **Block reward placeholder hai** — demand-based formula abhi pending
  hai (Sanjay ka faisla baki). 5 BUT + halving sirf mechanism dikhane
  ke liye hai, FINAL nahi.
- **Swap me abhi koi fee nahi hai** — fee ka faisla pending hai. **Liquidity
  nikaalna (remove) aur LP shares abhi nahi hain** (Phase 3c/testnet).
  **Slippage protection / min_out nahi hai** — §11 ke no-fail rule ke
  hisaab se output 0 bhi ho sakta hai; user ko quote pehle dekhna chahiye.
  **Koi price oracle nahi hai** — pool ratio hi price hai.
- **Slashing sirf equivocation par hai** — downtime (validator ka lambe
  time tak offline rehna) ya galat block propose karne par slash abhi
  nahi hai. **Reporter ko koi inaam (bounty) nahi milta.**
- **Registry merge conflict ka hal manual hai** — do nodes ne ek hi ID
  do alag logo ko de di ho to merge reject hota hai; kaunsi chain sahi
  hai, ye haath se dekhna padega. Registry entries on-chain nahi hain.
- **P2P me encryption / peer discovery / fork-choice nahi hai** — sync
  sirf apni chain ko aage badhata hai, reorg nahi karta.
- **API localhost-only dev API hai** — koi auth nahi, HTTPS nahi,
  `/mine` fixed dev validator se sign karta hai (sirf tab jab dev
  validator us height ka proposer ho). Internet par mat kholo.
- **Mempool disk par save nahi hoti** — node band hua to pending
  transactions/stake/swap/slash ops gayab.
- **Tokenomics ke kuch faisle abhi pending hain** — demand-based reward
  formula, baaki 59% allocation, team vesting schedule, BUT Coin
  reserves/legal (dekho `../docs/TOKENOMICS-DRAFT.md`).
- **Testnet taiyari ho gayi hai (local)** — faucet + plan (`../docs/TESTNET-PLAN.md`).
  **Asli testnet launch abhi baaki hai** — iske liye hosted node + HTTPS
  chahiye. **Koi independent audit nahi hua hai** — audit ke liye bahar
  ki audit firm chahiye.
- Web wallet v1 real API se juda hai (scope `../docs/WALLET-V1-SCOPE.md`
  me) — par abhi bhi koi hosted node/HTTPS nahi hai. Swap/slash abhi
  web wallet me nahi jude hain (sirf Rust API/CLI me hain).
