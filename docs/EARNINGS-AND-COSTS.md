# BUT Network — Earning aur Kharcha

> **Sanjay ka sawal (7 Nov 2026):** "Main kitna earn kar sakta hun aur kahan se meri earning hogi?"
>
> **Seedha jawab, ek line me:** Abhi **₹0** — aur pehle 1–2 saal kharcha hi kharcha hai.
> Is document me koi earning number **promise nahi** kiya gaya hai. Jo
> "agar" examples hain, wo sirf samjhane ke liye hain — unhe vaada mat
> samjho. Coin ki value market banata hai, hum nahi.
>
> Date: 7 November 2026. Status: **PLAN ONLY** — kuch launch nahi hua
> hai, koi user nahi hai, koi listing nahi hai, koi sale nahi hui hai.
>
> Note: $ ko ₹ me badalne ke liye $1 ≈ ₹88 mana hai (andaaza).
> Ye document investment advice ya legal advice nahi hai.

---

## 1. Abhi earning ZERO kyun hai — seedhi baat

Earning tabhi hoti hai jab koi cheez **bik** sake ya **use** ho. Aaj:

- BUT coin kisi exchange par nahi hai — use khareedne-bechne ki jagah hi nahi hai.
- Koi user nahi hai — coin se kuch karne wala hi nahi hai.
- Web wallet abhi bhi demo hai, mainnet nahi aaya, audit nahi hua.

**Isliye aaj Sanjay ki earning ₹0 hai — aur ye normal hai.** Bitcoin ke
pehle 1–2 saal me bhi uski keemat lagbhag zero thi. Earning pehle nahi
aati — pehle chain banti hai, phir use banta hai, phir value, aur
sabse aakhri me earning.

**Pehle kharcha aayega, earning baad me.** Ye order ulta nahi ho sakta.

---

## 2. Future me earning kahan se aa sakti hai (agar network chala to)

Earning ke 5 sources hain. **"Agar" par dhyaan do — ye tabhi honge jab
network chalega, users aayenge aur coin ki value banegi.** Koi bhi source
guarantee nahi hai.

### Source 1 — Team allocation: 1% (10,000,000 BUT) + har block reward ka 0.1%

Sanjay ka FINAL faisla (2026-11-07, report padhne ke baad badla — pehle
10% tha): team ke paas total supply ka **1%** rahega
(docs/TOKENOMICS-DRAFT.md §3) — matlab **10,000,000 BUT**. Iske alawa
**har block reward ka 0.1%** team ke account me jayega (ongoing funding).

- Ye Sanjay ki sabse badi earning ka source ho sakta hai — **lekin sirf
  kagaz par**, jab tak coin ki keemat na bane.
- **Vesting zaroori hai:** Industry ka norm hai — team tokens **12 mahine
  ke cliff + 4 saal linear vesting** (matlab pehle saal kuch nahi, phir
  har mahine thoda-thoda). Bina vesting ke team ek saath bech sakti hai —
  ICP me Foundation par vesting nahi thi aur token 95% gira tha.
- Sanjay ke faisle me vesting abhi tay nahi hui hai — **ye jald decide
  karna chahiye** (sifarish: 12-month cliff + 4 saal linear, 0% TGE par).
- **Bechne par kya hoga:** 10M BUT bechne ke liye utne kharidne wale
  chahiye. Agar liquidity kam hai to bechte hi price gir jayega. Isliye
  team tokens **dheere-dheere** beche jate hain, ek saath nahi.
- **Tax (India):** Bechne par profit par **flat 30% tax + 1% TDS**
  lagega. Loss ko doosri income se adjust nahi kar sakte.

### Source 2 — Validator rewards (staking + block rewards)

Sanjay apna validator node chala sakte hain — jo coins stake karenge,
un par reward milega.

- Reward kahan se: **block rewards** (23% rewards pool se) + **transaction
  fees** (jo block banane wale ko milti hain).
- Reward ka hisaab: `(aapka stake ÷ kul stake) × kul rewards`. Jitna
  zyada stake, utna zyada reward — par utna zyada paisa lock bhi hoga.
