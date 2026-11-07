# BUT Network — Tokenomics (PARTLY FINAL)

> **KUCH FAISLE FINAL HAIN, KUCH ABHI BHI PENDING HAIN.**
> Sanjay ne 2026-11-07 ko khud 7 me se 5 faisle liye (neeche §8 dekho).
> Jo FINAL likha hai, wo chain me likha ja chuka hai. Jo PENDING hai,
> wo abhi bhi sujhaav hai.
>
> Date: 7 November 2026 (Phase 2). Status: **PARTLY FINAL — Sanjay ke faisle, 2026-11-07**.

---

## 1. Sabse chhoti unit (smallest unit) — FINAL (Sanjay, 2026-11-07)

| Cheez | FINAL faisla |
|---|---|
| Coin ka naam | **BUT** |
| Sabse chhoti unit ka naam | **bits** (Sanjay ne B-I-T-S spell kiya) |
| 1 BUT = kitne bits | **1,000 bits** (3 decimal) |
| Chain me amounts kis me hain | Sirf **bits** me (u64 integer). Chain ko "BUT" ka matlab nahi pata — wo sirf bits ginta hai |

Matlab:
- 1 BUT = 1,000 bits
- 100 BUT = 100,000 bits
- 1 bit = 0.001 BUT

> Note: Sanjay ka analogy tha — "jaise 1 Bitcoin me satoshi hote hain,
> waise 1 BUT me bits honge." (Sach baat: 1 Bitcoin me asal me
> 10 crore satoshi hote hain, 6 lakh nahi — par Sanjay ka faisla saaf hai:
> 1 BUT = 1,000 bits.)
>
> Code me `BITS_PER_BUT = 1_000` constant `chain/src/consensus.rs` me
> likha ja chuka hai. Display conversion (bits ↔ BUT) abhi jodna baaki hai.

---

## 2. Total supply — FINAL (Sanjay, 2026-11-07)

| Cheez | FINAL faisla |
|---|---|
| Max supply (kabhi isse zyada nahi banegi) | **1,000,000,000 BUT** (1 arab / 1 billion BUT) |
| Bits me | 1,000,000,000,000 bits (10^12 — u64 me aaram se aata hai) |
| Supply kaise release hogi | **Dheere-dheere, demand aur value ke hisaab se** — sari ek saath unlock nahi hogi. Demand zyada hogi to release dheere-dheere badhegi; demand kam hogi to release slow rahegi |
| Naye coins kaise bante hain | Sirf **block rewards** se (neeche §4) — koi bhi ek saath saare coins nahi bana sakta |
| Kya supply kabhi badhegi? | **Nahi** — 1 billion hard cap hai. Cap badlana ho to chain ka rule badalna padega (sab validators ki sahamati se) |

> Code me `MAX_SUPPLY_BUT = 1_000_000_000` constant `chain/src/consensus.rs`
> me likha ja chuka hai. Release schedule (kab kitne coins unlock honge)
> ka exact formula abhi design hona baaki hai — wo demand ke hisaab se
> banega, isliye abhi sirf principle FINAL hai.

---

## 3. Shuru ki baant (genesis allocation) — PARTLY FINAL (Sanjay, 2026-11-07)

Genesis block me 1 billion BUT kaise baante jayein:

| Hissa | % | Kitna (BUT) | Status |
|---|---|---|---|
| Rewards pool (validator rewards) | 23% | 230,000,000 | **FINAL — Sanjay** |
| Community (developers, grants, users) | 17% | 170,000,000 | **FINAL — Sanjay** |
| Team / founders (Bharat Unified Technology) | **1%** | **10,000,000** | **FINAL — Sanjay (2026-11-07, report ke baad badla: pehle 10% tha, ab 1%)** |
| Baaki 59% (public sale / treasury / ecosystem — abhi tay nahi) | 59% | 590,000,000 | **PENDING** |
| **Kul** | **100%** | **1,000,000,000** | |

