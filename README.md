# BUT Wallet

BUT Network ka wallet — local Rust chain (`chain/`, `but-node`) se juda hua.
Ye **non-custodial wallet** hai: private key sirf user ke device par rehti
hai, server par kabhi nahi jati. Balance aur lenden chain se aate hain —
browser me koi fake balance ya fake ledger nahi hai.

## Wallet v1 me kya hai (Sanjay ka FINAL scope)

1. **Seed backup + verify** — wallet banate waqt 24 words (real BIP39)
   dikhte hain, user se verify karwaye jate hain.
2. **Password unlock + biometric (WebAuthn PRF) + auto-lock (5 minute)** —
   vault AES-GCM se encrypt hokar localStorage me rehta hai.
3. **Bhejne se pehle saaf preview** — kis ko, kitna, fee kitni — phir confirm.
4. **BUT Pay Send/Receive + QR** — Receive par QR code (address/ID),
   Send par QR scan ya BUT Pay ID/handle (jaise `100001` ya `@sanjay`).
5. **Scam warning** — pehli baar bhejna, badi rakam, galat address par warning.

Baad me: swap, staking, hardware wallet, dApp browser, WalletConnect, NFT.

## इसे चलाएँ

1. GitHub पेज पर **Code → Download ZIP** चुनें, या repository को clone करें।
2. [Node.js](https://nodejs.org/) और pnpm उपलब्ध होने पर प्रोजेक्ट फ़ोल्डर में `pnpm install` चलाएँ।
3. **Pehle chain node chalao** (doosra terminal): `cd chain && cargo run -- serve`
   — local API `http://127.0.0.1:3030` par chalegi. Bina node ke balance/history
   nahi dikhenge (wallet banana aur unlock karna phir bhi chalega)।
4. `pnpm dev:static` चलाएँ और टर्मिनल में दिखा स्थानीय पता ब्राउज़र में खोलें।

वैकल्पिक रूप से `pnpm check` TypeScript जाँचता है, `pnpm test` उपलब्ध tests चलाता है, और `pnpm build:static` static site बनाता है।

## Units

Sabse chhoti unit **"bits"** hai — **1 BUT = 1,000 bits** (Sanjay ka final
faisla)। Fee default **1 bit** (0.001 BUT) hai।

## सुरक्षा और प्राइवेसी की अहम सीमा — imaandari se

- Seed phrase **kagaz par likho** — photo, cloud ya chat me kabhi nahi।
  Jiske paas seed hai, uske paas paise hain।
- Password (kam se kam 12 akshar) kahin save nahi hota — sirf tumhari
  memory me rehta hai। Password bhool gaye to wallet sirf seed phrase se
  wapas milega।
- Web vault AES-GCM se encrypt hota hai, par ye **hardware-backed storage
  nahi hai** — asli phone me hardware-backed storage ke liye asli Android
  app chahiye (abhi skeleton hai, APK nahi bana)।
- **Koi independent security audit abhi nahi hua hai।** Is wallet me asli
  paisa tabhi rakho jab audit ho jaye। Ye prototype निवेश, भुगतान ya
  production उपयोग ke liye abhi taiyar nahi hai।
- Node API sirf localhost par chalti hai — internet par khula node mat
  chhodo (koi auth nahi hai)।
- Scam warning sirf local rules hain (pehli baar, badi rakam) — v1 me koi
  central scam list nahi hai।

पूरी सुरक्षा नीति के लिए `SECURITY.md` पढ़ें। उसमें लिखा है कि क्या नहीं
डालना है, और कोई कमज़ोरी मिले तो उसे private तरीके से कैसे report करें।

**Wallet modules:** `shared/wallet-secure.ts` me real BIP39 24-word mnemonic
se Ed25519 keypair banta hai (wahi derivation jo Rust core me hai —
`chain/src/wallet.rs`)। Signing `shared/but-tx.ts` me vetted `tweetnacl`
library se hoti hai। Vault AES-GCM + PBKDF2 (210,000 iterations, slow mode
~2 minute) se encrypt hota hai।

**F-Droid:** F-Droid par launch ke liye checklist dekho — `docs/FDROID-CHECKLIST.md`। License taiyar hai — project Apache License 2.0 ke under hai (`LICENSE` dekho)। Abhi Android app nahi hai, isliye F-Droid launch abhi possible nahi hai।

## ब्लॉक डेटा का आकार

Lossless compression bytes को कम जगह में रखता है, लेकिन इतिहास मिटाता नहीं है। Pruning के बाद कोई नोड अपनी स्थानीय कॉपी से पुराने ब्लॉक विवरण हटा सकता है; नेटवर्क के archive nodes इतिहास उपलब्ध रख सकते हैं। Snapshot नए नोड को वर्तमान state से sync करने में मदद कर सकता है, पर अपने-आप हर पुराना transaction उपलब्ध नहीं कराता। Hash या state commitment हटाए गए bytes को वापस नहीं बना सकता। Ye jaankari shaikshanik hai।

## लाइसेंस

Ye project **Apache License 2.0** ke under licensed hai — poora text `LICENSE` file me dekho। Copyright 2026 Bharat Unified Technology (Sanjay Nain)।
