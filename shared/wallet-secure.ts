/**
 * shared/wallet-secure.ts — DEMO ONLY (sirf demo ke liye)
 *
 * Ye module demo wallet banane ke liye hai: mnemonic, keypair, address
 * aur password se encrypt kiya hua wallet vault.
 *
 * ZAROORI SACH — pehle padho:
 * - Ye module is demo ko asli wallet ya production blockchain NAHI banata.
 * - Isse bana wallet SIRF DEMO hai. Isme asli paisa kabhi mat rakho.
 * - Mnemonic yahan "BIP39-style" hai, real BIP39 NAHI hai. Real BIP39 me
 *   2048 words ki official list hoti hai (11 bit per word). Yahan demo ke
 *   liye 256 words ki list hai (8 bit per word). Real BIP39 ke liye vetted
 *   library (jaise `bip39`) aur independent security audit chahiye.
 * - Slow unlock (2 minute) brute-force ko mehenga banata hai, par ye
 *   wallet ko "unbreakable" NAHI banata. Kamzor password (jaise "123456")
 *   phir bhi dictionary attack se toot jayega. Strong, unique password
 *   (kam se kam 12 akshar) hamesha zaroori hai.
 * - Ye WEB demo hai. Ye Android Keystore / iPhone Secure Enclave ko use
 *   NAHI kar sakta. Asli phone me hardware-backed storage ke liye asli
 *   Android app chahiye (EncryptedSharedPreferences + Android Keystore).
 *   Web demo me encrypted vault localStorage me rehta hai — ye hardware
 *   storage NAHI hai. Slow KDF yahan uski kami ko thoda kam karta hai.
 *
 * Security rules (kabhi mat todo):
 * - Random ke liye sirf CSPRNG (crypto.getRandomValues). Math.random
 *   kabhi key, seed ya password ke liye use mat karo.
 * - Private key aur mnemonic ko kabhi plain localStorage me mat rakho.
 * - Password ko kabhi code me, kabhi localStorage me mat rakho.
 *   Password sirf user ki memory me aur unlock ke waqt RAM me rehta hai.
 */

import {
  decryptFromLocalStorage,
  encryptForLocalStorage,
  generateDemoKeypair,
  secureRandomBytes,
  sha256Hex,
} from "./crypto-secure";

// ---------------------------------------------------------------------------
// 0. Demo wordlist (256 words) — real BIP39 list NAHI hai
//    256 words = 8 bit per word. 32 bytes entropy + 1 checksum byte
//    = 33 words. Real BIP39: 2048 words, 11 bit per word.
// ---------------------------------------------------------------------------

export const DEMO_WORDLIST: readonly string[] = [
  "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
  "absurd", "accent", "accept", "access", "accident", "account", "accuse", "achieve",
  "acid", "acoustic", "acquire", "across", "action", "actor", "actress", "actual",
  "adapt", "add", "addict", "address", "adjust", "admit", "adult", "advance",
  "advice", "aerobic", "affair", "afford", "afraid", "again", "age", "agent",
  "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album",
  "alcohol", "alert", "alien", "align", "alike", "alive", "all", "alley",
  "allow", "almost", "alone", "alpha", "already", "also", "alter", "always",
  "amateur", "amazing", "among", "amount", "amused", "analyst", "anchor", "ancient",
  "anger", "angle", "angry", "animal", "ankle", "announce", "annual", "another",
  "answer", "antenna", "antique", "anxiety", "any", "apart", "apology", "appear",
  "apple", "approve", "april", "arch", "arctic", "area", "arena", "argue",
  "arm", "armed", "armor", "army", "around", "arrange", "arrest", "arrive",
  "arrow", "art", "artefact", "artist", "artwork", "ask", "aspect", "assault",
  "asset", "assist", "assume", "asthma", "athlete", "atom", "attack", "attend",
  "attitude", "attract", "auction", "audit", "august", "aunt", "author", "auto",
  "autumn", "average", "avocado", "avoid", "awake", "aware", "away", "awesome",
  "awful", "axis", "baby", "bachelor", "bacon", "badge", "bag", "balance",
  "balcony", "ball", "bamboo", "banana", "banner", "bar", "barely", "bargain",
  "barrel", "base", "basic", "basket", "battle", "beach", "bean", "beauty",
  "become", "beef", "before", "begin", "behave", "behind", "believe", "below",
  "belt", "bench", "benefit", "best", "betray", "better", "between", "beyond",
  "bicycle", "bid", "bike", "bind", "biology", "bird", "birth", "bitter",
  "black", "blade", "blame", "blanket", "blast", "bleak", "bless", "blind",
  "blood", "blossom", "blouse", "blue", "blur", "blush", "board", "boat",
  "body", "boil", "bomb", "bone", "bonus", "book", "boost", "border",
  "boring", "borrow", "boss", "bottom", "bounce", "box", "boy", "bracket",
  "brain", "brand", "brass", "brave", "bread", "breeze", "brick", "bridge",
  "brief", "bright", "bring", "brisk", "broccoli", "broken", "bronze", "broom",
  "brother", "brown", "brush", "bubble", "buddy", "budget", "buffalo", "build",
  "bulb", "bulk", "bullet", "bundle", "bunker", "burden", "burger", "burst",
  "bus", "business", "busy", "butter", "buyer", "buzz", "cabbage", "cabin",
];