> Team ka hissa 10% se **1%** kar diya gaya hai (Sanjay ka faisla,
> 2026-11-07, research report padhne ke baad). Isse 9% khali hua hai —
> wo baaki 59% me jud gaya hai, uska hissa abhi tay nahi hua hai.
> Team ko ongoing funding block reward se milegi (neeche §4 dekho —
> har block reward ka 0.1% team account me).
>
> Purana proposal (rewards 40%, community 25%, team 15%, sale 10%,
> treasury 10%) Sanjay ke faislon se badal gaya hai.
>
> Genesis allocations abhi chain me is hisaab se nahi hain — Phase 1–2
> ki demo chains me sirf Alice ko 1,000,000 bits (1000 BUT) test ke liye
> diye gaye the. Asli genesis tab banega jab poori baant final hogi.

---

## 4. Validator rewards (block reward) — FINAL (principle) — Sanjay, 2026-11-07

| Cheez | FINAL faisla |
|---|---|
| Block reward kitna | **Demand ke hisaab se hoga** — fixed number nahi hai. Demand/value zyada hogi to reward us hisaab se, kam hogi to kam |
| Block time (lakshya) | **~5 seconds** per block (proposal se hi — is par alag se faisla nahi liya gaya) |
| Halving | **Abhi nahi** — purana 5 BUT + 4-saal halving wala proposal Sanjay ke demand-based faisle se badal gaya hai |
| Reward kisko milta hai | Block banane wale validator ko (proposer), aur aage chal kar vote dene walon ko bhi hissa |
| Reward kahan se aata hai | Rewards pool se (§3 ki baant final hone par) — jab pool khatm ho jayega, tab sirf fees se rewards milenge (Bitcoin jaisa model) |

> Demand-based reward ka exact formula abhi design hona baaki hai
> (demand kaise naapi jayegi, reward kab badhega/ghatenga). Isliye
> principle FINAL hai, formula PENDING hai.
>
> **Team share (FINAL — Sanjay, 2026-11-07):** Jab bhi koi block reward
> hoga, uska **0.1% team ke account me jayega**. Ye team ki ongoing
> funding hai — team ka genesis allocation sirf 1% hai, isliye team ko
> har block reward se chhota hissa milta rahega. **Code me hai (Phase 3a):**
> har block par reward mint hota hai, proposer ko 99.9% aur team ko 0.1%
> milta hai; team account genesis me set hota hai, set na ho to team share
> `team_accrued` me jama hota hai.
>
> Code me abhi block reward hai (Phase 3a) — par **amount PLACEHOLDER hai**
> (5 BUT/block + ~4 saal me halving, sirf mechanism ke liye). Sanjay ka
> FINAL demand-based formula aate hi ye badlega. Validator ko fees ke
> saath-saath block reward bhi milta hai.

---

## 5. Fees — FINAL (principle) — Sanjay, 2026-11-07

| Cheez | FINAL faisla |
|---|---|
| Fees kaisi hongi | **Solana jaisi bilkul kam** — Sanjay ka faisla hai |
| Fees kis me kategi | **Bits me** (sabse chhoti unit) |
| Fee kisko milti hai | Block banane wale validator ko (ye Phase 1 se hi code me hai) |
| Minimum fee (exact number) | **Abhi tay nahi hua** — "bilkul kam" principle FINAL hai, exact bits number PENDING hai (Solana ki fee ~$0.00025 hai — us hisaab se number tay hoga) |
| Fee burn? | **Abhi nahi** — aage chal kar fee ka ek chhota hissa (jaise 10%) burn karne par vichaar ho sakta hai. Final faisla Sanjay ka |

> Chain abhi bhi 0 fee allow karti hai — minimum fee lagana baaki hai.

---

## 6. Staking rules — PROPOSAL

Ye rules `chain/src/consensus.rs` ke constants se jude hain:

