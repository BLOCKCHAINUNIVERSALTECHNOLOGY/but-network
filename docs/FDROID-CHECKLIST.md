# F-Droid Checklist — BUT Wallet (सरल Hindi + English)

> Ye checklist batati hai ki F-Droid par launch ke liye kya taiyar hai aur
> kya abhi BAaki hai. F-Droid sirf free open-source apps leta hai. Demo apps
> F-Droid par NAHI liye jate — isliye pehle asli Android app chahiye.

## Sach baat pehle (Honest limits)

- Ye repo abhi **web demo** hai (React + Vite + Express + tRPC). Ye Android
  app NAHI hai. F-Droid ko APK chahiye jo source code se bane.
- Ye demo **Bitcoin se behtar ya production-safe NAHI hai**. Bitcoin powerful
  hai kyunki uska network bahut bada hai, sirf code se nahi.
- Koi encryption 100% unbreakable nahi hota. Strong encryption + strong
  password + hardware storage milkar safety badhate hain.
- F-Droid review me **hafte ya mahine** lag sakte hain. Ye normal hai.

## 1. Kya taiyar hai (Ready)

| Cheez | Status |
|---|---|
| Source code public hai (GitHub) | ✅ Taiyar |
| LICENSE file hai (Apache-2.0) | ✅ Taiyar |
| package.json me license field hai (Apache-2.0) | ✅ Taiyar |
| SECURITY.md hai | ✅ Taiyar |
| Secure random (CSPRNG) — `Math.random` key ke liye nahi | ✅ Taiyar |
| Wallet encryption (AES-256-GCM + PBKDF2, slow unlock option) | ✅ Taiyar (demo) |
| Demo ledger encrypted save hota hai (Home.tsx) | ✅ Taiyar |
| Firebase / Google Mobile Services nahi hai | ✅ Taiyar |
| Analytics / tracker library nahi hai | ✅ Taiyar |

## 2. Kya NAHI taiyar hai (Not ready)

| Cheez | Kya chahiye |
|---|---|
| **Android app nahi hai** | Asli Android app banana hoga (Kotlin, Android Studio). Web demo ko APK me badalna kaafi nahi hai. |
| **Stable App ID nahi hai** | Android app ka fixed ID chahiye, jaise `in.bharatunifiedtechnology.butwallet`. Ek baar publish hone ke baad ID badla nahi jata. |
| **AWS S3 dependency check karni hai** | `@aws-sdk/client-s3` library Apache-2.0 hai (FOSS), par S3 ek proprietary service hai. Agar app ko S3 chahiye to F-Droid "NonFreeNet" anti-feature laga sakta hai. Decide karna hoga: S3 hataye ya optional rakhe. |
| **Google Maps types** | `@types/google.maps` sirf type definitions hain (dev time), runtime tracker nahi hai. Phir bhi Android app me Google Maps use mat karna — OpenStreetMap use karna. |
| **Reproducible build** | Best practice hai, zaroori nahi hai. F-Droid khud source se build karta hai. Build steps ko document karna hoga. |
| **Hardware-backed wallet storage** | Web demo localStorage use karta hai — ye hardware storage NAHI hai. Asli Android app me `EncryptedSharedPreferences` + Android Keystore use karna hoga. Keystore ki key hardware se kabhi bahar nahi aati, PIN/biometric se bandhi hoti hai. |
| **Independent security audit** | Launch se pehle kisi independent expert se audit karwana chahiye. Ye zaroori hai, optional nahi. |
| **F-Droid metadata** | App ka naam, description, icon, screenshots aur fastlane metadata chahiye. |

## 3. F-Droid ke niyam (Rules)

1. Sirf free open-source software — proprietary library, tracker ya ad nahi.
2. Code public hona chahiye, FOSS license ke saath.
3. App kaam ki honi chahiye — sirf demo / "hello world" accept nahi hoti.
4. F-Droid khud source se APK banata hai.
5. Reproducible build best practice hai, zaroori nahi hai.
6. Human review me hafte/mahine lag sakte hain.

## 4. Agle steps (Next steps, order me)

1. ✅ **License chun liya — Apache-2.0** — `LICENSE` file add ho gayi hai, README aur package.json bhi update hain।
2. **Asli Android app banaye** — Kotlin me, stable App ID ke saath.
3. **Hardware wallet storage** — Android Keystore + EncryptedSharedPreferences.
4. **S3 dependency ka faisla** — hataye ya optional rakhe.
5. **Security audit** — independent audit karwaye.
6. **F-Droid ko submit kare** — metadata + screenshots ke saath.

## 5. Kya ye checklist complete hai?

Nahi — ye shuruaat hai. Jaise-jaise Android app banega, ye checklist
update hogi. Koi bhi cheez "complete" tabhi mani jayegi jab F-Droid
usey accept kar le.
