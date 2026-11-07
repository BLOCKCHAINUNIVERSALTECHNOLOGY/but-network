# Ek Successful Chain Banane Ke Liye Kya-Kya Chahiye — Checklist + Benchmarks

> **Sanjay ne kaha (7 Nov 2026):** "Main tukke maar raha hun, khud confident nahi hun — tum online dekh kar batao ki ek successful chain ke liye kya-kya chahiye aur kaunsa number sabse best rahega."
>
> Ye document usi research ka natija hai: 10 chains (Bitcoin, Ethereum, Solana, BNB Chain, Polygon, Cosmos, Avalanche, NEAR, Sui, Aptos) + fail hui chains (Terra/Luna, EOS, ICP) se benchmarks nikaale gaye hain, aur Sanjay ke faislon ko unse milaya gaya hai.
>
> **Ye sifarish hai, faisla nahi.** Sanjay ke FINAL faisle docs/TOKENOMICS-DRAFT.md §8 me wahi hain — ye document unhe overwrite nahi karta. Har sifarish ke saath reason likha hai; aakhiri faisla hamesha Sanjay ka rahega.
>
> Date: 7 November 2026. Numbers us waqt ke online sources se hain; waqt ke saath badal sakte hain.

---

## 1. Benchmark table — 10 chains ek nazar me

| Chain | Decimals | Max supply | Team / insiders | Min stake (validator) | Unbonding | Fee model | Inflation / rewards |
|---|---|---|---|---|---|---|---|
| **Bitcoin** | 8 (satoshi) | 21M (hard cap) | 0% — fair launch | n/a (mining) | n/a | Free market, miner ko | ~0.8%/yr, 2140 tak 0 |
| **Ethereum** | 18 (wei) | Koi cap nahi | 16.7% (genesis) | 32 ETH | Queue + ~27h + sweep (din–hafte) | EIP-1559: base fee burn + tip | ~0% net (burn vs issuance) |
| **Solana** | 9 (lamport) | Koi cap nahi | ~48% (genesis, high) | ~1 SOL | ~2–3 din | Base fee 50% burn; priority fee 100% validator | ~3.7%/yr → 1.5% floor |
| **BNB Chain** | 18 | 200M → 100M target | Binance ecosystem | 2,000 BNB | 7 din | BEP-95: hissa burn | 0% (deflationary) |
| **Polygon (POL)** | 18 | 10B cap hataya; +2%/yr | Team 16% | ~10k–100k POL | ~3–4 din | EIP-1559 burn (~0.27%/yr) | +2%/yr net |
| **Cosmos (ATOM)** | 6 (uatom) | Koi cap nahi | Tendermint 10% + Foundation 10% | 1 ATOM self-bond | **21 din** | Koi burn nahi | 7–10%/yr |
| **Avalanche** | 9 (nAVAX) | 720M (hard cap) | Team 10% | 2,000 AVAX | Lock 2 hafte–1 saal (koi unbonding nahi) | **100% fees burn** | Mint cap ki taraf slow |
| **NEAR** | 24 (yocto) | 1B, koi cap nahi | Core contributors 8.5% | Seat ~25,500 NEAR | ~36–48 ghante | 70% burn, 30% developer ko | 2.5%/yr (Oct 2025 me 5% se aadha) |
| **Sui** | 9 (MIST) | 10B (hard cap) | Contributors 20% + Investors 14% | 1 SUI delegate | **~24 ghante** | Koi burn nahi; storage fee Storage Fund me | 0% inflation; fixed subsidy pool |
| **Aptos** | 8 (octas) | 2.1B cap (Mar 2026 me lagaya) | Contributors 19% + Foundation 16.5% + Investors 13.5% | 1M APT pool | ~14–30 din | Fees burn / buyback | ~2.6% staking return |