| Cheez | Proposal | Code me abhi |
|---|---|---|
| Validator banne ke liye minimum stake | **1% se 5% tak — RELEASED supply ka** — FINAL (Sanjay, 2026-11-07) | Code me hai (Phase 3a): `min_stake_for(released_supply)` = released supply ka 1% (floor); purana 100 BUT placeholder hata diya gaya |
| Minimum stake ka base | **Released (circulating) supply** — total supply ka nahi | Sanjay ka faisla: jitni supply release hoti jayegi, uska 1-5%. Jaise aaj aadhi supply release hai to uska 1-5%; released supply badhti jayegi to 1-5% usi hisaab se badhta jayega |
| Staking lock tiers | **90 din → 180 din → 1 saal → 2 saal → 3 saal → 5 saal** — FINAL (Sanjay, 2026-11-07) | Code me hai (Phase 3a): `chain/src/staking.rs` me `LockTier` — stake karte hi coins lock hote hain, kharch nahi ho sakte |
| Unstake par release | **72 ghanton ke andar, dheere-dheere** — ek saath nahi | Code me hai (Phase 3a): `Unbonding` — 72 ghante me linear gradual release, claim se milega |
| Proposer chunav | Stake-weighted (zyada stake = zyada chance) | Code me hai (`proposer_for_round`) |
| Block final hone ke liye vote | Kul active stake ke **2/3 se zyada** approvals | Code me hai (`VoteTally::has_quorum_for`) |
| Slashing — equivocation (ek height par do blocks ko vote) | Stake ka **10%** kat jayega | Code me hai (`SLASH_PERCENT = 10`) |
| Slashing — baar-baar galat kaam | Stake `MIN_STAKE` se neeche gira to **jail** | Code me hai |
| Unbonding period (unstake ke baad coins kab wapas milenge) | **Tiers: pehle 90 din, phir 180 din, phir 1 saal, 2 saal, 3 saal, 5 saal** — FINAL (Sanjay, 2026-11-07) | Code me hai (Phase 3a): lock tiers lock period hain; unstake par 72 ghante me dheere-dheere release (doc §6 note wali samajh ke hisaab se) |
| Unbonding par note | Sanjay ne "staking" tiers bataye hain — 90 din se 5 saal tak. Ye tiers lock period hain ya unbonding period, ye samajh record ki gayi hai: **staking lock tiers + unstake par 72 ghante me dheere-dheere release**. Agar Sanjay ka matlab kuch aur tha to ye badlega. Dusri chains se tulna: Cosmos me unbonding 21 din, Ethereum me kuch din — 90 din se 5 saal bahut lamba hai, isse staking kam ho sakti hai | — |
| Jail se wapas aana | Dobara stake karke (stake ≥ min stake — released supply ka 1%) | Code me hai (`restake`) |

---

## 7. Kya-kya abhi code me NAHI hai (imaandari se)

1. **Block reward nahi hai** — validator ko sirf fees milti hain.
2. **Stake-locking nahi hai** — stake sirf validator set me darj hai, coins chain par lock nahi hote. Koi bhi apne "staked" coins ko abhi bhi kharch kar sakta hai (ye sabse badi kami hai — Phase 3 me band hogi).
3. **Unbonding period nahi hai** — kyunki stake-locking hi nahi hai.
4. **Vesting nahi hai** — team ke coins ka time-lock abhi sirf kagaz par hai.
5. **Fee burn nahi hai.**
6. **Genesis allocation is table ke hisaab se nahi hai** — demo chains me sirf test allocations hain.

---

## 8. Sanjay ke faisle (2026-11-07) — kya FINAL hua, kya PENDING hai

