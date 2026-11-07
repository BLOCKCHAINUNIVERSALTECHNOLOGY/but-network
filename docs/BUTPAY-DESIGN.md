# BUT Pay — Chhoti ID se Payment (Design + Implementation Note)

> Simple Hindi + English. BUT Pay Binance UID jaisi chhoti unique ID hai,
> jo lambe wallet address ki jagah use hoti hai. Chain-side kaam ban gaya
> hai (Rust core me) — app screen (Send/Receive + QR) Phase 3 me web/Android
> app me banegi. Ye demo project hai, production nahi — koi audit nahi hua hai.

## 1. BUT Pay ID kya hai?

Har wallet ko ek **chhota unique number** milta hai, jo uske wallet address
se juda hota hai:

- **Number ID:** `100001` se shuru, har naye wallet ko agla number
  (100001, 100002, 100003, ...). Ye number hi BUT Pay ID hai.
- **Naam / handle (optional):** apna chhota naam bhi chun sakte ho, jaise
  `@sanjay`. Ye bhi usi address se judta hai.

Dono ka matlab ek hi hai — **paisa bhejne ke liye address ki jagah ID ya
naam likho**. Jaise:

| Bhejne ka tareeka | Kya likhna hai |
|---|---|
| Address se (purana tareeka) | `but1af50f1cae2556c6a78024ca7b2113fc9facf59318c78303a1037` (56 chars) |
| ID se (BUT Pay) | `100001` |
| Naam se (BUT Pay) | `@sanjay` ya `sanjay` |

Teeno se paisa **wahi ek wallet** me pahunchega.

## 2. Dusre systems se tulna (ek line me har ek)

- **Binance UID:** Binance har user ko ek number deta hai (jaise 123456789) — usi tarah BUT Pay har wallet ko number deta hai, par Binance band (central) system hai, BUT Pay chain ki registry se chalta hai.
- **UPI ID:** UPI me `naam@bank` se paise bhejte hain bina account number ke — usi tarah BUT Pay me ID/naam se bhejte hain bina lambe address ke.
- **ENS (Ethereum Name Service):** ENS me `sanjay.eth` naam address se judta hai — usi tarah BUT Pay handle address se judta hai, par ENS ke ulta BUT Pay me number ID bhi milti hai (naam lena zaroori nahi).

## 3. Rules (Sanjay ke faisle)

1. **ID unique hogi:** har wallet ko ek hi number milta hai, aur ek number
   sirf ek hi wallet ka hota hai. Duplicate impossible hai — registry
   har naye wallet ko agla number deti hai (`100001` ke baad `100002`).
2. **Ek baar li to dobara nahi milegi:** ek address ki ID ban gayi to
   dobara register karne par error milega (aur pehli wali ID bata di
   jayegi). Ek address ki ek hi ID hoti hai.
3. **Naam pehle aao, pehle pao:** handle jo pehle le lega, usi ka rahega.
   Dobara wahi naam lene par error milega. Naam 3–32 characters ka hona
   chahiye (sirf `a-z 0-9 _ - .`), shuru/aakhir me `_` ya `-` nahi, aur
   `..` nahi. System wale naam koi nahi le sakta: `but`, `butpay`,
   `butcoin`, `admin`, `support`, `official`, `team`, `help`, `system`,
   `root`, `mod`, `security` (nakal/dhokha rokne ke liye).
4. **ID aur naam public honge:** ID/naam sirf **receive karne** ke liye
   hain — inhe kisi ko bhi de sakte ho, isme koi khatra nahi. Inka private
   key ya seed phrase se **koi lena-dena nahi** hai. ID/naam jaan kar koi
   tumhare paise nahi chura sakta.
5. **ID kabhi nahi badalti:** ek baar ID kisi address se jud gayi to
   hamesha usi address ki rahegi — kabhi dusre address par nahi jayegi
   (immutable registry). Isi liye purani signed transaction ka matlab
   bhi kabhi nahi badlega (neeche §5 dekho).

## 4. Send kaise hoga?

1. User **ID ya naam likhta hai** (jaise `100001` ya `@sanjay`) — ya QR
   scan karta hai (neeche §7 dekho).
2. System registry se **ID/naam ko address me badalta hai** (resolve).
3. Phir **normal transaction** banti hai — signature, nonce, balance,
   fee, sab waisa hi jaise address se bhejte hain.
4. Block me transaction **jaisi likhi gayi thi** (receiver = ID string)
   waisi hi save hoti hai; credit hamesha **resolve kiye gaye address**
   ko milta hai.

Galat ID/naam likha ho to transaction **bante hi reject** hogi — paise
kahin nahi jayenge, error milega: "BUT Pay ID/naam nahi mila".