- **Dusri chains ke APY (sirf examples, 2026 ke snapshots):**

| Chain | Nominal APY | Inflation | Asli yield (APY − inflation) |
|---|---|---|---|
| Ethereum | ~2.8–3.8% | ~0.5% | ~3% |
| Solana | ~6–8% | ~5% | ~1–3% |
| Cosmos | ~14–21% | ~10–14% | ~2–8% |
| Polkadot | ~10–15% | ~11% | ~4% |
| Avalanche | ~7–8.5% | — | ~7% |

- **Sabse important sabak: headline APY jhooth bolta hai.** Cosmos 19%
  deta hai par 12% naye coins bhi banata hai — asli fayda sirf ~7% hai.
  Ethereum ka 3% chhota lagta hai par inflation lagbhag zero hai, isliye
  asli yield sabse zyada hai. **BUT ke liye bhi yahi rule hoga: nominal
  APY minus inflation = asli earning.**
- BUT ka block reward abhi **demand-based** hai (formula pending) — isliye
  BUT ka APY abhi bataya hi nahi ja sakta. Formula final hone par hi
  number aayega.

### Source 3 — Fees ka hissa (endgame model)

Sanjay ka FINAL faisla (§9): jab poori supply release ho jayegi to fees
**ek main wallet me ikatthi** hongi; usse rewards dheere-dheere validators
ko jayenge aur thoda hissa team ko batega.

- Ye Bitcoin ke model jaisa hai — 2140 ke baad Bitcoin me bhi sirf fees
  se miners ko milega.
- **Par dhyaan do:** Fees se earning tabhi banti hai jab **karodon
  transactions** hon. Neeche §3 me hisaab dekho — kam fee ka matlab kam
  revenue hai.
- Main wallet ka control abhi tay nahi hua hai — multisig + timelock se
  hona chahiye (TOKENOMICS-DRAFT.md §10, sifarish 8). Agar main wallet
  kisi ek insaan ke control me hai to wo hack aur centralization ka risk
  hai (Ronin bridge: 5-of-9 multisig hack, $624M chori).

### Source 4 — Listing se paise? NAHI — listing me paise LAGTE hain

Galatfehmi door kar do: **exchange par list hone se founder ko paise
nahi milte — paise lagte hain.**

- Binance, Coinbase, Kraken kehte hain "listing fee nahi lete" — par
  security deposit, airdrop ke liye tokens (Binance HODLer Airdrops me
  median **1.5% of supply**), market maker aur liquidity par kharcha
  hota hai.
- Ek founder ne October 2025 me bataya tha ki Binance ne ~8% supply +
  $2M deposit manga tha (Binance ne mana kiya). Sach jo bhi ho, ye
  samjho: **listing free nahi hoti.**
- Aur listing ka matlab value nahi: Empirica ne 500+ Binance listings
  dekhi — tokens listing ke **3 mahine me average 22.66% gire**, aur
  **80% se zyada tokens 6 mahine me value kho baithe**. Listing sirf
  kharidne-bechne ki jagah deti hai, demand nahi banati.

### Source 5 — BUT par banne wali apps/services (sabse door ka source)

Jab chain par apps banengi (wallet, prediction market, payments), to unki
fees se bhi earning ho sakti hai — jaise Solana par apps ne Q3 2026 me
memecoin trading se zyada kamaaya. **Par ye sabse aakhri stage hai** —
pehle chain, phir users, phir apps, phir inki earning.

---

## 3. Fee revenue ka hisaab — kam fee ka matlab kam revenue

Sanjay ka faisla: fees Solana-jaisi bilkul kam, bits me kategi.
1 BUT = 1,000 bits (FINAL). Maan lo fee = **1 bit (0.001 BUT)** per
transaction:

