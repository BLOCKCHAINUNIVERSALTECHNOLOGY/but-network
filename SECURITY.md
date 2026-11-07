# SECURITY.md — BUT Wallet Demo (सुरक्षा नीति)

> Simple Hindi me likha hai, taaki koi bhi aasani se samajh sake.
> Written in simple Hindi, with simple English headings.

## 1. Ye demo kya HAI? / What this demo IS

- Ye **BUT Wallet Demo** hai. Ye sirf seekhne ke liye hai.
- Isme Hot Mode aur Cold Mode ka demo hai.
- Ye browser me chalta hai. Iska data aapke browser ke `localStorage` me rehta hai.
- Isme naya `shared/crypto-secure.ts` module hai. Ye module sahi tareeke se random number banata hai (CSPRNG), SHA-256 hash karta hai, aur demo data ko AES-GCM se encrypt kar sakta hai.
- Ye module demo ko thoda safe banata hai. Par ye demo ko asli wallet nahi banata.

## 2. Ye demo kya NAHI hai? / What this demo IS NOT

- Ye **asli cryptocurrency ya asli blockchain NAHI hai**.
- Isme dikhne wala balance aur ledger **nakli hai**. Ye asli paisa nahi hai.
- Ye asli payment, asli network, asli fund nahi sambhalta.
- Ye **production wallet NAHI hai**. Ise asli paise ke liye kabhi use mat karo.
- Isme jo keypair banta hai, wo sirf demo ke liye hai. Wo production key nahi hai.
- Naya secure module hone ka matlab ye nahi hai ki ye ab production-safe hai. Ye abhi bhi demo hai.

## 3. Isme kya NAHI dalna hai? / What NOT to put in this demo

- Asli seed phrase kabhi mat dalo.
- Asli private key kabhi mat dalo.
- Asli wallet key kabhi mat dalo.
- Asli coin ya asli paisa kabhi mat dalo.
- Kisi ki asli private jaankari (password, bank detail, ID) kabhi mat dalo.

Agar aapne galti se kuch asli daal diya hai, to use turant hatao. Browser ka localStorage clear karo aur demo ko reset karo.

## 4. Cold Mode ki limit / Cold Mode limits

- Cold Mode sirf screen par details chhupata hai.
- Cold Mode asli cryptographic privacy **NAHI deta**.
- Cold Mode me bhi demo data browser ke localStorage me padha ja sakta hai.
- `shared/crypto-secure.ts` se encrypt kiya hua demo data bina password ke nahi khulta. Par password kamzor hoga, to data bhi kamzor hoga.
- Apna demo password kisi ko mat batao. Password ko code me mat likho.

## 5. Kamzori mile to kya karein? / How to report a vulnerability

Agar aapko is demo me koi security kamzori mile, to please use public me mat post karo.

**Private tareeke se report karo:**

1. GitHub par is repository me **Private vulnerability report** kholo (Security tab → Report a vulnerability). Ye sabse achha tareeka hai.
2. Ya repository ke owner ko GitHub par message karo. Owner: **BLOCKCHAINUNIVERSALTECHNOLOGY**.
3. Report me ye likho: kamzori kya hai, kahan hai (file aur line), aur use kaise dobara kiya ja sakta hai (steps). Koi asli key ya asli data report me mat dalo.

**Hum kya karenge:**

- Hum report ko padhenge aur check karenge.
- Agar kamzori sahi hai, to hum use fix karenge.
- Fix hone tak please us kamzori ko public mat karo.

**Koi bug bounty nahi hai.** Is demo ke liye koi inaam ya payment nahi hai. Ye ek seekhne wala demo hai.

## 6. Kya scope me hai? / Scope

**Scope me hai:**

- Is repository ka code (`client/`, `server/`, `shared/`).
- Demo ka localStorage data aur Cold Mode ka behave.
- Naya `shared/crypto-secure.ts` module.

**Scope me NAHI hai:**

- Kisi aur website ya app ki kamzori.
- Kisi aur ke account ya wallet ki kamzori.
- Social engineering (kisi ko dhokha dekar password lena).
- Bina permission ke kisi ke system ko test karna.

## 7. Safe use ke niyam / Safe use rules

- Sirf demo data use karo. Nakli balance aur nakli address hi use karo.
- `Math.random()` ko kabhi key ya password ke liye use mat karo. Hamesha `shared/crypto-secure.ts` wala secure random use karo.
- Demo password strong rakho. Kam se kam 12 character ka password rakho.
- Apna kaam karne ke baad demo ko reset karo.

## 8. Versions

- Ye policy `main` branch ke liye hai.
- Purane version ke liye alag se report karne ki zaroorat nahi hai. Latest `main` par check karo.

---

*Ye demo seekhne ke liye hai. Asli paise aur asli key ke liye kabhi is demo ka use mat karo.*
*This demo is for learning only. Never use it for real money or real keys.*