## 5. Implementation — chain me kya bana hai (Rust core)

**Naya module:** `chain/src/butpay.rs`

- `ButPayRegistry` — chaar maps: ID → address, handle → address,
  address → ID (ulta lookup, duplicate rokne ke liye), address → handle.
- `register(address, handle)` — agla number do (100001 se shuru),
  duplicate address/naam reject ho. Return: nayi ID.
- `resolve(input)` — number ho to ID samjho, valid `but1...` address ho
  to waisa ka waisa do (pass-through), warna handle samjho (`@` ke saath
  ya bina). Na mile to `None`.
- `lookup(query)` — ID/handle/address se poori entry do
  (`{id, address, handle}`) — API aur CLI ke liye.
- `resolve_for_tx(receiver)` — transaction ke liye resolve; na mile to
  error.

**Signature ka niyam (sabse zaroori):** signature hamesha **jaisi likhi
gayi payload par** verify hoti hai — receiver = ID string, jaisa user ne
sign kiya tha. Registry immutable hai, isliye ID ka matlab kabhi nahi
badlega, to signature ka matlab bhi kabhi nahi badlega. Chain block ko
ID wali transaction ke saath verify kar sakti hai, aur credit resolve
kiye gaye address ko milta hai.

**Jude hue hisse:**

| File | Kya joda gaya |
|---|---|
| `chain/src/transaction.rs` | `verify_with_registry()` / `verify_with_resolved_receiver()` — signature ID wali payload par verify; `verify()` purana behaviour (address receiver) |
| `chain/src/mempool.rs` | `add_with_registry()` / `add_resolved()` — ID wali tx mempool me aati hai |
| `chain/src/chain.rs` | `add_block_with_registry()`, `verify_chain_with_registry()`, `add_block_bft_with_registry()` — ID wali tx block me judti hai aur credit address ko milta hai; purane `add_block` / `verify_chain` khaali registry se chalte hain (ID tx reject) |
| `chain/src/storage.rs` | `save_butpay()` / `load_butpay()` / `load_chain_with_registry()` / `butpay_path_for()` — registry `data/butpay.json` me, chain file ke saath |
| `chain/src/api.rs` | `GET /butpay/<id-ya-handle>`, `POST /butpay/register`, `POST /tx` me receiver ke roop me ID/handle; `mine()` registry ke saath block jodta hai |
| `chain/src/main.rs` | CLI: `butpay-register`, `butpay-lookup`; `demo` me Block 3: Alice → ID 100001 (Bob) 10 |
| `chain/src/lib.rs` | `butpay` module export (`ButPayRegistry`, `BUTPAY_FIRST_ID`) |

**Persistence — registry kahan save hoti hai:**

- File: **`data/butpay.json`** — hamesha chain file (`data/chain.json`)
  ke saath, wahi directory me.
- Save: pehle temp file (`.json.tmp`) me likho, phir rename karo — crash
  hone par bhi file aadhi nahi likhi jayegi (atomic write).
- Load: file na mile to khaali registry milti hai (pehli baar chalane par
  normal hai). File mili lekin data kharab hua to error — galat registry
  se chain verify nahi hogi.
- Registry file chain ke saath hi travel karti hai — node band ho jaye to
  bhi IDs disk par rehti hain.

**CLI:**

```text
but-node butpay-register <address> [handle] [chain-file]
    # Jaise: but-node butpay-register but1af50... bob
    # Jawab: BUT Pay ID mil gayi: 100001

but-node butpay-lookup <id-ya-handle> [chain-file]
    # Jaise: but-node butpay-lookup 100001   ya   but-node butpay-lookup @bob
    # Jawab: BUT Pay ID: 100001, handle: @bob, address: but1af50...
```

**API (local, port 3030):**

```text
GET  /butpay/<id-ya-handle>     → {"ok": true, "id": 100001, "handle": "bob", "address": "but1..."}
POST /butpay/register            → body {"address": "but1...", "handle": "bob"}
                                   → {"ok": true, "id": 100001, "address": "but1...", "handle": "bob"}
POST /tx                         → receiver me address ke saath "100001" ya "@bob" bhi chalega
```

**Tests:** `cargo test` — **70/70 pass** (pehle 55 the, 15 naye BUT Pay
tests: register + lookup + duplicate reject + galat ID error + ID se
send + mempool + chain + storage roundtrip + API end-to-end).

**Demo:** `but-node demo` — Block 3 me Alice Bob ko ID `100001` se 10
bhejti hai, Bob ka balance 75 → 85 hota hai, reload + verify PASS.
`but-node p2p-demo` abhi bhi chalta hai.