**Is table se 5 baatein saaf dikhti hain:**
1. **Decimals kam se kam 6 hain** — sabse kam Cosmos (6), aam taur par 8–9, EVM chains 18.
2. **Unbonding sabse lamba 21 din hai (Cosmos)** — baaki sab 1–7 din. Koi bhi successful chain 90 din ka unbonding nahi rakhti.
3. **Minimum stake chhota hota hai** — delegator ke liye 1 coin ya usse bhi kam (Sui 1 SUI, Solana ~1 SOL, Cosmos koi minimum nahi).
4. **Cap todna ya hatana mehenga padta hai** — Polygon ne cap hataya, POL ATH se ~97% gira; Aptos ko 2026 me cap lagana pada.
5. **Team ka hissa 0–20% ke beech hai** — Bitcoin 0%, Avalanche 10%, Solana ~48% (zyada mana jata hai).

---

## 2. Tech checklist — chain kaise banti hai

| # | Cheez | Benchmark (successful chains) | BUT ki haalat |
|---|---|---|---|
| 1 | Consensus + finality | PoS + BFT, finality second–minute me (Solana ~150ms target, Avalanche sub-second, Cosmos ~6s) | ✅ PoS+BFT Phase 2 me; single-machine test only |
| 2 | P2P network | Nodes khud sync karein, peer discovery, fork-choice/reorg | ⏳ Basic TCP sync hai; discovery/reorg baaki |
| 3 | Spam protection | Solana ne QUIC + stake-weighted QoS + priority fees lagaye (2021 outage ke baad) | ❌ Nahi hai — bot-flood test launch se pehle zaroori |
| 4 | Client diversity | Solana ne Firedancer (doosra client) banaya — ek client me bug aaye to chain na ruke | ❌ Ek hi Rust client hai |
| 5 | Security audit | Kam se kam 2 independent firms, mainnet se pehle | ❌ Abhi nahi hua |
| 6 | Bug bounty + pause button | Sab badi chains me hai; pause sirf emergency me, multisig se | ⏳ Design me hai, code me nahi |
| 7 | Slashing | Ethereum: double-sign par slashing + 36 din wait; Cosmos: double-sign 5% + tombstone | ✅ 10% slashing + jail code me hai (par stake lock nahi hai) |
| 8 | Block explorer | Har chain ka apna explorer hota hai (Etherscan, Solscan jaisa) | ❌ Nahi hai |
| 9 | Wallet | Official wallet + hardware wallet support | ⏳ Web demo fake hai; Rust core + API hai |
| 10 | Testnet | Public testnet + faucet, kai machines par nodes | ⏳ Agla step (Phase 3 ke baad) |
| 11 | Docs | Developer 10 minute me pehli transaction bhej sake | ⏳ Docs hain, developer quickstart baaki |

---

## 3. Tokenomics checklist — numbers kya hone chahiye