| Roz ke transactions | Fees (BUT/roz) | Fees (BUT/mahina) | ₹1/BUT par | ₹10/BUT par |
|---|---|---|---|---|
| 10,000 | 10 | 300 | ₹300 | ₹3,000 |
| 1,00,000 | 100 | 3,000 | ₹3,000 | ₹30,000 |
| 10,00,000 | 1,000 | 30,000 | ₹30,000 | ₹3,00,000 |
| 1,00,00,000 | 10,000 | 3,00,000 | ₹3,00,000 | ₹30,00,000 |

**Ye poore network ki kul fee income hai** — ye saare validators me
batni hai, sirf Sanjay ko nahi milegi.

Tulna ke liye — Solana (Sept 2026): ~157 million user transactions
roz, phir bhi fees se sirf ~$25 million mahina. Median fee sirf ~$0.00043
hai. Aur Solana ki revenue ka ~95% priority fees + tips se aata hai,
base fee se nahi.

**Seedhi baat:** Kam fee achhi baat hai (users aate hain), par fees se
earning tabhi banti hai jab **roz karodon transactions** hon. 10 lakh
transactions par bhi fee income sirf ₹30,000/mahina hai (₹1/BUT par).
Isliye fees ko earning ka main source mat samjho — **team allocation aur
staking rewards hi main sources hain**.

---

## 4. "Agar" examples — team ke 1% ki kagazi value

> **YE SIRF SAMJHANE KE LIYE HAIN. YE VAADA NAHI HAI. Ye tabhi hoga
> jab 1 BUT ki keemat itni bane — aur keemat banegi ya nahi, koi nahi
> janta.**

Team allocation = 10,000,000 BUT (1% — pehle 10% tha, Sanjay ne
2026-11-07 ko 1% kar diya):

| Agar 1 BUT ki keemat | Team ke 10M BUT ki kagazi value | 4-saal vesting par saal bhar me unlock |
|---|---|---|
| ₹0.10 | ₹10 lakh | 2.5M BUT = ₹2.5 lakh |
| ₹1 | ₹1 crore | 2.5M BUT = ₹25 lakh |
| ₹10 | ₹10 crore | 2.5M BUT = ₹2.5 crore |
| ₹100 | ₹100 crore | 2.5M BUT = ₹25 crore |

**Par ye kagazi value hai, cash nahi.** Cash tab milega jab:

1. Itne BUT beche jayein — aur utne kharidne wale hon (liquidity chahiye).
2. Bechte waqt price na gire — 2.5M BUT ek saath bechne par price gir
   sakta hai, isliye dheere-dheere bechna padta hai.
3. Tax katne ke baad — profit par **30% tax** lagega. Jaise ₹25 lakh
   ka profit ho to ~₹7.5 lakh tax, haath me ~₹17.5 lakh (aur 1% TDS
   alag se katega).

> Note: Iske alawa har block reward ka **0.1%** team account me jayega
> — ye ongoing income hai, upar wali kagazi value se alag.

**Validator rewards ka ek "agar" example:** Maan lo released supply
100M BUT hai, usme se 50M staked hai, aur saal bhar me rewards pool se
5M BUT baante gaye — to APY = 5M ÷ 50M = **10% nominal**. Agar Sanjay
khud 1M BUT stake karein to saal bhar me ~1,00,000 BUT reward (nominal).
Asli yield isse kam hogi — inflation minus karke (upar §2 dekho).
**Ye sirf ek example hai — BUT ka reward formula abhi bana hi nahi hai.**

---

## 5. Pehle kharcha kitna aayega (earning se pehle)

Earning se pehle ye kharcha aayega. Ye ranges research se hain (2025–26):

| Kharcha | Kitna (andaaza) | Kab |
|---|---|---|
| **Independent audit × 2 firms** | $30,000–$200,000+ (₹26 lakh – ₹1.76 crore+) | Testnet se pehle |
| **Legal advice (India)** | ₹2–10 lakh | Mainnet se pehle |
| **Legal docs (listing ke liye)** | $50,000–$200,000 (₹44 lakh – ₹1.76 crore) | Listing se pehle |
| **Servers — testnet + mainnet nodes (3–5 nodes, 1 saal)** | ₹1–5 lakh | Shuru se |
| **Explorer, faucet, website** | ₹1–3 lakh (ek baar) | Testnet par |
| **Market maker (pehla saal)** | $100,000–$1,000,000+ (₹88 lakh – ₹8.8 crore+) | Listing par |
| **Listing deposits / allocations** | Alag-alag; deposit aksar refundable | Listing par |
| **Marketing / community (optional)** | ₹5–20 lakh/saal (agency ho to) | Testnet ke baad |
| **Bug bounty reserve** | $10,000–$50,000 (₹9–44 lakh) | Mainnet se pehle |

