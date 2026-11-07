# BUT Chain — Real Blockchain Core (Rust, Phase 2)

Ye BUT Network ka **asli** blockchain core hai — web demo ka fake ledger nahi.
Foundation + P2P + PoS/BFT machinery + local API. Mainnet-grade nahi.
Poora plan: `../docs/REAL-NETWORK-PLAN.md`. Tokenomics abhi sirf
PROPOSAL hai: `../docs/TOKENOMICS-DRAFT.md`.

## Kya hai isme

| Module | Kaam |
|---|---|
| `src/wallet.rs` | Real BIP39 24-word mnemonic → Ed25519 keypair. Fixed lengths: mnemonic 24 words, private key 128 chars, address 56 chars (`but1` + 52) |
| `src/transaction.rs` | Ed25519 signature wali transaction. Bina sahi signature ke reject. Nonce replay rokta hai |
| `src/merkle.rs` | Transactions ka Merkle root |
| `src/block.rs` | Block: hash, prev hash, Merkle root, timestamp, validator signature |
| `src/mempool.rs` | Pending transactions |
| `src/consensus.rs` | Phase 2: PoS + BFT — staking (`MIN_STAKE` = 100 BUT, proposal), stake-weighted proposer chunav, Ed25519 vote, 2/3 quorum, slashing (10%) + jail |
| `src/chain.rs` | Genesis, block add (poori validation), `add_block_bft` (quorum + proposer check), balances, nonce, full chain verify |
| `src/storage.rs` | Chain disk par JSON me save/load — load karte waqt poori verify |
| `src/p2p.rs` | Phase 2: plain-TCP P2P — height handshake, block sync (verify karke add), tx gossip |
| `src/api.rs` | Phase 2: local HTTP/JSON API — `/balance/<address>`, `/chain/height`, `/tx`, `/mine` |
| `src/main.rs` | CLI: `but-node` |

## Chalana

```bash
cargo test                          # 55 tests
cargo run -- demo                   # genesis + 2 transfers + verify + save (data/chain.json)
cargo run -- new-wallet             # naya real wallet (mnemonic/private key KABHI share mat karo)
cargo run -- verify data/chain.json  # chain file verify karo
cargo run -- p2p-demo               # do nodes (ports 9001/9002) — sync karke dikhao
cargo run -- serve [port] [chain-file]   # local HTTP API (default 3030, data/chain.json)
cargo run -- node [p2p-port] [http-port] [chain-file]  # poora node: P2P + HTTP API
cargo run --example make_tx -- <amount> <fee> <nonce>  # signed test tx ka JSON (Alice -> Bob)
```

API smoke test (do terminal chahiye — ek me `serve`, doosre me curl):

```bash
cargo run -- serve 3031 data/chain.json
curl http://127.0.0.1:3031/chain/height
curl http://127.0.0.1:3031/balance/but1071e3bb9bb2e5acc7491f94c7526debeaaef09ee6126312824a0
curl -X POST http://127.0.0.1:3031/tx -d @tx.json   # tx.json: make_tx se
curl -X POST http://127.0.0.1:3031/mine
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

## Sach (honest limits)

- **P2P me encryption / peer discovery / fork-choice nahi hai** — sync
  sirf apni chain ko aage badhata hai, reorg nahi karta. Vote gossip
  abhi P2P par nahi ghoomta.
- **Slashing bookkeeping hai** — stake validator set me darj hai, coins
  chain par lock nahi hote. Asli stake-locking Phase 3 me.
- **BFT opt-in hai** — `add_block_bft` quorum maangta hai, par purana
  `add_block` abhi bhi PoA hai. Har block ke liye vote zaroori karna baaki hai.
- **API localhost-only dev API hai** — koi auth nahi, HTTPS nahi,
  `/mine` fixed dev validator se sign karta hai. Internet par mat kholo.
- **Tokenomics PROPOSAL hai** — final faisla Sanjay ka hoga.
- Koi independent audit nahi hua hai.
- Web demo abhi bhi fake hai — is core se abhi juda nahi hai (wiring
  `../docs/REAL-NETWORK-PLAN.md` me likhi hai).