| # | Cheez | Benchmark | BUT (Sanjay ka faisla) | Sifarish |
|---|---|---|---|---|
| 1 | Decimals | Kam se kam 6 (Cosmos); aam 8–9; EVM 18 | **3 (1 BUT = 1,000 bits)** | **BADLO → 6 decimals.** 3 decimals har chain se kam hai; micro-fees aur chhote payments mushkil honge. "bits" naam rakho, 1 BUT = 1,000,000 bits karo (Cosmos/USDC jaisa). |
| 2 | Max supply | Hard cap achha hai (BTC 21M, AVAX 720M, Sui 10B). Cap hatana bura (Polygon −97%) | **1 billion, hard cap** | **RAKHO.** Sahi hai. Cap ko code + docs dono me likho aur kabhi mat badlo. |
| 3 | Emission / release | Fixed public schedule: BTC halving (4 saal), Solana disinflation (15%/yr → 1.5%), AVAX slowing mint | **Demand-based release, formula nahi hai** | **BADLO → fixed schedule.** Koi bhi successful chain "demand-based" reward nahi deti — demand naapna mushkil hai, aur unpredictable reward par validator bharosa nahi karte. Halving (4 saal) ya Solana-jaisi disinflation rakho. |
| 4 | Block reward | Fixed + halving (BTC), ya formula (ETH: 166.3 ÷ √stake; Cosmos: bonded ratio se 7–10%) | **Demand-based, fixed number nahi** | **BADLO → formula likho.** Cosmos-jaisa model sabse kareeb hai: reward staking ratio se adjust ho. Ya seedha: shuru me X BUT/block, har 4 saal aadha. |
| 5 | Minimum stake | Delegator: 1 coin ya kam (Sui 1, Solana ~1, Cosmos koi minimum nahi). Validator: 2,000 (BNB/AVAX) se 32 ETH tak | **1–5% of RELEASED supply** | **BADLO → chhota fixed number.** Released supply ka 1% bhi bahut bada hai: 100M release par 1% = 1,000,000 BUT; 500M release par 1% = 5,000,000 BUT. Itna bada stake sirf ameer log de payenge → validator kam honge → centralization (EOS ke 21 producers wali galti). Sifarish: validator 1,000 BUT, delegator 1 BUT — taaki bahut log jud sakein. |
| 6 | Unbonding period | Solana 2–3 din, NEAR 36–48h, Sui ~24h, Polygon 3–4 din, BNB 7 din, Cosmos 21 din (sabse lamba, aur iski bhi aalochna hoti hai) | **Tiers: 90 din → 5 saal; unstake par 72h gradual release** | **DO HISSON ME BAANTO.** (a) Lock tiers 90 din–5 saal **rakho** — Avalanche-jaisa model hai (lamba lock = zyada reward), ye theek hai. (b) Unbonding **2–7 din** rakho — 90 din se 5 saal ka unbonding duniya ki har chain se 4–80 guna lamba hai; log stake hi nahi karenge. 72 ghante me dheere-dheere release wala idea **rakho** — wo achha hai. |
| 7 | Allocation | Team 0–20% (BTC 0%, AVAX 10%, ETH 16.7%, SOL ~48% zyada); rewards pool: AVAX 50%, NEAR 39%; community: Sui 50%, Aptos 51% | **Rewards 23%, Community 17%, Team 10%, baaki 50% undecided** | **Baaki 50% jaldi decide karo.** Aadhi supply ka hissa khula chhodna investors ko darata hai (ICP ka sabak: opaque allocation = bharosa khatm). Sifarish: public sale/distribution 15%, treasury/reserve 15%, ecosystem grants 10%, liquidity 10%. Team 10% theek hai — par **4 saal vesting** lagao (Avalanche-jaisa), bina vesting ke team dump kar sakti hai. Rewards 23% thoda kam hai — AVAX 50% aur NEAR 39% rakhte hain; kam se kam 30% socho. |
| 8 | Fee model | Solana ~$0.00025; AVAX 100% burn; ETH EIP-1559 burn; BNB part burn; NEAR 70% burn | **Solana-jaisi kam fees, bits me; burn abhi nahi** | **RAKHO + burn jodo.** Kam fees sahi hai. Aage chal kar fee ka hissa burn karna achha hai (AVAX 100% burn sabse saaf model hai) — burn trustless hai, kisi wallet par bharosa nahi chahiye. |
| 9 | Endgame (supply khatm hone par) | Bitcoin: 2140 ke baad sirf fees, seedha miner ko. Sui: Storage Fund (protocol ka fund, rules code me, principal kabhi kharch nahi hota) | **Fees ek main wallet me, usse gradual rewards + thoda team ko; community ko kuch nahi** | **Wallet ko protocol fund banao.** "Main wallet" agar kisi insaan ke control me hai to wo centralization + trust ka risk hai (Ronin bridge $624M hack; ICP treasury dump). Sui-jaisa model lo: fund ke rules code me hon, koi insaan paise na nikaal sake; control multisig + timelock se ho. Ye mainnet se pehle decide karna zaroori hai. |

---

## 4. Security checklist