**Kul milakar (listing se pehle, minimum):** lagbhag **₹30–50 lakh**
(audit + legal + servers + bounty).
**Listing ke saath:** **₹1–10 crore+** ho sakta hai.

Audit cost ke benchmarks: simple token contract $5K–$15K; mid DeFi
$20K–$80K; complex $80K–$500K+. Trail of Bits $80K–$200K+;
CertiK $10K–$150K+. BUT ek poori chain hai — isliye 2 audits ka
budget upar wale range me rakho.

Server kharcha: BUT ek chhoti chain hai — ek aam VPS (₹2,000–₹8,000/
mahina per node) kaafi hai. Solana jaisi badi chain ke validator ka
kharcha isse 10–50 guna zyada hota hai ($1,280–$5,120/mahina hosting +
~1 SOL/roz vote cost) — BUT ko wo scale nahi chahiye.

---

## 6. Realistic timeline — kab tak earning shuru ho sakti hai

| Kab | Stage | Earning |
|---|---|---|
| Abhi – 6 mahine | Build + testnet | **₹0** (sirf kharcha) |
| 6–12 mahine | Audit + testnet + community | **₹0** (sirf kharcha) |
| 12–24 mahine | Mainnet + pehla use-case | **~₹0** (users abhi kam) |
| 2–3 saal | Listing (DEX, phir CEX) | **Pehli baar kuch value ban sakti hai** — guarantee nahi |
| 3–4 saal | Team vesting unlock shuru | Team tokens dheere-dheere bechne layak |
| 5+ saal | Agar network chal gaya | Validator rewards + fees se regular earning |

**Ye timeline bhi optimistic hai.** Bahut se chains 3–4 saal me bhi
earning tak nahi pahunchte. Aur kuch kabhi nahi pahunchte.

---

## 7. Kya NAHI karna (earning ke naam par galtiyan)

| # | Galti | Kyun khatarnak hai |
|---|---|---|
| 1 | **Team tokens jaldi bechna** | Price gir jayega, bharosa khatm. ICP me vesting nahi thi, token 95% gira. |
| 2 | **Price / returns ki guarantee dena** | Koi guarantee nahi de sakta. Kai deshon me illegal bhi hai. |
| 3 | **"Guaranteed returns" wale vaade** | Ye scam ka nishan hai — aisa vaada karne wale par case ho sakta hai. |
| 4 | **Paid listing agents ko paise dena** | Binance ne Dec 2025 me saaf kaha: wo kisi agent ko appoint nahi karta, listing kabhi guarantee nahi hoti. "Listing karwa dunga" keh kar paise maange to fraud hai. |
| 5 | **Fake volume / wash trading** | Ek wallet khud se khareed-bech kar volume dikhata hai. Pakde jaane par delist + bharosa khatm. |
| 6 | **Loan + call option wala market maker** | Shuru me "free" lagta hai, par token ki keemat badhi to option ki keemat karodon me pahunch jati hai (example: strike $0.50, price $5 → MM ko $4.50/token). |
| 7 | **Bina vakeel ke coin bechna / ICO** | Securities law ke andar aa sakta hai. India me bhi legal advice zaroori hai. |
| 8 | **Kagazi value ko cash samajhna** | 10M BUT ki "value" tabhi cash hai jab koi use khareede. Bina liquidity ke kagazi value ka koi matlab nahi. |

---

## 8. India — tax aur legal (seedhi baat)