1. **1 BUT = kitne bits? — FINAL: 1 BUT = 1,000 bits.** Sabse chhoti unit ka naam **bits** (Sanjay ne B-I-T-S spell kiya). Code me `BITS_PER_BUT = 1_000` likha gaya.
2. **Total supply kitni? — FINAL: max 1 billion BUT, dheere-dheere release hogi.** Demand aur value ke hisaab se release hogi, sari ek saath unlock nahi hogi. Code me `MAX_SUPPLY_BUT = 1_000_000_000` likha gaya. Release schedule ka exact formula PENDING hai.
3. **§3 wali baant (%) — PARTLY FINAL: rewards 23%, community 17%, team 1%.** Team pehle 10% tha, report padhne ke baad Sanjay ne **1%** kar diya (2026-11-07). Baaki 59% (public sale / treasury / ecosystem) PENDING hai — khali hue 9% ka hissa bhi isme hai.
4. **Block reward kitna? — FINAL (principle): demand ke hisaab se hoga.** Fixed number nahi hai. Exact formula PENDING hai (demand kaise naapi jayegi).
5. **Minimum stake kitna? — FINAL: 1% se 5% tak, RELEASED supply ka.** Jitni supply release hoti jayegi, uska 1-5% (jaise aadhi supply release hai to uska 1-5%). Code me hai (Phase 3a): `min_stake_for(released_supply)` = released supply ka 1% (floor); purana 100 BUT placeholder hata diya gaya.
6. **Fees — FINAL (principle): Solana jaisi bilkul kam, bits me kategi.** Exact minimum fee number PENDING hai. Fee burn: abhi nahi.
7. **Unbonding / staking tiers — FINAL: 90 din → 180 din → 1 saal → 2 saal → 3 saal → 5 saal.** Unstake par release **72 ghanton ke andar, dheere-dheere** — ek saath nahi. (Samajh note §6 me dekho.)
8. **Supply khatm hone ke baad (endgame) — FINAL: jab poori supply release ho jayegi to na rewards, na community, na team — blockchain poori tarah swatantra ho jayega.** Uske baad jo fees kategi, wo ek main wallet me ikatthi hogi; usse rewards dheere-dheere validators ko jayenge aur thoda hissa team ko batega; community ko us phase me kuch nahi milega.
9. **Block reward se team share — FINAL: har block reward ka 0.1% team ke account me jayega** (Sanjay, 2026-11-07). Team ka genesis allocation sirf 1% hai, isliye ongoing funding block rewards se milegi.
10. **Liquidity rule — FINAL: swap sirf available liquidity tak hoga** (Sanjay, 2026-11-07, 2026-11-07 ko clarify kiya). User ko swap se PEHLE bataya jayega ki kitna available hai (jaise 49%); user phir bhi poora swap kare to transaction FAIL nahi hogi — uske saare coins pool me chale jayenge, use sirf available output milega, baad me kuch nahi milega; liquidity badhne par dobara swap kar sakta hai.

---

## 9. Supply khatm hone ke baad — endgame (FINAL — Sanjay, 2026-11-07)

Jab poori 1 billion supply release ho jayegi:

- **Na rewards, na community, na team** — inke hisse khatm ho jayenge.
- **Blockchain poori tarah swatantra ho jayega.**
- Uske baad jo fees kategi, wo **ek main wallet me ikatthi hogi**.
- Us main wallet se **rewards dheere-dheere validators ko jayenge**,
  aur **thoda hissa team ko batega**.
- **Community ko us phase me kuch nahi milega.**

> Ye Bitcoin ke "2140 ke baad sirf fees" wale model jaisa hai — par BUT me
> fees ek main wallet se dheere-dheere baanti jayengi. Main wallet ka
> control (multisig? kaun?) abhi tay nahi hua hai — ye PENDING hai.

---

## 10. Muse ki sifarish (research ke baad) — 7 Nov 2026

> **YE SIFARISH HAI, FAISLA NAHI.**
> Sanjay ne kaha: wo tokenomics ke numbers par tukke maar rahe hain aur
> chahte hain ki Muse khud online dekh kar bataye kaunsa number sabse best
> rahega. Isliye 10 chains (Bitcoin, Ethereum, Solana, BNB Chain, Polygon,
> Cosmos, Avalanche, NEAR, Sui, Aptos) + fail hui chains (Terra, EOS, ICP)
> se benchmarks nikaale gaye — poora detail
> docs/SUCCESSFUL-CHAIN-CHECKLIST.md me hai.
>
> **Is section se upar §8 ke FINAL faisle NAHI badle hain.** Har sifarish
> ke saath likha hai: rakho ya badlo, aur kyun. Aakhiri faisla hamesha
> Sanjay ka rahega — jo sifarish manzoor hogi, tabhi wo §8 me FINAL likhi
> jayegi.

### 1. Decimals — BADLO: 3 se 6 karo (naam "bits" rakho)

- **Sanjay ka faisla:** 1 BUT = 1,000 bits (3 decimals).
- **Benchmark:** Kam se kam 6 decimals hain (Cosmos 6, Bitcoin 8, Solana/Sui/Avalanche 9, Ethereum/BNB/Polygon 18). 3 decimals kisi bhi successful chain me nahi hai.
- **Sifarish:** 1 BUT = 1,000,000 bits karo. Naam "bits" wahi rakho.
- **Kyun:** 3 decimals par sabse chhoti raqam 0.001 BUT hai — micro-payments aur bahut kam fees ke liye bahut mota hai. Agar BUT ki keemat kabhi badhi to 0.001 BUT bhi mehenga ho jayega. 6 decimals Cosmos/USDC-jaisa simple hai (₹1 = 100 paise ki tarah samajh aata hai).