| # | Cheez | Kyun zaroori hai |
|---|---|---|
| 1 | Independent audit (kam se kam 2 firms) | Bina audit ke mainnet mat chalao — har badi chain audit karwati hai. |
| 2 | Bug bounty | White-hat hackers ko inaam do taaki wo bug pehle dhoondein, attacker baad me nahi. |
| 3 | Multisig + timelock | Ek insaan akele contract ya fund na badal sake; bade badlav 2–3 din baad lagu hon. |
| 4 | Pause button | Emergency me chain ko roka ja sake — par sirf multisig se, ek insaan se nahi. |
| 5 | Bridge mat banao (apna) | Bridges sabse zyada hack hote hain (~$2.8bn+ chori: Ronin $624M, BNB Bridge $568M, Wormhole ~$320M). Multi-chain ke liye IBC ya LayerZero/Wormhole/CCIP jaisa established protocol use karo. |
| 6 | Client diversity | Ek client me bug aaye to doosra client chain chalaye (Solana: Firedancer). |
| 7 | Incident response plan | Hack/outage ho to kaun kya karega, pehle se likha ho — Solana 2021 me validators ko phone karke restart karna pada tha. |

---

## 5. Legal checklist (India)

- Crypto ban nahi hai, par legal tender bhi nahi hai.
- Tax: 30% flat tax + 1% TDS; loss set-off nahi hota.
- FIU-IND registration zaroori hai agar business crypto handle kare.
- **Coin sale / ICO bina crypto vakeel ke bilkul nahi.**
- Koi bhi price/value ki guarantee mat do — kai deshon me illegal ho sakta hai.
- (Detail docs/USERS-AND-VALUE-PLAN.md §6 me hai.)

---

## 6. Ecosystem + community checklist

| # | Cheez | Benchmark |
|---|---|---|
| 1 | Grants program | Polygon: 1 billion POL grants; Avalanche Retro9000 $40M; NEAR $800M fund. Chhote se shuru karo: pehle 5–10 developers ko chhote grants. |
| 2 | Hackathons | Solana Radar: 13,672 participants, 156 desh. Hackathon audition hai, finish line nahi — winners ko aage badhao. |
| 3 | Developer docs | 10 minute me pehli transaction — agar docs mushkil hain, developer nahi aayega. |
| 4 | Governance ka rasta | Aaj faisle Sanjay lete hain (theek hai, shuruaat me). Mainnet se pehle likho: aage faisle kaise honge — on-chain voting (Cosmos-jaisa) ya EIP-jaisa proposal process (Ethereum-jaisa)? |
| 5 | Decentralization target | Kam se kam 100 validators ka lakshya (Polygon 105, Cosmos 180). EOS ke 21 producers wali galti mat karo. Cloud diversity bhi dekho — Aptos ka 1/3 stake ek hi cloud provider par hai. |
| 6 | Transparency | Allocation, treasury, vesting — sab public ho. ICP ka sabak: jo chhupaya gaya, usi par bharosa toota. |

---

## 7. Fail hui chains se sabak (numbers ke saath)

| Chain | Kya hua | Sabak |
|---|---|---|
| **Terra/Luna (May 2022)** | UST algorithmic stablecoin, zero dollar reserve; Anchor ne ~20% APY diya subsidy se; ~$40–45B ek hafte me khatm; LUNA supply 350M se 6.5 trillion ho gayi | Fixed high yield subsidy se mat do; ek app me 74% supply mat aane do |
| **EOS (2018–2024)** | $4B ICO raise karke bhi ecosystem nahi bana; sirf 21 block producers; exchanges ne vote-trading ki; token ATH se 99.6% gira | Paisa ≠ ecosystem; kam validators = centralization |
| **ICP (May 2021)** | Launch par ~$630–730, ek mahine me ~95% gira; Foundation ke ~25% supply par koi vesting nahi thi, chhote investors par 4 saal vesting thi | Insiders ka lock-up public se chhota mat rakho; allocation transparent rakho |
| **Bridges (2022–)** | Ronin $624M, BNB Bridge $568M, Wormhole ~$320M, Nomad $190M — kul ~$2.8bn+ | Apna bridge mat banao; established protocol use karo |
| **Polygon POL** | Cap hatakar +2%/yr emission shuru ki; POL ATH se ~97% gira | Supply cap ko chup-chap mat hatao |
| **Airdrops (aam)** | 88% airdropped tokens 3 mahine me price kho dete hain; ~64% recipients milte hi bech dete hain | Airdrop tabhi jab product pehle se chal raha ho, retroactive, vesting ke saath |

