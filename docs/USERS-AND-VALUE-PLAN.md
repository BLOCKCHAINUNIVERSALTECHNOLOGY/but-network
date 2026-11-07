# BUT Network — Users aur Coin Value ka Plan

> **Sanjay ka sawal (7 Nov 2026):** "Abhi hamare paas koi user nahi hai, aur coin ki koi value nahi hai — to agar koi aaye bhi to hum use kya denge?"
>
> Ye document usi sawal ka imaandari se jawab hai. Isme koi jhootha vaada nahi hai — coin ki value ki **guarantee koi nahi de sakta**, na hum, na koi aur. Value kaise banti hai, wo yahan simple Hindi me likha hai.
>
> Date: 7 November 2026. Status: **PLAN ONLY** — ye kagaz par plan hai, abhi koi user nahi hai, koi listing nahi hai, koi sale nahi hui hai.
>
> BUT ki aaj ki haalat (docs/REAL-NETWORK-PLAN.md se): Rust chain Phase 1+2 taiyar hai (real wallet, transactions, blocks, P2P sync, PoS+BFT, local API port 3030). Tokenomics sirf PROPOSAL hai (1 billion BUT, 1 BUT = 1,000,000 anu). Web demo abhi bhi fake hai, koi audit nahi hua, Android app nahi bani.

---

## 1. Coin ki value kahan se aati hai — simple samajh

Coin ki value koi decide nahi karta — **market karta hai**. Ye chain hai:

```
Use (kaam)  →  Demand (zaroorat)  →  Liquidity (kharidne-bechne ki jagah)  →  Price (keemat)
```

Har kadam ko samjho:

1. **Use (kaam):** Coin se koi kaam hona chahiye. Ethereum ka ETH isliye chahiye kyunki uske bina Ethereum par kuch nahi chal sakta — fees ETH me hi deni padti hain. Agar coin se kuch karna hi nahi hai, to koi kyun kharidega?
2. **Demand (zaroorat):** Jab logon ko coin **chahiye** hota hai (fees dene ke liye, stake karne ke liye, kisi app me use karne ke liye), tab demand banti hai. Bina demand ke price nahi banta.
3. **Liquidity:** Jab coin ko aasani se kharida-becha ja sake (exchange par), tab uski keemat dikhne lagti hai. Bina liquidity ke price sirf kagaz par hota hai.
4. **Price:** Demand zyada, supply kam → price upar. Demand kam, supply zyada → price neeche. Ye koi control nahi kar sakta.

**Seedhi baat:** Value pehle nahi aati, use pehle aata hai. Jo project pehle value (price) ke peeche bhaagta hai, wo aam taur par fail hota hai. Jo pehle **kaam ki cheez** banata hai, uski value baad me banti hai.

### Bitcoin ka sabse bada sabak

Bitcoin ki value isliye hai kyunki:
- Uska network duniya bhar me lakhon computers se chalta hai (security network ke size se aati hai — sirf code se nahi).
- Log use store-of-value (digital sona) ki tarah istemal karte hain.
- Supply fixed hai: sirf 21 million BTC kabhi banenge.

BUT ke paas abhi inme se kuch nahi hai — na bada network, na use, na market. Ye sach hai, aur isi sach se plan shuru hota hai.

---

## 2. Stages — testnet se value tak ka rasta

Koi bhi naya network seedha "value" par nahi pahunchta. Rasta ye hai:

| Stage | Kya hota hai | User ko kya milta hai | BUT ki haalat |
|---|---|---|---|
| **0. Foundation** | Chain bane, test ho, audit ho | Kuch nahi — abhi sirf code hai | ✅ Phase 1+2 done; audit baaki |
| **1. Testnet** | Free test coins, log chain ko todein, bugs dhoondein | Free test coins + seekhne ka mauka | ⏳ Agla step (Phase 3 ke baad) |
| **2. Pehla real use-case** | Ek kaam jo sach me chale (payment, staking, prediction) | Coin se kuch **kar** sakna | ⏳ Design hai, code baaki |
| **3. Community** | Developers aur users aayein, apps banein | Kaam ki apps, madad, rewards | ⏳ Shuru nahi hua |
| **4. Mainnet** | Asli chain live, asli coins | Asli wallet, asli transactions | ⏳ Testnet ke baad |
| **5. Listing** | DEX par pehle, phir CEX par | Coin khareedna-bechna | ⏳ Bahut door hai |
| **6. Value** | Use + demand + liquidity se price bane | Coin ki market value | ⏳ Sabse aakhri cheez |

