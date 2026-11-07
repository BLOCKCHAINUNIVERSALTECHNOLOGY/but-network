# Device Lock + 3-Key Design — BUT Trust (Design Note)

> Simple Hindi + English. Ye design note hai, code nahi. Isme likhi koi
> cheez abhi bani nahi hai — ye aage banane ka plan hai. Ye demo project
> hai, production nahi.

## 1. Device Lock — sirf ek phone se transaction

**Sawal:** Agar chor private key chura kar dusre phone me daal le, to
transaction kaise rokein?

**Jawab:** Key ko phone se baandh do, aur kharch karne ka faisla
BUT Trust Contract ko do.

1. Wallet banate hi phone ki ek **device key** Android Keystore
   (hardware) me banti hai. Ye key **kabhi phone se bahar nahi
   niklegi** — non-exportable. Sign hardware ke andar hota hai.
2. BUT Trust Contract me tumhare phone ki device key **register** hoti
   hai. Contract sirf usi key se sign ki hui transaction maanega.
3. Device Lock **ON** ho to dusre phone se transaction contract
   **reject** kar dega — chahe chor ke paas private key ho.
4. Naya phone add karna ho to: purane phone se approval + **48 ghante
   ka wait**. Chor turant device nahi badal sakta; 48 ghante me asli
   malik ko pata chal jayega aur woh request cancel kar sakta hai.
5. Phone kho jaye to recovery: seed phrase (24 words) se ya pehle se
   rakhe **guardian** (bharose ka dusra wallet) se.

**Imaandaar limit (ye mat bhoolna):**
- Seed phrase (24 words) **master key** hai. Agar woh chor ko mil jaye
  to woh naya wallet bana sakta hai. Isliye seed ko kabhi photo,
  screenshot ya cloud me mat rakho — sirf kagaz par, safe jagah.
- Device Lock **seed chori ko nahi rokta**, woh key ke galat istemal
  ko rokta hai.
- Agar chor ke paas purana phone + PIN/biometric dono hon, to rokna
  mushkil hai. Isliye daily limit bhi rakho (neeche dekho).

## 2. Teen key — Monero jaisi (Master / Spend / View)

Monero me do key hoti hain: view key (sirf dekhna) aur spend key
(bhejna). BUT me teen hongi:

| Key | Kaam | Kisko do |
|---|---|---|
| **Master key** | Backup / recovery, sab kuch dobara banana | Sirf khud ko, kagaz par safe |
| **Spend key** | Roz paise bhejna, device se bandhi hui | Sirf apne phone me, Keystore me |
| **View key** | Sirf balance aur transaction dekhna | Kisi ko bhi dikha sakte ho — isse paise NAHI bheje ja sakte |

**Daily limit:** Spend key se ek din me zyada se zyada ek fixed amount
hi bheji ja sake (jaise 1,000 BUT). Bada payment ho to master key +
48 ghante ka wait chahiye. Isse chori ka nuksaan limited rehta hai.

## 3. Fixed lengths (Sanjay ka faisla — na kam, na zyada)

- Seed phrase: bilkul **24 words** (real BIP39, 2048-word list).
- Private key: bilkul **128 characters** (hex, 64 bytes = 32-byte seed
  + 32-byte public key).
- Wallet address: bilkul **56 characters** (`but1` + 52 hex characters).
- Wallet galat length ko reject karega — code me validation hai
  (`shared/wallet-secure.ts`: `isValidAddress`,
  `isValidPrivateKeyFormat`, `assertFixedLengths`).

## 4. 1024-bit wali baat — confirm hua conclusion

**Sawal tha:** Kya 1024-bit aur usse upar ki key par wallet ban sakta hai?

**Confirm kiya (sources neeche):**

1. **RSA-1024 GALAT raasta hai.** NIST SP 800-57 ke hisaab se RSA-1024
   sirf ~80-bit security deta hai. NIST SP 800-131A ne RSA-1024 ko
   **disallowed** kar diya hai. Aaj RSA ka minimum 2048-bit hai, aur
   recommended 3072-bit hai.
2. **256-bit ECC, 3072-bit RSA ke barabar hai.** NIST equivalence table:
   128-bit security = 3072-bit RSA = 256-bit ECC = AES-128. Matlab
   Ed25519 (256-bit) RSA-1024 se **bahut zyada** strong hai, size me
   12 guna chhota hone par bhi.
3. **Asli wallet yahi use karte hain:** Bitcoin aur Ethereum secp256k1
   (256-bit), Solana aur Sui Ed25519 (256-bit), seed phrase BIP39
   12/24 words (128/256-bit entropy).
4. **"1024-bit" ka matlab samjho:** Key ka size bada hona suraksha
   nahi badhata — algorithm matter karta hai. RSA me 1024-bit kamzor
   hai, ECC me 256-bit strong hai. 1024-bit ECC curve standard me hai
   hi nahi (sabse bada standard P-521 hai).
5. **BUT ke liye sahi raasta:** Ed25519 256-bit (abhi), aur future me
   post-quantum upgrade (NIST FIPS 203/204: ML-KEM / ML-DSA) jab zaroorat
   pade. Quantum computer aane par RSA aur ECC dono Shor's algorithm se
   toot jayenge — koi bhi key size bacha nahi payega.

**Sources:**
- NIST SP 800-57 — key size equivalence (80-bit ≈ RSA-1024 ≈ ECC-160;
  128-bit ≈ RSA-3072 ≈ ECC-256)
- NIST SP 800-131A Rev. 2 — RSA-1024 disallowed, RSA-3072+ recommended
- https://www.classicalcryptotoday.com/learn/key-sizes-and-security-levels
- https://www.encryptionconsulting.com/symmetric-vs-asymmetric-encryption-top-use-cases-in-2025/
- https://www.fixmycert.com/guides/rsa-vs-ecc/
- BIP39 standard — 24 words = 256-bit entropy, 2048-word list
  (https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)

## 5. Aage ka kaam (order me)

1. Vetted `bip39` library se cross-check + independent security audit
   (abhi implementation haath se likha hai, wordlist official hai).
2. Android app build karna (JDK 17 + SDK chahiye — `android/README.md`
   dekho) aur Device Lock + 3-key wahan implement karna.
3. BUT Trust Contract (smart contract) likhna — device allowlist,
   48-ghante timelock, daily limit, pause button, multisig.
4. Testnet par Red Team / Blue Team testing, phir audit, phir launch.