const WORD_INDEX = new Map(DEMO_WORDLIST.map((w, i) => [w, i]));

/** Wallet password kam se kam 12 akshar ka hona chahiye. */
export const MIN_WALLET_PASSWORD_LENGTH = 12;

/** Fast unlock: 210,000 PBKDF2 iterations (roz ke use ke liye). */
export const FAST_KDF_ITERATIONS = 210_000;

/** Slow unlock ka default target: 2 minute (120,000 ms). */
export const SLOW_UNLOCK_TARGET_MS = 120_000;

/** Slow unlock ek chunk me kitne iterations karta hai. */
const SLOW_KDF_CHUNK_ITERATIONS = 500_000;

/** Test mode: sirf test ke liye, kabhi asli use me mat lao. */
export const TEST_KDF_ITERATIONS = 1_000;

function getCrypto(): Crypto {
  const cryptoObj = globalThis.crypto;
  if (!cryptoObj || !cryptoObj.getRandomValues || !cryptoObj.subtle) {
    throw new Error(
      "Secure crypto available nahi hai. Modern browser ya Node.js 19+ use karo.",
    );
  }
  return cryptoObj;
}

function assertStrongPassword(password: string): void {
  if (typeof password !== "string" || password.length < MIN_WALLET_PASSWORD_LENGTH) {
    throw new Error(
      `Wallet password kam se kam ${MIN_WALLET_PASSWORD_LENGTH} akshar ka hona chahiye.`,
    );
  }
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

// ---------------------------------------------------------------------------
// 1. Mnemonic (BIP39-style, real BIP39 NAHI hai — upar padho)
// ---------------------------------------------------------------------------

/**
 * Secure entropy se mnemonic banao. Default 32 bytes entropy + 1 checksum
 * byte = 33 words. Entropy hamesha CSPRNG se aati hai.
 */
export async function generateMnemonic(entropyBytes = 32): Promise<string> {
  if (entropyBytes !== 16 && entropyBytes !== 32) {
    throw new Error("Entropy 16 ya 32 bytes ki honi chahiye.");
  }
  const entropy = secureRandomBytes(entropyBytes);
  const checksumHex = await sha256Hex(entropy);
  const checksumByte = Number.parseInt(checksumHex.slice(0, 2), 16);
  const bytes = new Uint8Array(entropy.length + 1);
  bytes.set(entropy, 0);
  bytes[entropy.length] = checksumByte;
  return Array.from(bytes, (b) => DEMO_WORDLIST[b]).join(" ");
}

/** Mnemonic sahi hai ya nahi — words, length aur checksum check karo. */
export async function validateMnemonic(mnemonic: string): Promise<boolean> {
  if (typeof mnemonic !== "string") return false;
  const words = mnemonic.trim().split(/\s+/);
  if (words.length !== 17 && words.length !== 33) return false;
  const bytes: number[] = [];
  for (const word of words) {
    const index = WORD_INDEX.get(word);
    if (index === undefined) return false;
    bytes.push(index);
  }
  const entropy = new Uint8Array(bytes.slice(0, -1));
  const checksumHex = await sha256Hex(entropy);
  const expected = Number.parseInt(checksumHex.slice(0, 2), 16);
  return bytes[bytes.length - 1] === expected;
}

/**
 * Mnemonic se seed banao (BIP39 jaisa: PBKDF2-SHA-512, 2048 iterations,
 * 64 bytes). Salt me "but-mnemonic" prefix hai — ye real BIP39 ka
 * "mnemonic" salt NAHI hai, isliye seed real BIP39 wallet se alag hoga.
 */
export async function mnemonicToSeed(
  mnemonic: string,
  passphrase = "",
): Promise<Uint8Array> {
  if (!(await validateMnemonic(mnemonic))) {
    throw new Error("Mnemonic sahi nahi hai.");
  }
  const cryptoObj = getCrypto();
  const baseKey = await cryptoObj.subtle.importKey(
    "raw",
    new TextEncoder().encode(mnemonic),
    "PBKDF2",
    false,
    ["deriveBits"],
  );
  const bits = await cryptoObj.subtle.deriveBits(
    {
      name: "PBKDF2",
      salt: new TextEncoder().encode(`but-mnemonic${passphrase}`),
      iterations: 2048,
      hash: "SHA-512",
    },
    baseKey,
    512,
  );
  return new Uint8Array(bits);
}

// ---------------------------------------------------------------------------
// 2. Wallet banana: keypair + mnemonic + address
// ---------------------------------------------------------------------------

export type DemoWallet = {
  /** Address: "but1" + SHA-256(public key) ke pehle 20 bytes (hex). */
  address: string;
  /** Public key, base64 (SPKI). Share karna safe hai. */
  publicKeyBase64: string;
  /** Private key, base64 (PKCS8). Kabhi share mat karo, kabhi plain mat rakho. */
  privateKeyBase64: string;
  /** Kaunsa algorithm: "Ed25519" ya fallback "ECDSA P-256". */
  algorithm: string;
  /** Mnemonic (33 words). Sirf ek baar dikhao, user ko likhne ko kaho, phir kabhi store mat karo. */
  mnemonic: string;
  warning: string;
};

const WALLET_WARNING =
  "DEMO WALLET — Ye sirf demo ke liye hai. Isme asli paisa kabhi mat rakho. Mnemonic aur private key ko kisi ko mat dikhao.";

/** Naya demo wallet banao: CSPRNG se mnemonic + keypair, address SHA-256 se. */
export async function generateWallet(): Promise<DemoWallet> {
  const keypair = await generateDemoKeypair();
  const mnemonic = await generateMnemonic(32);
  const publicKeyBytes = base64ToBytes(keypair.publicKeyBase64);
  const addressHash = await sha256Hex(publicKeyBytes);
  const address = `but1${addressHash.slice(0, 40)}`;
  return {
    address,
    publicKeyBase64: keypair.publicKeyBase64,
    privateKeyBase64: keypair.privateKeyBase64,
    algorithm: keypair.algorithm,
    mnemonic,
    warning: WALLET_WARNING,
  };
}

// ---------------------------------------------------------------------------
// 3. KDF: fast / slow / test — password se AES-GCM key banana
// ---------------------------------------------------------------------------

export type KdfMode = "fast" | "slow" | "test";

export type DerivedWalletKey = {
  key: CryptoKey;
  /** Kul kitne PBKDF2 iterations hue. */
  iterations: number;
  /** Kitna time laga (ms). Slow mode me ye target ke aas-paas hota hai. */
  elapsedMs: number;
};

async function pbkdf2Bits(
  passwordBytes: Uint8Array<ArrayBufferLike>,
  salt: Uint8Array<ArrayBufferLike>,
  iterations: number,
): Promise<Uint8Array> {
  const cryptoObj = getCrypto();
  const baseKey = await cryptoObj.subtle.importKey(
    "raw",
    passwordBytes as BufferSource,
    "PBKDF2",
    false,
    ["deriveBits"],
  );
  const bits = await cryptoObj.subtle.deriveBits(
    {
      name: "PBKDF2",
      salt: salt as BufferSource,
      iterations,
      hash: "SHA-256",
    },
    baseKey,
    256,
  );
  return new Uint8Array(bits);
}

async function importAesKey(
  material: Uint8Array<ArrayBufferLike>,
): Promise<CryptoKey> {
  return getCrypto().subtle.importKey(
    "raw",
    material as BufferSource,
    "AES-GCM",
    false,
    ["encrypt", "decrypt"],
  );
}

/**
 * Fixed iterations se key banao (decrypt ke liye — slow mode me bhi wahi
 * iterations dobara chalenge, isliye unlock karne me bhi utna hi time
 * lagega jitna lock karne me laga tha. Ye trade-off jaan-bujh kar hai.)
 */
export async function deriveWalletKeyFixed(
  password: string,
  salt: Uint8Array,
  totalIterations: number,
): Promise<DerivedWalletKey> {
  assertStrongPassword(password);
  if (!Number.isInteger(totalIterations) || totalIterations <= 0) {
    throw new Error("Iterations sahi honi chahiye.");
  }
  const start = Date.now();
  let material: Uint8Array<ArrayBufferLike> =
    new TextEncoder().encode(password);
  let remaining = totalIterations;
  let chunkIndex = 0;
  while (remaining > 0) {
    const chunk = Math.min(SLOW_KDF_CHUNK_ITERATIONS, remaining);
    const chunkSalt = new Uint8Array(salt.length + 2);
    chunkSalt.set(salt, 0);
    chunkSalt[salt.length] = chunkIndex & 0xff;
    chunkSalt[salt.length + 1] = (chunkIndex >> 8) & 0xff;
    material = await pbkdf2Bits(material, chunkSalt, chunk);
    remaining -= chunk;
    chunkIndex += 1;
  }
  const key = await importAesKey(material);
  return { key, iterations: totalIterations, elapsedMs: Date.now() - start };
}

/**
 * Slow unlock: time-targeted KDF. Tab tak chunks chalao jab tak target
 * time (default 2 minute) poora na ho jaye. Fast device par bhi poora
 * target time lagega — ye jaan-bujh kar hai, taaki har guess mehenga ho.
 * Slow mode ka matlab "unbreakable" NAHI hai — kamzor password phir
 * bhi toot jayega. Ye sirf brute-force ko mehenga banata hai.
 */
export async function deriveWalletKeySlow(
  password: string,
  salt: Uint8Array,
  targetMs: number = SLOW_UNLOCK_TARGET_MS,
): Promise<DerivedWalletKey> {
  assertStrongPassword(password);
  if (!Number.isFinite(targetMs) || targetMs <= 0) {
    throw new Error("Slow target time sahi hona chahiye.");
  }
  const start = Date.now();
  let material: Uint8Array<ArrayBufferLike> =
    new TextEncoder().encode(password);
  let totalIterations = 0;
  let chunkIndex = 0;
  while (Date.now() - start < targetMs) {
    const chunkSalt = new Uint8Array(salt.length + 2);
    chunkSalt.set(salt, 0);
    chunkSalt[salt.length] = chunkIndex & 0xff;
    chunkSalt[salt.length + 1] = (chunkIndex >> 8) & 0xff;
    material = await pbkdf2Bits(material, chunkSalt, SLOW_KDF_CHUNK_ITERATIONS);
    totalIterations += SLOW_KDF_CHUNK_ITERATIONS;
    chunkIndex += 1;
  }
  const key = await importAesKey(material);
  return { key, iterations: totalIterations, elapsedMs: Date.now() - start };
}

export async function deriveWalletKey(
  password: string,
  salt: Uint8Array,
  mode: KdfMode = "fast",
  targetMs: number = SLOW_UNLOCK_TARGET_MS,
): Promise<DerivedWalletKey> {
  if (mode === "slow") return deriveWalletKeySlow(password, salt, targetMs);
  if (mode === "test") {
    return deriveWalletKeyFixed(password, salt, TEST_KDF_ITERATIONS);
  }
  return deriveWalletKeyFixed(password, salt, FAST_KDF_ITERATIONS);
}

// ---------------------------------------------------------------------------
// 4. Wallet vault: private key ko encrypt karke store karo
//    Mnemonic yahan store NAHI hota — user ko ek baar dikhao, likhwao,
//    phir bhool jao. Vault me sirf keypair + address rehta hai.
// ---------------------------------------------------------------------------

export type WalletVault = {
  address: string;
  publicKeyBase64: string;
  privateKeyBase64: string;
  algorithm: string;
};

export type EncryptedWalletPayload = {
  v: 2;
  kdf: "PBKDF2-SHA-256";
  mode: KdfMode;
  iterations: number;
  salt: string;
  iv: string;
  data: string;
};

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 8192;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

/**
 * Wallet vault ko password se encrypt karo. Fast mode roz ke use ke liye
 * hai. Slow mode unlock/lock me ~2 minute lagata hai — brute-force ko
 * mehenga banane ke liye. Galat password par decrypt fail hoga.
 */
export async function encryptWalletVault(
  vault: WalletVault,
  password: string,
  mode: KdfMode = "fast",
  targetMs: number = SLOW_UNLOCK_TARGET_MS,
): Promise<string> {
  assertStrongPassword(password);
  if (!vault.address || !vault.privateKeyBase64 || !vault.publicKeyBase64) {
    throw new Error("Vault me address aur keys honi chahiye.");
  }
  const cryptoObj = getCrypto();
  const salt = secureRandomBytes(16);
  const iv = secureRandomBytes(12);
  const derived = await deriveWalletKey(password, salt, mode, targetMs);
  const encrypted = await cryptoObj.subtle.encrypt(
    { name: "AES-GCM", iv: iv as BufferSource },
    derived.key,
    new TextEncoder().encode(JSON.stringify(vault)),
  );
  const payload: EncryptedWalletPayload = {
    v: 2,
    kdf: "PBKDF2-SHA-256",
    mode,
    iterations: derived.iterations,
    salt: bytesToBase64(salt),
    iv: bytesToBase64(iv),
    data: bytesToBase64(new Uint8Array(encrypted)),
  };
  return JSON.stringify(payload);
}

/**
 * Encrypted wallet vault ko password se kholo. Slow mode se lock kiya
 * hoga to unlock me bhi utna hi time lagega. Galat password ya badla
 * hua data hoga to error aayega — vault nahi khulega.
 */
export async function decryptWalletVault(
  payloadJson: string,
  password: string,
): Promise<WalletVault> {
  assertStrongPassword(password);
  const cryptoObj = getCrypto();
  let payload: EncryptedWalletPayload;
  try {
    payload = JSON.parse(payloadJson) as EncryptedWalletPayload;
  } catch {
    throw new Error("Vault sahi format me nahi hai.");
  }
  if (
    payload.v !== 2 ||
    payload.kdf !== "PBKDF2-SHA-256" ||
    (payload.mode !== "fast" && payload.mode !== "slow" && payload.mode !== "test") ||
    !Number.isInteger(payload.iterations) ||
    payload.iterations <= 0 ||
    typeof payload.salt !== "string" ||
    typeof payload.iv !== "string" ||
    typeof payload.data !== "string"
  ) {
    throw new Error("Vault sahi format me nahi hai.");
  }
  const salt = base64ToBytes(payload.salt);
  const iv = base64ToBytes(payload.iv);
  const data = base64ToBytes(payload.data);
  const derived = await deriveWalletKeyFixed(
    password,
    salt,
    payload.iterations,
  );
  let decrypted: ArrayBuffer;
  try {
    decrypted = await cryptoObj.subtle.decrypt(
      { name: "AES-GCM", iv: iv as BufferSource },
      derived.key,
      data as BufferSource,
    );
  } catch {
    throw new Error("Vault nahi khula. Galat password ya data badla gaya hai.");
  }
  const vault = JSON.parse(new TextDecoder().decode(decrypted)) as WalletVault;
  if (!vault.address || !vault.privateKeyBase64 || !vault.publicKeyBase64) {
    throw new Error("Vault ka data sahi nahi hai.");
  }
  return vault;
}

/**
 * Demo wallet ko encrypt karke localStorage me save karo.
 * Sirf DEMO wallet ke liye hai. Ye hardware-backed storage NAHI hai —
 * asli phone me hardware storage ke liye asli Android app chahiye.
 */
export async function saveEncryptedWallet(
  key: string,
  vaultJson: string,
  password: string,
  mode: KdfMode = "fast",
  targetMs: number = SLOW_UNLOCK_TARGET_MS,
): Promise<void> {
  if (typeof window === "undefined" || !window.localStorage) {
    throw new Error("localStorage is environment me available nahi hai.");
  }
  const vault = JSON.parse(vaultJson) as WalletVault;
  const encrypted = await encryptWalletVault(vault, password, mode, targetMs);
  window.localStorage.setItem(key, encrypted);
}

/** localStorage se encrypted wallet vault padho. Data nahi hai to null milega. */
export async function loadEncryptedWallet(
  key: string,
  password: string,
): Promise<WalletVault | null> {
  if (typeof window === "undefined" || !window.localStorage) {
    throw new Error("localStorage is environment me available nahi hai.");
  }
  const stored = window.localStorage.getItem(key);
  if (!stored) return null;
  return decryptWalletVault(stored, password);
}

// Re-export: demo ledger encrypt karne ke liye purana module bhi use hota hai.
export { decryptFromLocalStorage, encryptForLocalStorage } from "./crypto-secure";
