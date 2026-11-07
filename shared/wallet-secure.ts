/**
 * shared/wallet-secure.ts — BUT Wallet (real wallet, web v1)
 *
 * Ye module BUT wallet banane ke liye hai: mnemonic, keypair, address
 * aur password se encrypt kiya hua wallet vault.
 *
 * ZAROORI SACH — pehle padho:
 * - Mnemonic REAL BIP39 hai: 24 words, official 2048-word English list
 *   (shared/bip39-wordlist.ts, source: bitcoin/bips). Lekin implementation
 *   haath se likha hai — vetted `bip39` library use NAHI hui.
 *   Production se pehle vetted library + independent audit CHAHIYE.
 * - **Mnemonic HI wallet hai** — keypair mnemonic se banta hai (BIP39 seed
 *   ke pehle 32 bytes se Ed25519 keypair, bilkul waisa hi jaise Rust core
 *   `chain/src/wallet.rs` me hai). Wahi 24 words hamesha wahi address
 *   denge — isliye seed se wallet wapas laya ja sakta hai. (Purane demo me
 *   mnemonic aur keypair alag-alag bante the — wo galti ab theek hai.)
 * - Ed25519 signing ke liye vetted `tweetnacl` library use hoti hai
 *   (pure-JS, audited). Signing ka code `shared/but-tx.ts` me hai.
 * - Slow unlock (2 minute) brute-force ko mehenga banata hai, par ye
 *   wallet ko "unbreakable" NAHI banata. Kamzor password (jaise "123456")
 *   phir bhi dictionary attack se toot jayega. Strong, unique password
 *   (kam se kam 12 akshar) hamesha zaroori hai.
 * - Ye WEB wallet hai. Ye Android Keystore / iPhone Secure Enclave ko use
 *   NAHI kar sakta. Asli phone me hardware-backed storage ke liye asli
 *   Android app chahiye (EncryptedSharedPreferences + Android Keystore).
 *   Web me encrypted vault localStorage me rehta hai — ye hardware
 *   storage NAHI hai. Slow KDF yahan uski kami ko thoda kam karta hai.
 * - Koi independent audit abhi nahi hua hai. Asli paisa tabhi rakho jab
 *   audit ho aur tum seed phrase ko kagaz par surakshit rakh sako.
 *
 * 1024-BIT WALI BAAT — SAHI SAMAJH:
 * - "1024-bit wallet" agar RSA-1024 matlab hai to wo GALAT raasta hai.
 *   NIST ke hisaab se RSA-1024 sirf ~80-bit security deta hai, deprecated
 *   hai aur disallowed hai. 256-bit ECC (Ed25519) lagbhag 3072-bit RSA ke
 *   barabar hai (~128-bit security) — matlab chhoti key, zyada suraksha.
 *   Isliye BUT Ed25519 (256-bit) use karta hai, RSA-1024 NAHI.
 * - Private key 128 characters ki hai = 64 bytes stored data (32-byte
 *   seed + 32-byte public key). Iska matlab "1024-bit security" NAHI hai —
 *   asli secret 256-bit hai. Bada number likhna suraksha nahi badhata.
 *
 * FIXED LENGTH SPEC (Sanjay ka faisla — na isse kam, na isse zyada):
 * - Seed phrase: bilkul 24 words (real BIP39).
 * - Private key: bilkul 128 characters (hex, 64 bytes).
 * - Wallet address: bilkul 56 characters ("but1" + 52 hex characters).
 * - Galat length wallet reject karega — neeche validation functions dekho.
 *
 * Security rules (kabhi mat todo):
 * - Random ke liye sirf CSPRNG (crypto.getRandomValues). Math.random
 *   kabhi key, seed ya password ke liye use mat karo.
 * - Private key aur mnemonic ko kabhi plain localStorage me mat rakho.
 * - Password ko kabhi code me, kabhi localStorage me mat rakho.
 *   Password sirf user ki memory me aur unlock ke waqt RAM me rehta hai.
 */

import nacl from "tweetnacl";
import { BIP39_WORDLIST } from "./bip39-wordlist";