**Galti ye hogi:** Stage 1 se seedha Stage 5-6 par koodna (listing aur price ke peeche bhaagna jab na use hai na user). Aise coins 88% time me fail hote hain (neeche §5 dekho).

---

## 3. Har stage par BUT users ko kya dega (concrete)

### Stage 1 — Testnet (pehla kadam)

Testnet matlab: **nakli network, asli code**. Coins free hote hain, koi asli paisa nahi lagta.

- **Faucet:** Ek website jahan koi bhi wallet connect karke **free test BUT** le sake. Jaise Ethereum ka Sepolia faucet 0.1 test ETH deta hai — waise hi BUT faucet thode test coins dega (rate limit ke saath: ek address, ek din me ek baar). Faucet ka kaam value dena nahi hai — **chain ko chhoone ka mauka dena** hai. User wallet banayega, transaction bhejega, fees dekhega — bina ek rupya lagaye.
- **Kya milega user ko:** Free test coins, ek kaam karta wallet, aur testnet par apna naam (leaderboard / contributor list).
- **Humko kya milega:** Log chain ko todenge, bugs milenge, performance pata chalega. Tanssi Network ka testnet campaign isi tareeke se 10x participation badha paya tha.

### Stage 2 — Pehla real use-case (sabse important stage)

**Ek chain tabhi jeeti hai jab us par koi kaam hota hai.** BUT ke pehle use-cases, order me:

1. **Payments (bhejna / paana):** Sabse pehla aur sabse simple kaam — ek wallet se doosre wallet ko BUT bhejna, kam fees par (proposal: 0.001 BUT per transaction). Ye tabhi kaam karega jab web wallet asli chain se jud jayega (Phase 3).
2. **Staking:** Coin ko lock karke network secure karna aur reward paana (proposal: minimum 100 BUT stake, block rewards). User ko milega: apne coins par reward + network ko secure karne me hissa.
3. **Prediction market (aage ka plan):** Polymarket se seekha — log future events par apni rai se kama sakte hain (USDC se settlement, oracle se result). Ye BUT Trust Contract ka future feature hai, abhi nahi ban raha. Iske liye oracle sabse mushkil hissa hai — pehle oracle design hoga, tabhi ye feature aayega.
4. **Device Lock wallet:** Sirf ek registered phone se transaction — ye BUT ki pehchan ban sakta hai (security feature jo bade chains me aam nahi hai).

**Rule:** Pehla use-case tabhi launch karna jab wo **sach me chale** — aadha-adhura feature launch karne se behtar hai ek poora feature launch karna.

### Stage 3 — Community (developers aur users)

Developers ke bina chain par apps nahi banti, aur apps ke bina users nahi aate. Dusre chains ne ye kaise kiya:

- **Solana:** Hackathons (Colosseum) → winners ko accelerator → $250,000 pre-seed investment + mentorship. Hackathon finish line nahi, audition hai.
- **Base (Coinbase):** Builder Rewards (har hafte 2 ETH), retroactive grants (1–5 ETH), aur "Base Batches" program. Creators ko bhi reward — Farcaster par promote karne walon ko inaam.
- **Arbitrum:** Pehle broad incentives (STIP, LTIPP), phir targeted program (DRIP) — sirf un protocols ko reward jo pehle se product-market fit dikha chuke hon. Audit ka kharcha bhi subsidize kiya ($10M ARB audit program).
- **Polygon:** Community grants program — 1 billion POL startups ko dene ka plan.

**BUT ke liye (jab chain taiyar ho):**
- Chhote grants: pehle 5–10 developers ko chhote kaam (tool, explorer, wallet feature) ke liye test coins + baad me mainnet coins.
- Hackathon: ek weekend ka online hackathon — BUT par koi app banao, best ko reward.
- Docs aur examples: developer ko 10 minute me pehli transaction bhejni aani chahiye. Agar docs mushkil hain, developer nahi aayega.

### Stage 4 — Mainnet

- Asli chain live hogi, asli coins honge.
- Genesis allocation tabhi final hogi jab Sanjay tokenomics approve karenge (docs/TOKENOMICS-DRAFT.md §8 — 7 faisle).
- **Mainnet se pehle zaroori:** independent security audit (kam se kam 2 firms), testnet par kai machines par nodes chalana, Red Team / Blue Team testing.

### Stage 5 — Listing (DEX pehle, CEX baad me)