### 2. Max supply 1 billion — RAKHO

- **Sanjay ka faisla:** Max 1 billion BUT, hard cap, dheere-dheere release.
- **Benchmark:** Bitcoin 21M, Avalanche 720M, Sui 10B — hard cap wali chains par bharosa zyada hota hai. Polygon ne cap hataya to POL ATH se ~97% gira.
- **Sifarish:** Rakh lo. Cap ko code + docs dono me likho aur kabhi mat badlo — fixed policy hi bharosa banati hai (Bitcoin ka sabse bada sabak).

### 3. Release + block reward — BADLO: demand-based se fixed schedule par aao

- **Sanjay ka faisla:** Supply demand ke hisaab se release hogi; block reward bhi demand ke hisaab se, fixed number nahi.
- **Benchmark:** Koi bhi successful chain "demand-based" reward nahi deti. Bitcoin: fixed halving (4 saal). Solana: inflation schedule 8% → 1.5% (15%/yr disinflation). Ethereum: formula (166.3 ÷ √stake). Cosmos: inflation bonded ratio se 7–10%. Avalanche: mint cap ki taraf slow hota hai.
- **Sifarish:** Fixed, public schedule rakho — jaise shuru me X BUT/block, har 4 saal aadha (Bitcoin-jaisa halving), ya Solana-jaisi disinflation. Demand ko fees se naapo, supply se nahi.
- **Kyun:** "Demand" ko code me naapna mushkil hai — kaun tay karega demand kitni hai? Agar reward ka formula saaf nahi hai to validator bharosa nahi karenge, aur reward badalna ek insaan ke haath me aa jayega (centralization). Predictable policy = bharosa.

### 4. Minimum stake 1–5% of released supply — BADLO: chhota fixed number rakho

- **Sanjay ka faisla:** Minimum stake released supply ka 1% se 5% tak.
- **Benchmark:** Delegator minimum: Sui 1 SUI, Solana ~1 SOL, Cosmos koi minimum nahi, Polygon 1 POL, BNB 1 BNB. Validator minimum: BNB/AVAX 2,000 coins, Aptos 1M APT pool (sabse zyada, aur iski aalochna hoti hai).
- **Sifarish:** Validator minimum **1,000 BUT**, delegator minimum **1 BUT**.
- **Kyun — number se samjho:** Agar 100M BUT release hue hain to 1% = 1,000,000 BUT. Agar 500M release hue to 1% = 5,000,000 BUT. Itna bada stake sirf ameer log ya team de payenge — aam log validator nahi ban payenge, validator kam honge, aur chain centralized ho jayegi (EOS ke sirf 21 producers wali galti). Har successful chain ka minimum stake chhota hota hai taaki bahut log jud sakein.

### 5. Staking tiers 90 din–5 saal — DO HISSON ME BAANTO

- **Sanjay ka faisla:** Tiers 90 din → 180 din → 1 saal → 2 saal → 3 saal → 5 saal; unstake par 72 ghanton ke andar dheere-dheere release.
- **Benchmark:** Unbonding — Sui ~24h, NEAR 36–48h, Solana 2–3 din, Polygon 3–4 din, BNB 7 din, Cosmos 21 din (sabse lamba, aur iski bhi aalochna hoti hai). Lock tiers — Avalanche: 2 hafte se 1 saal tak lock, lamba lock = zyada reward.
- **Sifarish:** (a) **Lock tiers rakho** — 90 din se 5 saal tak lock karna, lamba lock = zyada reward (Avalanche-jaisa model, ye theek hai). (b) **Unbonding 2–7 din rakho** — 90 din se 5 saal ka unbonding duniya ki har chain se 4–80 guna lamba hai; log apne coins 5 saal ke liye band dekh kar stake hi nahi karenge. (c) 72 ghante me dheere-dheere release wala idea **rakho** — wo achha hai.
- **Kyun:** Lock aur unbonding do alag cheezein hain. Lock matlab "maine khud itne din ke liye band kiya, badle me zyada reward milega" — ye theek hai. Unbonding matlab "nikalne ke baad kitne din wait karna padega" — ye chhota hona chahiye, warna log aayenge hi nahi.