/** BIP39 official English wordlist (2048 words) — bahar se bhi use ho sake. */
export { BIP39_WORDLIST };
import { secureRandomBytes, sha256Hex } from "./crypto-secure";

// ---------------------------------------------------------------------------
// 0. Fixed-length spec — Sanjay ka faisla (na kam, na zyada)
// ---------------------------------------------------------------------------

/** Seed phrase hamesha bilkul 24 words ki hogi (real BIP39). */
export const MNEMONIC_WORD_COUNT = 24;

/** Private key hamesha bilkul 128 characters ki hogi (hex, 64 bytes). */
export const PRIVATE_KEY_LENGTH = 128;

/** Wallet address hamesha bilkul 56 characters ka hoga. */
export const ADDRESS_LENGTH = 56;

/** Address hamesha "but1" se shuru hoga, uske baad 52 hex characters. */
export const ADDRESS_PREFIX = "but1";

/** BIP39 entropy: 24 words ke liye 256 bit (32 bytes). */
const BIP39_ENTROPY_BYTES = 32;

/** BIP39 checksum: entropy ke har 32 bit par 1 bit = 8 bit. */
const BIP39_CHECKSUM_BITS = 8;

const WORD_INDEX = new Map(BIP39_WORDLIST.map((w, i) => [w, i]));

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

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

async function sha256Bytes(data: Uint8Array): Promise<Uint8Array> {
  const digest = await getCrypto().subtle.digest(
    "SHA-256",
    data as BufferSource,
  );
  return new Uint8Array(digest);
}

// ---------------------------------------------------------------------------
// 1. Fixed-length validation — galat length reject hogi
// ---------------------------------------------------------------------------

/** Private key sahi format me hai? Bilkul 128 hex characters. */
export function isValidPrivateKeyFormat(privateKeyHex: unknown): boolean {
  return (
    typeof privateKeyHex === "string" &&
    privateKeyHex.length === PRIVATE_KEY_LENGTH &&
    /^[0-9a-f]{128}$/.test(privateKeyHex)
  );
}

/** Address sahi format me hai? Bilkul 56 characters, "but1" + 52 hex. */
export function isValidAddress(address: unknown): boolean {
  return (
    typeof address === "string" &&
    address.length === ADDRESS_LENGTH &&
    address.startsWith(ADDRESS_PREFIX) &&
    /^[0-9a-f]{52}$/.test(address.slice(ADDRESS_PREFIX.length))
  );
}

/**
 * Wallet ki fixed lengths check karo. Galat ho to Hindi error ke saath
 * throw karo. Har wallet banane / kholne se pehle ye check chalti hai.
 */
export function assertFixedLengths(
  address: unknown,
  privateKeyHex: unknown,
): void {
  if (!isValidAddress(address)) {
    throw new Error(
      `Address bilkul ${ADDRESS_LENGTH} characters ka hona chahiye ("but1" + 52 hex characters).`,
    );
  }
  if (!isValidPrivateKeyFormat(privateKeyHex)) {
    throw new Error(
      `Private key bilkul ${PRIVATE_KEY_LENGTH} characters ki honi chahiye (hex).`,
    );
  }
}

/**
 * 128-character private key ko kholo: pehle 64 characters = 32-byte seed,
 * aakhri 64 characters = 32-byte public key. Format galat ho to error.
 */
export function parsePrivateKeyHex(privateKeyHex: string): {
  seed: Uint8Array;
  publicKey: Uint8Array;
} {
  if (!isValidPrivateKeyFormat(privateKeyHex)) {
    throw new Error(
      `Private key bilkul ${PRIVATE_KEY_LENGTH} characters ki honi chahiye (hex).`,
    );
  }
  return {
    seed: hexToBytes(privateKeyHex.slice(0, 64)),
    publicKey: hexToBytes(privateKeyHex.slice(64)),
  };
}

// ---------------------------------------------------------------------------
// 2. Mnemonic — REAL BIP39, bilkul 24 words
//    32 bytes entropy + 8-bit checksum = 264 bits = 24 x 11-bit words.
// ---------------------------------------------------------------------------