> **Ye legal/tax advice nahi hai.** Koi bhi bada kadam uthane se pehle
> crypto vakeel / CA se salah lena zaroori hai.

- Crypto gains par **flat 30% tax + cess**; har transfer par **1% TDS**.
- Loss ko doosri income se adjust nahi kar sakte, loss aage carry
  forward nahi hota.
- Team tokens milne aur bechne — dono par tax ke sawal hain; kab tax
  lagega (milne par ya bechne par) ye vakeel/CA se poochhna hoga.
- FIU-IND registration: agar business crypto ko rupees me badle ya
  customers ke liye crypto rakhe/transfer kare to registration zaroori
  hai.

---

## 9. Sanjay ke sawal ka seedha jawab

**"Main kitna earn kar sakta hun aur kahan se meri earning hogi?"**

- **Aaj: ₹0.** Aur pehle 1–2 saal sirf kharcha hai (₹30–50 lakh minimum).
- **Future sources (agar network chala to):** team ke 1% tokens
  (10M BUT) + har block reward ka 0.1%, apna validator chalakar staking rewards, aur endgame me
  fees ka hissa.
- **Kitna? Koi number nahi de sakta.** Agar 1 BUT = ₹1 bane to team ke
  10M BUT kagaz par ₹1 crore honge; agar ₹0.10 bane to ₹10 lakh;
  agar coin hi na chala to ₹0. **Keemat market tay karega.**
- **Sabse pehle:** chain poori karo, audit karwao, testnet lao, users
  lao — earning uske baad aayegi, pehle nahi.

**Pehle kharcha, phir kaam, phir users, phir value, aur sabse aakhri
me earning.**

---

## Sources (research, 7 Nov 2026)

- Audit costs: CertiK alternatives comparison (Trail of Bits $80K–$200K+, CertiK $10K–$150K+, established firms $40K–$100K); Smart Contract Audit Guide 2025 (simple $5K–$15K, mid $20K–$80K, complex $80K–$500K+); Spark audit firm comparison.
- Staking APY: altcoininvestor.com staking guides (ETH 2.8–3.8%, SOL 6–8%, Cosmos 18.5–21%, real yield = nominal − inflation); Hivelocity validator economics 2026; gov.capital staking table.
- Listing: Lunar Strategy "How to Get Your Token Listed" (Binance/Coinbase/Kraken no listing fee, deposits/allocations may apply); Flexe.io Binance listing 2026 (HODLer Airdrops median 1.5% of supply); AnkhLabs (Empirica: −22.66% in 3 months after Binance listing); The Defiant / InnMind (Limitless 8% + $2M deposit claim, Oct 2025); USERS-AND-VALUE-PLAN.md §5 (Tier-1 listing $100K–$1M+, legal $50K–$200K).
- Market maker: thesignal.directory (retainer $5K–$50K/month, first-year $100K–$1M+); giantslabs.pro (loan + call option danger, example $4.5M option cost); spark.money (GSR $100K setup + $20K/month); The Block Research (median first-year cost across two tier-2 exchanges ~$280K).
- Team vesting: Streamflow token allocation guide (team 15–20%, 12-month cliff + 3–4 years linear, 0% at TGE); findas.org Token Vesting 101; tokenomics.net (team 10–20%, insider total <50%).
- Fee revenue: Blockworks Solana staking economy (priority fees ~73% of validator revenue; stakers ~$4.56M/day from issuance); tradingstrategies.work (Solana Sept 2026: 157.3M user tx/day, ~$25M/month fees, $0.0063/tx); Bybit Solana revenue guide (median fee ~$0.00043).
- Node costs: 7blocklabs (Solana validator hosting $1,280–$5,120/month; vote ~1.1 SOL/day); Cherry Servers Solana node cost; tatums.co (Ethereum node $500–$2,000/month).
- India tax: Income Tax Act §115BBH (30% flat) & §194S (1% TDS); FIU-IND/PMLA (March 2023) — see USERS-AND-VALUE-PLAN.md §6.

*Ye document koi investment advice nahi hai, aur koi earning/price ki guarantee nahi deta.*