### 6. Allocation (baaki 50%) — JALDI DECIDE KARO + team par vesting lagao

- **Sanjay ka faisla:** Rewards 23%, Community 17%, Team 10%, baaki 50% undecided.
- **Benchmark:** Team: Bitcoin 0%, Avalanche 10%, Ethereum 16.7%, Solana ~48% (zyada mana jata hai). Rewards pool: Avalanche 50%, NEAR 39%. Community: Sui 50%, Aptos 51%.
- **Sifarish:** Baaki 50% aise baanto — public sale/distribution 15%, treasury/reserve 15%, ecosystem grants 10%, liquidity/market making 10%. Team ke 10% par **4 saal vesting** lagao (Avalanche-jaisa: har saal 25%). Rewards 23% thoda kam hai — kam se kam 30% socho (AVAX 50%, NEAR 39% rakhte hain).
- **Kyun:** Aadhi supply ka hissa khula chhodna investors aur users ko darata hai (ICP ka sabak: jo chhupaya gaya, usi par bharosa toota). Bina vesting ke team ek saath bech sakti hai — ICP me Foundation par koi vesting nahi thi aur token 95% gira.

### 7. Fees (Solana-jaisi kam, bits me) — RAKHO + aage burn jodo

- **Sanjay ka faisla:** Fees Solana-jaisi bilkul kam, bits me kategi.
- **Benchmark:** Solana fee ~$0.00025; Avalanche 100% fees burn; Ethereum EIP-1559 burn; NEAR 70% burn.
- **Sifarish:** Kam fees rakho. Aage chal kar fee ka hissa **burn** karna jodo — Avalanche ka 100% burn sabse saaf model hai.
- **Kyun:** Burn trustless hai — kisi wallet ya insaan par bharosa nahi chahiye. Fee se validator ko inaam aur burn se scarcity — dono ek saath ho sakte hain.

### 8. Endgame (fees main wallet me) — WALLET KO PROTOCOL FUND BANAO

- **Sanjay ka faisla:** Supply khatm hone par fees ek main wallet me ikatthi hongi; usse gradual rewards + thoda team ko; community ko kuch nahi.
- **Benchmark:** Bitcoin: 2140 ke baad fees seedha miner ko — koi beech wala wallet nahi. Sui: Storage Fund — protocol ka fund, rules code me hain, principal kabhi kharch nahi hota. Ronin bridge: 5-of-9 multisig hack, $624M chori.
- **Sifarish:** "Main wallet" ko **protocol fund** banao — uske rules code me likhe hon, koi insaan paise na nikaal sake; control multisig + timelock se ho. Ye mainnet se pehle decide karna zaroori hai.
- **Kyun:** Agar main wallet kisi insaan ya chhoti committee ke control me hai to wo centralization + hack ka risk hai. Sui-jaisa model lo: fund code se chale, insaan se nahi.

### Top 5 sifarishein (chhota saar)

1. **Decimals 3 → 6 karo** (1 BUT = 1,000,000 bits) — har successful chain me kam se kam 6 hain.
2. **Minimum stake 1–5% se badal kar validator 1,000 BUT / delegator 1 BUT karo** — 1% bhi lakhon BUT hota hai, validator hi nahi ban payenge.
3. **Lock tiers rakho, unbonding 2–7 din karo** — 90 din–5 saal ka unbonding staking ko maar dega.
4. **Baaki 50% allocation decide karo + team par 4 saal vesting** — aadhi supply khuli chhodna bharosa todta hai.
5. **Block reward demand-based se fixed schedule par lao (halving/disinflation)** — predictable policy hi bharosa banati hai.


---

## 11. Liquidity rule — swap sirf available liquidity tak (FINAL — Sanjay, 2026-11-07)

- **Rule:** Swap hamesha pool me available liquidity tak hi hoga.
- Agar liquidity kam hai aur koi hazaron tokens swap karne aaye, to use
  **utna hi milega jitni liquidity hai** — chahe wo 1 token swap kare ya
  saare tokens.
- **Liquidity se zyada ka vaada kabhi nahi hoga** — na udhaar, na IOU,
  na hawa me tokens.