- **DEX (Decentralized Exchange) pehle:** Koi bhi liquidity pool bana kar token list kar sakta hai — koi application nahi, sirf gas fees + pool me daalne ke liye coins aur stablecoins. Ye pehla market hota hai. **Par:** DEX par listing ka matlab value nahi — bina demand ke pool khaali rehta hai.
- **CoinGecko / CoinMarketCap:** DEX par trading shuru hone ke baad in sites par listing — free standard review hoti hai.
- **CEX (Centralized Exchange) baad me:** Binance, Coinbase jaise bade exchanges par listing ke liye chahiye: application, legal opinion, **security audit**, team KYC, community size, pehle se DEX par trading volume, market maker (jo 24/7 liquidity de). Bade exchange par listing ka kharcha $100,000 se $1,000,000+ tak ho sakta hai (fees + market making + deposits), aur time 2–6 mahine. **Binance khud kehta hai: listing khareedi nahi ja sakti, koi guarantee nahi hai.**
- **Order yaad rakho:** DEX → CoinGecko/CMC → mid-size CEX → bade CEX. Seedha bade CEX ke peeche bhaagna paise aur time ki barbaadi hai.

### Stage 6 — Value (aakhri cheez, pehli nahi)

Jab use hai, demand hai, liquidity hai — tab market khud price banata hai. Is stage par hum kuch "karte" nahi — hum sirf use aur trust banate rehte hain.

---

## 4. Airdrops — kya kaam karta hai, kya nahi

Airdrop matlab: free me coins baantna. Ye sabse zyada galat samjha jaane wala tool hai.

### Kya kaam karta hai

- **Retroactive airdrop (peeche dekh kar reward):** Jo log **pehle se** product use kar chuke hain, unhe baad me reward do. Uniswap (2020) ne aisa kiya — har user ko 400 UNI mile (~$1,200). Arbitrum (2023) ne 625,000 wallets ko 1 billion+ ARB diye. Hyperliquid ne 94,000 logon ko $1 billion+ ke tokens diye — bina marketing department ke, bina VC paisa liye. **Kyun kaam kiya?** Kyunki reward unhe mila jo pehle se asli user the — farmer ko pata hi nahi tha kya count hoga.
- **Ownership baantna:** Token bahut logon ke haath me ho to wo security jaisa nahi lagta, aur market banti hai (kuch bechenge, kuch kharidenge, price banna shuru hoga).

### Kya nahi kaam karta (research ke numbers)

- **88% airdropped tokens 3 mahine me price kho dete hain** (Keyrock ne 62 airdrops dekhe; DappRadar ne $20 billion ki distributions me wahi number paya).
- **~64% recipients airdrop milte hi bech dete hain** (2 million+ addresses ka analysis).
- **Mercenary users:** Sirf airdrop ke liye aaye log airdrop ke agle din chale jate hain — kai projects ka volume airdrop ke baad aadha ho gaya.
- **Sybil / farmers:** LayerZero ke airdrop me 6 million addresses me se sirf 4–6 lakh asli the — baaki nakli the. 803,093 addresses ko confirmed Sybil mana gaya. Log scripts se sau-sau wallets banate hain.
- **Testnet farmers:** Testnet par tokens free hote hain, isliye log product ke liye nahi, sirf airdrop ki ummeed me click karte hain. Testnet ke numbers par bharosa mat karo.

### BUT ke liye rule

1. **Airdrop tabhi, jab product pehle se chal raha ho** — pehle use, phir reward. Ulta mat karo.
2. **Retroactive hi karna** — pehle se announce mat karo ki "airdrop milega", warna sirf farmer aayenge.
3. **Sybil check zaroori** — wallet ki umar, transaction pattern, aur asli activity dekho; sirf wallet count mat gino.
4. **Vesting ke saath do** — ek saath saare coins mat do; dheere-dheere do, taaki bechne ka pressure ek din me na aaye.
5. **Airdrop marketing nahi hai** — ye user acquisition ka ek tool hai, poori strategy nahi.

---

## 5. Kya NAHI karna (galtiyan list)

Ye galtiyan hazaaron projects ko le doobi hain. BUT inhe nahi karega:

| # | Galti | Kyun khatarnak hai |
|---|---|---|
| 1 | **Value / price ki guarantee dena** | Koi bhi price ki guarantee nahi de sakta. Guarantee dene wala ya jhooth bol raha hai, ya scam kar raha hai. Guarantee dena kai deshon me illegal bhi ho sakta hai. |
| 2 | **Paid listing agents ko paise dena** | Binance ne December 2025 me saaf kaha: wo listing ke liye kisi agent ko appoint nahi karta, listing kabhi guarantee nahi hoti, aur listing application ke liye koi fee nahi leta. Jo "listing karwa dunga" keh kar paise maange, wo fraud hai. |
| 3 | **Fake volume / wash trading** | Ek wallet khud se khareed-bech kar volume dikhata hai. Bitquery ne paya: ek Solana wallet ne $12,000 se $1.4 billion ka suspected wash trading kiya. Pakde jaane par exchange delist kar deta hai aur bharosa khatm ho jata hai. |
| 4 | **Bina legal advice ke coin bechna / ICO** | Coin bechna securities law ke andar aa sakta hai. Bina vakeel ke public sale mat karo. India me bhi (neeche §6 dekho). |
| 5 | **Fake partnerships / fake audit ka daava** | Helium ke developer par $200,000 ka SEC penalty laga — jhootha daava kiya tha ki badi companies uska network use karti hain. Partnership tabhi asli hai jab dono taraf se confirm ho. |
| 6 | **Influencers ko paise dekar shill karwana** | Paid promotion bina disclose kiye kai jagah illegal hai. Aur paid hype se aaye users airdrop farmers jaise hi hote hain — aate hain, bechte hain, chale jate hain. |
| 7 | **Team ke coins chhupana / vesting na karna** | Agar team ke paas bahut coins hon aur wo ek saath bech dein, price gir jayega aur bharosa khatm. Team allocation (proposal: 15%, 4 saal vesting) public honi chahiye. |
| 8 | **"Bitcoin se behtar" jaise daave karna** | Bitcoin ki taakat uska network hai, sirf code nahi. Aise daave par koi bharosa nahi karta — aur ye galat bhi hai. |
| 9 | **Testnet numbers ko success samajhna** | Testnet par free tokens hain — wahan ki activity asli demand nahi hai. Testnet ke user count se khush mat ho. |
| 10 | **Liquidity lock kiye bina DEX pool banana** | Agar liquidity lock nahi hai, to pool se paise nikaale ja sakte hain (rug pull). Chainalysis ke hisaab se 2024 me launch hue tokens me se ~94% suspicious DEX pools ko unke creators ne hi rug kiya. |

---

## 6. India context — legal aur tax (seedhi baat)

> **Ye legal advice nahi hai.** Ye sirf public information ka summary hai. Coin bechne ya koi bhi financial kadam uthane se pehle **crypto vakeel se salah lena zaroori hai.**

- **Crypto India me ban nahi hai, par legal tender bhi nahi hai.** Aap crypto khareed/sambhal/bech sakte hain — par koi aapko use payment ke roop me lene par majboor nahi kar sakta.
- **Tax (FY 2025-26):** Crypto gains (Virtual Digital Assets) par **flat 30% tax** + cess, aur har transfer par **1% TDS**. Loss ko doosri income se set-off nahi kar sakte, aur loss aage carry forward nahi hota. Sirf khareedne ki keemat (cost of acquisition) deduct hoti hai.
- **FIU-IND registration:** March 2023 se jo bhi business crypto ko rupees me badle, ek coin ko doosre se badle, ya customers ke liye crypto rakhe/transfer kare, use **FIU-IND me register** hona padta hai (PMLA ke tahat, AML rules ke saath). Bina registration ke exchange chalana band ho sakta hai — sarkar ne 53 providers ke apps/URLs band karwaye hain.
- **Koi dedicated crypto law nahi hai** (2026 tak). Parliamentary committee ne 2026 me study ki, par koi bill nahi aaya. Matlab: rules badal sakte hain — isliye legal advice ke bina koi bada kadam mat uthao.
- **BUT ke liye iska matlab:**
  - **Coin sale / ICO bina legal advice ke bilkul nahi.**
  - Testnet ke free test coins me koi tax/legal issue nahi hai (unke paas koi asli value nahi hoti).
  - Mainnet aur listing se pehle vakeel se ye poochhna hoga: token utility token hai ya security jaisa? FIU registration chahiye ya nahi? Tax kaise handle hoga?
  - **Budget:** Legal fees ko plan me pehle se rakho — Tier 1 listing ke liye legal documentation ka kharcha $50,000–$200,000 tak ho sakta hai.

---

## 7. Agle concrete steps (order ke saath)