---

## 8. Sanjay ke faislon ka audit — ek table me

| Faisla | Sanjay ka faisla | Benchmark | Sifarish |
|---|---|---|---|
| Decimals | 3 (1 BUT = 1,000 bits) | Kam se kam 6; aam 8–9 | **BADLO → 6** |
| Max supply | 1 billion, hard cap | BTC 21M, AVAX 720M, Sui 10B | **RAKHO** |
| Release | Demand-based, dheere-dheere | Fixed schedule (halving/disinflation) | **BADLO → fixed schedule** |
| Block reward | Demand-based, fixed number nahi | Fixed + halving ya formula | **BADLO → formula likho** |
| Min stake | 1–5% of released supply | Delegator 1 coin ya kam; validator 2,000 (BNB/AVAX) | **BADLO → validator 1,000 BUT, delegator 1 BUT** |
| Staking tiers | 90 din → 5 saal; unstake 72h gradual | Lock tiers: AVAX 2 hafte–1 saal; Unbonding: 1–7 din (Cosmos 21 din sabse lamba) | **DO HISSON ME BAANTO: lock tiers rakho, unbonding 2–7 din** |
| Allocation | Rewards 23%, Community 17%, Team 10%, baaki 50% undecided | Team 0–20%; rewards 39–50%; community 17–51% | **Baaki 50% decide karo; team par 4 saal vesting** |
| Fees | Solana-jaisi kam, bits me | Solana ~$0.00025; AVAX 100% burn | **RAKHO + aage burn jodo** |
| Endgame | Fees main wallet me → gradual rewards + team | BTC: fees seedha miner ko; Sui: Storage Fund (code rules) | **Wallet ko protocol fund banao (code rules + multisig)** |

---

## Sources (research, 7 Nov 2026)

- Bitcoin: blockstream.com (satoshi, 21M cap, halving); bitcoinadvisory.co (halving schedule).
- Ethereum: stakingrewards.com (18 decimals, 32 ETH); beaconcha.in (withdrawal queue); coindoo.com (EIP-1559 burn, fee drop).
- Solana: whatssolana.com (9 decimals, inflation); datawallet.com (staking ~68.9%, ~2–3 din unbonding); alchemy.com (Alpenglow).
- BNB Chain: docs.bnbchain.org (PoSA, 45 validators, 2,000 BNB min, 7 din unbonding); blockonomi.com (burn stats).
- Polygon: stakingrewards.com (allocation, 105 validators, 3–4 din unbonding); cryptowatchdaily.com (POL +2%/yr emission).
- Cosmos: docs.cosmos.network (21 din unbonding, slashing 5%, governance); securities.io (inflation 7–10%, Prop 848).
- Avalanche: bitcoin.com (720M cap, 9 decimals); stakingrewards.com (2,000 AVAX min, lock 2 hafte–1 saal); tokenomics.com (allocation).
- NEAR: nomicon.io (24 decimals); blog.web3wagmi.com (inflation 5% → 2.5%, Oct 2025); coinlaw.io (unbonding 36–48h).
- Sui: whatssui.com (9 decimals, 10B cap, 24h unbonding); docs.sui.io (Storage Fund, stake subsidies).
- Aptos: aptcore.one (8 decimals, 11 APT delegate min); Dropstab.com (2.1B cap, Mar 2026); stakingrewards.com (14–30 din unbonding).
- Failed chains: getswoopr.com / terra-claim.com (Terra $40–45B); switchere.com (EOS $4B ICO); Arkham report (ICP 95% drop); spark.money (bridge hacks $2.8bn+).
- Airdrops: Keyrock (88% lose value in 3 months); DappRadar; LayerZero Sybil case.

*Ye document koi investment advice nahi hai, aur koi value/price ki guarantee nahi deta. Numbers 7 Nov 2026 ke online sources se hain.*
