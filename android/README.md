# BUT Wallet — Android App (Skeleton / ढाँचा)

> **SKELETON ONLY — abhi ye app compile nahi hua hai aur na hi chalega.**
> Is machine par Java / Gradle / Android SDK kuch bhi installed nahi hai,
> isliye build test nahi ho paya. Ye folder asli app ki shuruaat hai:
> sahi project structure, sahi App ID, Keystore + EncryptedSharedPreferences
> wala security code, aur F-Droid ke liye Fastlane metadata.
> Ye demo hai — isme asli paisa kabhi mat rakho.

## App ID (draft — F-Droid submit karne se pehle final karna)

`tech.bharatunified.butwallet`

App ID ek baar F-Droid par submit hone ke baad **kabhi nahi badalta**,
isliye ise final karne se pehle Sanjay se confirm karo.

## Build karne ke liye kya chahiye (exact)

Is machine par abhi kuch nahi hai:

| Cheez | Size (lagbhag) | Kaise milega |
|---|---|---|
| JDK 17 (Temurin / OpenJDK) | ~200–300 MB | `apt install temurin-17-jdk` ya tarball |
| Android cmdline-tools | ~165 MB | dl.google.com (commandlinetools-linux-13114758_latest.zip) |
| platform-tools + platforms;android-34 + build-tools;34.0.0 | ~700 MB – 1 GB | `sdkmanager` se |
| Gradle 8.11.1 (wrapper download karega) | ~130 MB | pehle build par automatic |
| **Kul download** | **~1.5–2 GB** | + install/setup time ~30–60 min |

RAM note: is machine par abhi sirf ~736 MB free RAM dikhi — Gradle build
ke liye kam se kam 2 GB free RAM chahiye, warna build bahut slow hoga ya
fail hoga. Pehle RAM free karo ya zyada RAM wali machine use karo.

## Install karne ke commands (SDK install hone ke baad)

```bash
export ANDROID_HOME=$HOME/Android/sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools
sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0"
cd android
./gradlew assembleDebug
# APK: app/build/outputs/apk/debug/app-debug.apk
```

## Is skeleton me kya hai

- `app/src/main/java/tech/bharatunified/butwallet/MainActivity.kt` — main screen (Hindi UI)
- `app/src/main/java/tech/bharatunified/butwallet/WalletCrypto.kt` — Device-bound key (Android Keystore, non-exportable) + EncryptedSharedPreferences
- `app/src/main/AndroidManifest.xml` — `allowBackup=false`, cleartext off
- `fastlane/metadata/android/en-US/` — F-Droid ke liye short/full description + changelog
- `gradle/wrapper/gradle-wrapper.properties` — Gradle 8.11.1

## Abhi kya NAHI hai (aage ka kaam)

- Asli blockchain connection (koi network code nahi hai)
- Real BIP39 / Ed25519 wallet (web demo wala module yahan port karna hai)
- Device Lock + 3-key (master / spend / view) — design `docs/DEVICE-LOCK-DESIGN.md` me hai
- F-Droid ke liye `metadata/<app-id>.yml` (fdroiddata repo me MR se banta hai)
- Independent security audit