function bitsToWords(bits: number[]): string {
  const words: string[] = [];
  for (let i = 0; i < bits.length; i += 11) {
    let index = 0;
    for (let j = 0; j < 11; j++) {
      index = (index << 1) | bits[i + j];
    }
    words.push(BIP39_WORDLIST[index]);
  }
  return words.join(" ");
}

/**
 * Secure entropy (32 bytes, CSPRNG) se real BIP39 mnemonic banao.
 * Hamesha bilkul 24 words. Entropy hamesha CSPRNG se aati hai.
 */
export async function generateMnemonic(): Promise<string> {
  const entropy = secureRandomBytes(BIP39_ENTROPY_BYTES);
  const checksum = await sha256Bytes(entropy);
  // 256 entropy bits + checksum ke pehle 8 bits = 264 bits.
  const bits: number[] = [];
  for (const byte of entropy) {
    for (let i = 7; i >= 0; i--) bits.push((byte >> i) & 1);
  }
  for (let i = 7; i >= 0; i--) bits.push((checksum[0] >> i) & 1);
  return bitsToWords(bits);
}

/** Mnemonic sahi hai ya nahi — bilkul 24 words, list, aur checksum check. */
export async function validateMnemonic(mnemonic: string): Promise<boolean> {
  if (typeof mnemonic !== "string") return false;
  const words = mnemonic.trim().split(/\s+/);
  if (words.length !== MNEMONIC_WORD_COUNT) return false;
  const bits: number[] = [];
  for (const word of words) {
    const index = WORD_INDEX.get(word);
    if (index === undefined) return false;
    for (let i = 10; i >= 0; i--) bits.push((index >> i) & 1);
  }
  // Pehle 256 bits = entropy, aakhri 8 bits = checksum.
  const entropy = new Uint8Array(BIP39_ENTROPY_BYTES);
  for (let i = 0; i < 256; i++) {
    if (bits[i] === 1) entropy[Math.floor(i / 8)] |= 1 << (7 - (i % 8));
  }
  const checksum = await sha256Bytes(entropy);
  for (let i = 0; i < BIP39_CHECKSUM_BITS; i++) {
    if (bits[256 + i] !== ((checksum[0] >> (7 - i)) & 1)) return false;
  }
  return true;
}

/**
 * Mnemonic se seed banao — REAL BIP39: PBKDF2-HMAC-SHA-512, salt
 * "mnemonic" + passphrase, 2048 iterations, 64 bytes. Ye seed dusre
 * BIP39 wallet se bhi same hoga (agar passphrase same ho).
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
      salt: new TextEncoder().encode(`mnemonic${passphrase}`),
      iterations: 2048,
      hash: "SHA-512",
    },
    baseKey,
    512,
  );
  return new Uint8Array(bits);
}

// ---------------------------------------------------------------------------
// 3. Wallet banana: keypair + mnemonic + address (fixed lengths)
// ---------------------------------------------------------------------------

export type ButWallet = {
  /** Address: bilkul 56 characters — "but1" + SHA-256(public key) ke pehle 52 hex characters. */
  address: string;
  /** Public key, base64 (SPKI). Share karna safe hai. */
  publicKeyBase64: string;
  /**
   * Private key, bilkul 128 hex characters = 64 bytes:
   * pehle 32 bytes seed + aakhri 32 bytes public key.
   * Kabhi share mat karo, kabhi plain mat rakho.
   */
  privateKeyHex: string;
  /** Kaunsa algorithm: hamesha "Ed25519" (fixed-length wallet ke liye). */
  algorithm: string;
  /** Mnemonic (bilkul 24 words, real BIP39). Sirf ek baar dikhao, user ko likhne ko kaho, phir kabhi store mat karo. */
  mnemonic: string;
  warning: string;
};

/** Purana naam — compatibility ke liye alias (naya code ButWallet use kare). */
export type DemoWallet = ButWallet;

const WALLET_WARNING =
  "Mnemonic aur private key ko kisi ko mat dikhao. Seed phrase sirf kagaz par likho — photo / cloud / chat me kabhi nahi. Is wallet ka abhi independent audit nahi hua hai.";