- **User ko swap se PEHLE bataya jayega** — screen par saaf dikhega ki
  uske itne tokens ke badle itna hi milega, aur pool me abhi bas itna
  hi hai. Jab aage liquidity badhegi to wo dobara swap kar sakta hai.
- **User phir bhi poora swap kare to transaction FAIL nahi hogi.**
  Uske saare input coins pool (liquidity) me chale jayenge, aur use
  sirf available output milega. Baad me use kuch nahi milega — na
  pending balance, na IOU.
- **User ki marzi** — karwana hai to karwao, nahi to mat karwao; swap
  karne se koi rokega nahi, par milega utna hi jitna pool me hai.

> Sanjay ne ek blockchain me aisa hi dekha tha — wahan bhi swap
> available liquidity tak hi hota hai. **Ye rule Phase 3b me ban gaya
> hai** — `chain/src/swap.rs` (AMM x·y=k, `quote()` se swap se pehle
> disclosure, SwapOp) + API `GET /swap/quote`, `POST /swap`,
> `GET /swap/pools` + CLI `swap-quote` / `swap`. Swap kabhi fail nahi
> hota liquidity ki kami se — output AMM formula se milta hai (0 bhi ho
> sakta hai), input poora pool me jata hai, koi IOU nahi.
>
> **Kyun ye rule hai (Sanjay ne bataya):** Bitcoin/Binance ke saath hue
> hack wale haadson me hackers ne chori ke coins ko swap kiya tha — isliye
> swap par sakht rule chahiye. (Research note: BNB Chain bridge hack,
> Oct 2022 — neeche §12 dekho.)
>
> **Imaandari wali baat:** Ye rule tabhi fair hai jab user ko swap se
> PEHLE saaf bataya jaye ki use kitna milega. Bina bataye poora input
> lena user ko dhokha lagega — isliye upfront disclosure is rule ka
> sabse zaroori hissa hai.

---

## 12. Hack research — Binance / BNB swap incidents (Nov 2026 research)

Sanjay ne kaha tha: internet par search karke dekho, Bitcoin/Binance ke
saath hue haadson me hackers ne ek coin ko swap kiya tha. Research me
do incidents mile — aur dusra to abhi haal ka hai (Oct-Nov 2026):

### Incident 1 — BNB Chain bridge hack (6 Oct 2022)

- BSC Token Hub (cross-chain bridge) me bug se hackers ne **2 million
  extra BNB (~$566M)** bana liye.
- Hackers ne turant swap kiya: PancakeSwap par BNB/BUSD → USDT/USDC,
  Venus lending par 900k BNB girvi rakh kar **$150M stablecoins**
  udhaar liye, phir Stargate bridge se dusri chains par bhej diye.
- **Sirf ~$100M nikal paya**, ~$7M freeze ho gaya — kyunki BNB Chain
  ne chain ko **pause** kar diya tha.
- Sabak: bridge sabse zyada hack hota hai; pause button ne nuksan roka.

### Incident 2 — Binance $6.43 billion theft (29 Oct 2026)

- North Korea ke TraderTraitor group (FBI attribution, 31 Oct 2026) ne
  Binance ke hot wallets se **$6.43 billion** churaye, 11 blockchains par.
- Hackers ne freeze se pehle tezi se swap kiya:
  - Ethereum par **434M USDT → ETH** (Tether ne 37 minute baad freeze kiya)
  - Tron par **234M USDT → TRX** (SunSwap)
  - **42M XRP (~$50M) → Bitcoin** (THORChain se)
  - **~$87M SOL** poora swap/bridge/mix kar diya
  - Kul **~$124M altcoins → 1,650 BTC** me badle
- **$2.85 billion (44%) freeze/seize** ho gaya, **$3.11 billion abhi
  bhi** linked addresses me pada hai, lagbhag kuch cash-out nahi hua.
- 20,000+ wallets, 16 blockchains, 65,000+ transactions — sab automated.
- Sabak: hackers chori ke coins ko **swap karke** freeze se bachne ki
  koshish karte hain — isliye swap par limits, pause, aur blacklist
  capability zaroori hai.

### BUT ke liye sabak

- **Liquidity rule (§11)** + **pause button** (BUT Trust Contract me
  pehle se plan hai) + shaq wali activity par swap limits.