## 6. Kya abhi bhi baaki hai (imaandaar list)

- **Web/Android app se jodna baaki hai:** chain-side ready hai, par web
  demo (client/) abhi bhi fake localStorage ledger dikhata hai — BUT Pay
  button aur Send/Receive screen abhi app me nahi bani (Phase 3).
- **P2P par registry sync nahi hoti:** do nodes ke beech sirf blocks sync
  hote hain; `butpay.json` har node par alag file hai. Dusre node par ID
  register hui ho to is node ko pata nahi chalega — registry ko P2P sync
  ya chain me daalna aage ka kaam hai.
- **Handle badalna / ID transfer nahi hota:** design ke hisaab se ID
  immutable hai — handle badalne ka koi tareeka abhi nahi hai.
- **Koi fee / rate-limit nahi:** register karna abhi free hai — koi bhi
  hazaron ID bana sakta hai (spam). Production se pehle register par
  chhoti fee ya limit chahiye.
- **Koi audit nahi:** code sandbox me test hua hai, independent audit
  nahi hua hai. Production se pehle audit zaroori hai.

## 7. App UX — BUT Pay screen (Sanjay ka bataya hua UX)

> Sanjay ne bataya (2026-11-07): app me BUT Pay button hoga, us par
> click karte hi Send aur Receive — Google Pay jaisa. Ye section usi UX
> ka plan hai. **QR code sirf address/ID ko tasveer me badalta hai** —
> uske liye chain me kuch naya nahi chahiye; chain-side kaam registry +
> resolve + send-by-ID hi hai, jo §5 me ban gaya hai. QR aur screen
> app-side (web/Android) ka kaam hai, Phase 3 me banega.

### 7.1 BUT Pay button → 2 options

App me ek **BUT Pay button** hoga. Us par click karte hi 2 options
khulenge:

```text
        [ BUT Pay ]
            |
     +------+------+
     |             |
  [ Send ]    [ Receive ]
```

- **Send** — paise bhejna (neeche §7.3).
- **Receive** — paise lena, apna QR dikhana (neeche §7.2).

### 7.2 Receive — QR code dikhega (Google Pay jaisa)

- Receive par screen par **QR code** dikhega — bilkul Google Pay ke
  "receive" jaisa.
- QR code me hoga: **wallet address / BUT ID** (aur handle, agar ho to).
  Jaise QR me `100001` ya `but1af50...` hoga.
- Dusra user apne phone se QR **scan karte hi** paise bhej sakega —
  use address ya ID type karne ki zaroorat nahi padegi.
- Receive screen par apni **ID aur handle bhi likha dikhega** (copy karne
  ke liye), taaki jiske paas QR scan nahi hai woh ID likh kar bhej sake.
- **QR me kabhi private key / seed phrase NAHI hoga** — sirf public
  receive info (address / ID / handle). QR dikhana safe hai, jaise apni
  ID kisi ko batana safe hai (§3, rule 4).

### 7.3 Send — QR scan ya BUT ID likh kar bhejo

Send par 2 tareeke honge:

1. **QR scan karo:** dusre ki Receive screen ka QR scan karo — app QR
   se address/ID nikaal kar bhar degi, phir amount likho aur bhejo.
2. **BUT ID likh kar bhejo (QR nahi hai to):** receiver ki **BUT ID**
   (number, jaise `100001`) ya **naam** (jaise `@sanjay`) likho — bilkul
   **Google Pay UID ki tarah**. App ID ko registry se address me badal
   kar normal transaction bhejegi (§4).

Send karte waqt app pehle **ID check karegi**: galat ID/naam likha ho to
bhejne se pehle hi error dikhega ("ID nahi mili") — paise kahin nahi
jayenge. Sahi ID ho to receiver ka naam/handle dikhaya jayega, taaki
galat aadmi ko paise na jayen.

### 7.4 Har user ko unique ID kaise milegi (guarantee)

- Har naya wallet banate hi app **registry se agla number** leti hai
  (100001, 100002, 100003, ...) — number kabhi dobara nahi milta,
  isliye **duplicate impossible** hai (§3, rule 1).
- Wallet banate waqt user ko uski ID dikha di jayegi (Receive screen par
  bhi hamesha dikhegi).
- Handle optional hai — user chahe to baad me bhi apna naam chun sakta
  hai (pehle aao, pehle pao — §3, rule 3).

### 7.5 UX flow — ek line me

```text
Receive: BUT Pay button → Receive → QR dikhao (address/ID) → scan hote hi paisa aaya
Send:    BUT Pay button → Send → QR scan karo YA ID/naam likho → amount → bhejo
```