/** Ed25519 SPKI DER ka shuruwaati hissa (12 bytes), uske baad 32-byte public key. */
const ED25519_SPKI_PREFIX = [
  0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

/**
 * 24-word mnemonic se wallet banao — DETERMINISTIC: wahi words, wahi
 * address, wahi private key. Rust core (`chain/src/wallet.rs`) ke saath
 * bilkul same derivation:
 *   seed = PBKDF2-HMAC-SHA512(mnemonic, salt "mnemonic", 2048 iters, 64 bytes)
 *   secret = seed[0..32] → Ed25519 keypair (tweetnacl)
 *   address = "but1" + sha256(public key) ke pehle 52 hex characters
 *   private key = hex(secret) + hex(public key) — bilkul 128 characters
 */
export async function walletFromMnemonic(
  mnemonic: string,
): Promise<ButWallet> {
  const seed64 = await mnemonicToSeed(mnemonic);
  const secretSeed = seed64.slice(0, 32);
  const keypair = nacl.sign.keyPair.fromSeed(secretSeed);
  const publicKeyRaw = keypair.publicKey;
  const privateKeyHex = bytesToHex(secretSeed) + bytesToHex(publicKeyRaw);
  const addressHash = await sha256Hex(publicKeyRaw);
  const address = `${ADDRESS_PREFIX}${addressHash.slice(0, 52)}`;
  const publicKeyBase64 = bytesToBase64(
    new Uint8Array([...ED25519_SPKI_PREFIX, ...publicKeyRaw]),
  );
  assertFixedLengths(address, privateKeyHex);
  return {
    address,
    publicKeyBase64,
    privateKeyHex,
    algorithm: "Ed25519",
    mnemonic: mnemonic.trim().split(/\s+/).join(" "),
    warning: WALLET_WARNING,
  };
}

/**
 * Naya wallet banao: CSPRNG se 24-word BIP39 mnemonic, phir usi mnemonic
 * se keypair (walletFromMnemonic). Private key bilkul 128 hex characters
 * ki hogi, address bilkul 56 characters ka. Fixed lengths check karke hi
 * return hoga.
 */
export async function generateWallet(): Promise<ButWallet> {
  const mnemonic = await generateMnemonic();
  return walletFromMnemonic(mnemonic);
}

// ---------------------------------------------------------------------------
// 4. KDF: fast / slow / test — password se AES-GCM key banana
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
// 5. Wallet vault: private key ko encrypt karke store karo
//    Mnemonic yahan store NAHI hota — user ko ek baar dikhao, likhwao,
//    phir bhool jao. Vault me sirf keypair + address rehta hai.
//    Payload v3 hai — purana v2 vault isse NAHI khulega.
// ---------------------------------------------------------------------------

export type WalletVault = {
  address: string;
  publicKeyBase64: string;
  /** Private key, bilkul 128 hex characters. */
  privateKeyHex: string;
  algorithm: string;
};

export type EncryptedWalletPayload = {
  v: 3;
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
 * Encrypt karne se pehle fixed lengths check hoti hain.
 */
export async function encryptWalletVault(
  vault: WalletVault,
  password: string,
  mode: KdfMode = "fast",
  targetMs: number = SLOW_UNLOCK_TARGET_MS,
): Promise<string> {
  assertStrongPassword(password);
  if (!vault.address || !vault.privateKeyHex || !vault.publicKeyBase64) {
    throw new Error("Vault me address aur keys honi chahiye.");
  }
  assertFixedLengths(vault.address, vault.privateKeyHex);
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
    v: 3,
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
 * hua data hoga to error aayega — vault nahi khulega. Khulne ke baad
 * fixed lengths phir se check hoti hain.
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
    payload.v !== 3 ||
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
  if (!vault.address || !vault.privateKeyHex || !vault.publicKeyBase64) {
    throw new Error("Vault ka data sahi nahi hai.");
  }
  assertFixedLengths(vault.address, vault.privateKeyHex);
  return vault;
}

/**
 * Wallet vault ko encrypt karke localStorage me save karo.
 * Ye hardware-backed storage NAHI hai — asli phone me hardware storage
 * ke liye asli Android app chahiye.
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