| # | Step | Kab | Kya chahiye |
|---|---|---|---|
| 1 | **Tokenomics final karo** | Abhi | Sanjay ke 7 faisle (docs/TOKENOMICS-DRAFT.md §8) — supply, baant, reward, stake, fees |
| 2 | **Phase 3: Web wallet ko API se jodo** | Uske baad | Fake balance/ledger hatakar asli chain se jodna (docs/REAL-NETWORK-PLAN.md §3 wiring) |
| 3 | **Stake-locking + block reward** | Phase 3 ke saath | Stake ke coins chain par lock hon, unbonding period |
| 4 | **Independent audit** | Web wallet judne ke baad | Kam se kam 2 firms, testnet se pehle |
| 5 | **Public testnet + faucet launch karo** | Audit ke baad | Public testnet, faucet website, docs, explorer |
| 6 | **Pehla use-case live karo** | Testnet par | Payments + staking, test coins se |
| 7 | **Community shuru karo** | Testnet ke saath | X/Telegram/Discord, developer docs, pehle 10 builders ko chhote grants |
| 8 | **Hackathon** | Testnet ke 2–3 mahine baad | Online hackathon, BUT par app banao, winners ko reward |
| 9 | **Legal advice lo** | Mainnet se pehle | Vakeel se token classification, FIU, tax — **bina iske mainnet nahi** |
| 10 | **Mainnet launch** | Sab taiyar hone par | Genesis allocation final, validators taiyar |
| 11 | **DEX listing** | Mainnet ke baad | Liquidity pool (locked), CoinGecko/CMC listing |
| 12 | **CEX listing** | Demand dikhne ke baad | Application, audit report, market maker — listing khareedi nahi jaati |

**Sabse pehla step (aaj):** Sanjay tokenomics ke 7 faisle lein. Uske bina testnet ka genesis bhi nahi ban sakta.

---

## 8. Sanjay ke sawal ka seedha jawab

**"Abhi koi user nahi hai, coin ki koi value nahi hai — koi aaye to hum use kya denge?"**

Aaj ke din ka sach ye hai: **aaj hum user ko value nahi de sakte, kyunki value abhi bani hi nahi hai.** Jo is stage par value ka vaada kare, wo jhooth bol raha hai.

Lekin hum user ko ye de sakte hain — aur yehi sahi order hai:

1. **Aaj:** Ek kaam karta real chain (Phase 1+2 taiyar hai), aur imaandari — jo bana hai wo bana hai, jo baaki hai wo baaki hai.
2. **Testnet par:** Free test coins, ek kaam karta wallet, aur chain ko todne/seekhne ka mauka.
3. **Pehle use-case par:** Coin se kuch **kar** sakna — bhejna, stake karna, reward paana.
4. **Uske baad:** Community, apps, mainnet, listing — aur tab market khud value banayega.

**Value pehle nahi aati. Use pehle aata hai, value baad me.**

---

## Sources (research, 7 Nov 2026)

- Testnet/faucet: HashLock "Testnet Explained"; Whales Market "Crypto Faucet Explained"; Tanssi "Let's Forking Dance" testnet campaign (AirLyft case study).
- Airdrops: Keyrock analysis (62 airdrops, 88% price decline); DappRadar ($20B distributions, 88% lose value in 3 months); LayerZero Sybil case (6M addresses, 400–600k real, 803,093 confirmed Sybils); Hyperliquid $1B+ airdrop to ~94,000 users; Arbitrum 2023 airdrop to 625,000 wallets; Uniswap 400 UNI.
- Grants/hackathons: Solana Colosseum ($250k pre-seed); Base Builder Rewards/Grants; Arbitrum STIP/LTIPP/DRIP + $10M audit program; Polygon community grants (1B POL); Optimism Retro Funding.
- Listing: Spark "Token Listing"; Lunar Strategy "Crypto Exchange Listing" (listing ladder: DEX → CoinGecko/CMC → CEX); Crypto.news "How CEX listings work" (Tier 1: $100k–$1M+, 2–6 months; legal $50k–$200k); Binance statement Dec 2025 (no listing guarantee, no appointed agents).
- Scams/risks: Chainalysis 2026 Crypto Crime Report (~$17B scam proceeds in 2025; 74,037 suspected pump-and-dump tokens of 2.06M launched in 2024; ~94% suspicious DEX pools rugged); Bitquery wash trading ($1.4B suspected, one Solana wallet); Helium SEC penalty $200,000.
- India: Income Tax Act §115BBH (30% flat tax) & §194S (1% TDS); FIU-IND/PMLA registration (March 2023); Lok Sabha reply 30 March 2026 (54 registered VDA providers, 53 takedowns); Finance Bill 2026 penalties (₹200/day non-filing, ₹50,000 inaccurate reporting).
- Value/utility: C-Sharp Corner "Where Does the Value of Crypto Actually Come From"; Motley Fool "What Gives Bitcoin Value".

*Ye document koi investment advice nahi hai, aur koi value/price ki guarantee nahi deta.*
