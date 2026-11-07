# BUT Wallet Demo

BUT के Hot Mode, Cold Mode और ब्लॉक-डेटा स्टोरेज विकल्पों को समझने के लिए एक शुरुआती, ब्राउज़र-आधारित डेमो। यह **असली cryptocurrency या blockchain नहीं है**। यहाँ दिखने वाला बैलेंस और लेजर नकली है और केवल आपके ब्राउज़र के `localStorage` में रहता है। ऐप कोई असली भुगतान, निजी कुंजी, seed phrase, नेटवर्क या फंड नहीं संभालता।

## इसे चलाएँ

1. GitHub पेज पर **Code → Download ZIP** चुनें, या repository को clone करें।
2. [Node.js](https://nodejs.org/) और pnpm उपलब्ध होने पर प्रोजेक्ट फ़ोल्डर में `pnpm install` चलाएँ।
3. `pnpm dev:static` चलाएँ और टर्मिनल में दिखा स्थानीय पता ब्राउज़र में खोलें।

वैकल्पिक रूप से `pnpm check` TypeScript जाँचता है, `pnpm test` उपलब्ध tests चलाता है, और `pnpm build:static` static site बनाता है।

## डेमो में क्या है

- Hot Mode: नकली लेन-देन की रकम और नकली पता दिखाने वाला डेमो अनुभव।
- Cold Mode: लेजर में कुछ विवरणों को केवल UI स्तर पर छिपाने वाला प्रदर्शन।
- टेस्ट लेजर: नकली भेजें/प्राप्त करें प्रविष्टियाँ, स्थिति और समय।
- ब्लॉक डेटा जानकारी: lossless compression, node pruning और state snapshots के बीच अंतर।

## सुरक्षा और प्राइवेसी की अहम सीमा

Cold Mode सिर्फ स्क्रीन पर विवरण छिपाता है। यह क्रिप्टोग्राफ़िक गोपनीयता नहीं देता और डेमो डेटा ब्राउज़र के local storage में पढ़ा जा सकता है। इसमें असली निजी जानकारी, seed phrase, वॉलेट कुंजी या असली कॉइन न डालें। असली प्राइवेसी नेटवर्क के लिए सावधानीपूर्वक प्रोटोकॉल डिज़ाइन, स्वतंत्र सुरक्षा ऑडिट और लागू कानूनी आवश्यकताओं की समीक्षा चाहिए। यह prototype निवेश, भुगतान या production उपयोग के लिए नहीं है।

पूरी सुरक्षा नीति के लिए `SECURITY.md` पढ़ें। उसमें लिखा है कि ये डेमो क्या है और क्या नहीं है, क्या नहीं डालना है, और कोई कमज़ोरी मिले तो उसे private तरीके से कैसे report करें।

**नया secure module:** `shared/crypto-secure.ts` में अब सही CSPRNG random, SHA-256 hash, demo keypair और AES-GCM encryption (password से key बनाकर) है। Demo transaction ID अब `Math.random()` की जगह secure random से बनता है।

**साफ़ चेतावनी:** इस module के होने से ये डेमो अब भी असली blockchain या production wallet नहीं बनता।

**नया — encrypted ledger:** Demo balance aur ledger अब Home.tsx में password (कम से कम 12 अक्षर) से AES-GCM encrypt करके save होते हैं। पुराना plain data एक बार migrate करके delete कर दिया जाता है। Password kahin save नहीं होता — bhool gaye to data sirf reset karke milega।

**नया — demo wallet module:** `shared/wallet-secure.ts` में demo wallet banta hai — mnemonic (BIP39-style, real BIP39 नहीं), Ed25519 keypair, `but1` address, aur slow unlock (~2 minute, brute-force ko mehenga banane ke liye)। Slow unlock ka matlab unbreakable नहीं है — kamzor password phir भी टूट जाएगा।

**F-Droid:** F-Droid par launch ke liye checklist dekho — `docs/FDROID-CHECKLIST.md`। License taiyar hai — project Apache License 2.0 ke under hai (`LICENSE` dekho)। Abhi Android app nahi hai, isliye F-Droid launch abhi possible nahi hai।

## ब्लॉक डेटा का आकार

Lossless compression bytes को कम जगह में रखता है, लेकिन इतिहास मिटाता नहीं है। Pruning के बाद कोई नोड अपनी स्थानीय कॉपी से पुराने ब्लॉक विवरण हटा सकता है; नेटवर्क के archive nodes इतिहास उपलब्ध रख सकते हैं। Snapshot नए नोड को वर्तमान state से sync करने में मदद कर सकता है, पर अपने-आप हर पुराना transaction उपलब्ध नहीं कराता। Hash या state commitment हटाए गए bytes को वापस नहीं बना सकता। ऐप का यह अनुभाग शैक्षिक है और किसी वास्तविक नेटवर्क के आँकड़े नहीं दिखाता।

## लाइसेंस

Ye project **Apache License 2.0** ke under licensed hai — poora text `LICENSE` file me dekho। Copyright 2026 Bharat Unified Technology (Sanjay Nain)।
