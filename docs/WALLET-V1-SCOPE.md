# BUT Wallet v1 — Scope (FINAL — Sanjay, 2026-11-07)

Sanjay ne wallet research report padh kar faisla diya: v1 me pehle
sirf ye 5 cheezein banein. Baaki sab baad me.

## V1 me ye 5 honge

1. **Seed backup + verify karna** — wallet banate waqt 24 words dikhao,
   user se verify karwao (seed bilkul 24 words, real BIP39).
2. **PIN + biometric + auto-lock** — lock screen aur encrypted vault
   (AES-GCM); biometric WebAuthn se ho sake to, warna PIN + auto-lock.
3. **Bhejne se pehle saaf preview** — kis ko, kitna, fee kitni — phir
   confirm.
4. **BUT Pay Send/Receive + QR** — Receive par QR code (address/ID),
   Send par QR scan ya BUT ID/handle likh kar bhejo.
5. **Scam warning** — galat/anjaan address, badi rakam, ya shak wali
   cheez par warning dikhao.

## Baad me (v1 me nahi)

Swap, staking, hardware wallet, dApp browser, WalletConnect, NFT,
fiat se khareedna, multisig.

## Note

- Wallet non-custodial: private key sirf user ke phone par, server par
  kabhi nahi.
- Seed phrase kagaz par — photo / cloud / chat me kabhi nahi.
- Koi independent audit abhi nahi hua; Android app abhi skeleton hai.

## Build status — v1 ban gaya (7 Nov 2026, local, NOT pushed)

- **Seed backup + verify:** wallet banate waqt 24 words dikhte hain, 3 random
  words type karke verify karna padta hai. Import bhi seed se hota hai.
  Mnemonic HI wallet hai — keypair mnemonic se banta hai (Rust core jaisa
  derivation, `shared/wallet-secure.ts` + `shared/but-tx.ts` tweetnacl).
- **PIN + biometric + auto-lock:** password unlock (12+ akshar, AES-GCM
  vault) + WebAuthn PRF biometric (jahan browser/device support kare) +
  5 minute auto-lock. Web me password hi PIN ka kaam karta hai — chhota
  numeric PIN isliye nahi rakha kyunki browser vault offline attack ke
  liye khula rehta hai; hardware-backed PIN/biometric Android app me aayega.
- **Send preview:** kis ko (resolve kiya hua address + ID/handle), kitna,
  fee 1 bit, kul, bhejne ke baad balance — phir confirm.
- **BUT Pay + QR:** Receive par QR code (address), Send par QR scan
  (camera + jsQR) ya BUT ID/handle likh kar bhejo.
- **Scam warning:** pehli baar bhejna, balance ka aadhe se zyada, khud ko
  bhejna, balance se zyada — block/warn/info. Koi central scam list nahi
  hai (v1 me sirf local rules).
- Balance + history chain se aate hain (`shared/but-node.ts`, API port 3030).
- Verify: `cargo test` 71/71, `npx vitest run` 58/58, `npx tsc --noEmit`
  pass, `vite build` pass, live E2E (`scripts/e2e-wallet-v1.ts`) pass —
  Alice -> naya wallet 5000 bits, naya wallet -> Bob ID 100001 se 1000 bits,
  history chain se mili.
- Baaki hai: real browser me UI chalana (sandbox me browser nahi hai),
  biometric ka real device par test, independent audit, Android app,
  P2P par BUT Pay registry sync, register fee/limit, hosted node + HTTPS.