- **Bridge sabse zyada hack hota hai** — apna custodial bridge mat
  banao (research me pehle se likha hai).
- USDT/USDC freeze ho sakte hain (issuer ke paas freeze ka button hai)
  — BUT ka apna stablecoin nahi hai, isliye ye risk BUT par alag
  tareeke se lagta hai.

> Sources (Nov 2026): Elliptic report (6 Nov 2026), CoinDesk,
> Cointelegraph, Nansen — BNB Chain bridge exploit, Oct 2022; Binance
> $6.43B theft, Oct 2026.

---

## 13. Coin family — SIRF 2 coins (FINAL — Sanjay, 2026-11-07, badla hua faisla)

Sanjay ka AKHRI faisla: BUT ecosystem me **sirf 2 coins** honge.
(Pehle 4 coins ka faisla hua tha, phir Sanjay ne badal diya — Diamond
aur Gold kabhi launch nahi honge.)

| Coin | Kaam | Limit | Launch |
|---|---|---|---|
| **BUT** (main coin) | Chain ka asli coin, main | **1 billion** (max supply, §2) — limit HAI | **BUT Coin ke saath hi launch hoga** |
| **BUT Coin** (stablecoin) | USDT ke barabar, fixed — na upar, na neeche | **Koi limit NAHI** — jitna reserve, utne coin | **BUT ke saath hi launch hoga** |

**Naam badla (FINAL — Sanjay, 2026-11-07):** Aaj se stablecoin ka naam
**BUT USDT nahi, BUT Coin** likha jayega. BUT Coin stable hoga, BUT main hoga.

**CANCELLED — kabhi launch nahi honge:**
- **BUT Diamond** — pehle rare, 21 lakh limit, badi machines se mining ka
  faisla hua tha. Ab CANCELLED hai — kabhi launch nahi hoga.
- **BUT Gold** — pehle phone se mining ka faisla hua tha. Ab CANCELLED
  hai — kabhi launch nahi hoga.

**Rules (FINAL — Sanjay, 2026-11-07):**
- BUT aur BUT Coin **saath hi saath launch** honge.
- **Dono coins ki liquidity alag-alag hogi** — dono ki ek hi liquidity nahi rakh sakte; har coin ka apna alag pool hoga.

**Khule sawal (design me likhne se pehle confirm karne hain):**
- BUT main chain abhi **PoS** (staking) par bani hai — mining nahi hoti. "BUT laptop/PC par chalega" ka matlab mining hai ya node/wallet chalana — ye confirm karna hai. Agar BUT main ko mine karna hai to PoW ya hybrid design chahiye hoga.
- BUT Coin ke liye **reserve kahan rakha jayega, audit kaun karega, legal kya hoga** — stablecoin ke liye ye sabse zaroori hai (Terra UST ka crash yaad rakho — bina reserve ke stablecoin nahi chalta).

---

## 14. Swap routing rule — bahar ke coin se BUT Coin seedha nahi (FINAL — Sanjay, 2026-11-07)

- **Poori blockchain me sirf 2 coins honge — BUT aur BUT Coin. Koi
  teesra coin kabhi nahi aayega.** (BUT Tournament bhi kabhi launch
  nahi hoga — na Diamond, na Gold, na koi teesra coin.)
- **Bitcoin (ya koi bhi bahar ka coin) seedha BUT Coin me swap nahi
  ho sakta.**
- Rasta hamesha aisa hoga: **Bitcoin → BUT → BUT Coin.**
- Matlab: koi Bitcoin lekar aaye to pehle use BUT me aana padega, phir
  BUT se BUT Coin me ja sakta hai. BUT Coin tak pahunchne ka ek hi
  rasta hai — BUT se hokar.
- Isse BUT ki demand banti hai — har bahar ka coin pehle BUT kharidega.

> **Ye rule Phase 3b me ban gaya hai** — `chain/src/swap.rs` me
> `SwapOp::verify()` seedha BTC → BUT Coin (ya kisi bhi bahar ke coin →
> BUT Coin) reject karta hai, aur API `GET /swap/quote` bhi wahi reject
> karta hai. Rasta hamesha BTC → BUT → BUT Coin hi hai. (Bridge abhi
> nahi bana hai — bahar ke coins ka on-chain swap abhi nahi hota.)
