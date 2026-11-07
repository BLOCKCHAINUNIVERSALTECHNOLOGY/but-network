/**
 * shared/crypto-secure.ts — DEMO ONLY (sirf demo ke liye)
 *
 * Ye module demo data ke liye SAHI tareeke se random number, hash aur
 * encryption deta hai. Ye Web Crypto API (CSPRNG) use karta hai.
 *
 * IMPORTANT / ZAROORI BAAT:
 * - Ye module is demo ko production wallet ya asli blockchain NAHI banata.
 * - Isse bani key sirf DEMO key hai. Asli paisa ya asli key ke liye kabhi
 *   iska use mat karo.
 * - Kabhi bhi Math.random() ko key, password ya seed ke liye use mat karo.
 *   Math.random() security ke liye safe nahi hai.
 *
 * Ye module Node.js (v19+) aur modern browser dono me chalta hai, kyunki
 * ye sirf global Web Crypto API use karta hai. Isme `node:crypto` import
 * nahi hai, isliye ye browser bundle me bhi chalega.
 */

// ---------------------------------------------------------------------------
// Helpers: base64 <-> bytes (Node aur browser dono me chalte hain)
// ---------------------------------------------------------------------------

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 8192;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function getCrypto(): Crypto {
  const cryptoObj = globalThis.crypto;
  if (!cryptoObj || !cryptoObj.getRandomValues || !cryptoObj.subtle) {
    throw new Error(
      "Secure crypto available nahi hai. Modern browser ya Node.js 19+ use karo.",
    );
  }
  return cryptoObj;
}

/** Kya is environment me secure crypto available hai? */
export function isSecureCryptoAvailable(): boolean {
  try {
    getCrypto();
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// 1. Secure random — CSPRNG (kabhi Math.random use mat karo)
// ---------------------------------------------------------------------------

/**
 * Secure random bytes banao. Length 1 se 65536 ke beech honi chahiye.
 * Ye crypto.getRandomValues() use karta hai (CSPRNG).
 */
export function secureRandomBytes(length: number): Uint8Array {
  if (!Number.isInteger(length) || length <= 0 || length > 65536) {
    throw new Error("Length 1 se 65536 ke beech honi chahiye.");
  }
  const bytes = new Uint8Array(length);
  getCrypto().getRandomValues(bytes);
  return bytes;
}

/** 256-bit (32 bytes) secure random, hex string me. Default 32 bytes hai. */
export function secureRandomHex(bytes = 32): string {
  const random = secureRandomBytes(bytes);
  return Array.from(random, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Demo transaction ID ke liye secure random ID. Home.tsx ke Math.random ki jagah ye use karo. */
export function secureRandomId(length = 8): string {
  const bytes = secureRandomBytes(Math.ceil(length / 2));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0"))
    .join("")
    .slice(0, length)
    .toUpperCase();
}

// ---------------------------------------------------------------------------
// 2. SHA-256 hash
// ---------------------------------------------------------------------------

/** Text ya bytes ka SHA-256 hash, hex string me. */
export async function sha256Hex(
  data: string | Uint8Array,
): Promise<string> {
  const bytes =
    typeof data === "string" ? new TextEncoder().encode(data) : data;
  const digest = await getCrypto().subtle.digest("SHA-256", bytes as BufferSource);
  return Array.from(new Uint8Array(digest), (b) =>
    b.toString(16).padStart(2, "0"),
  ).join("");
}

// ---------------------------------------------------------------------------
// 3. Demo keypair — SIRF DEMO KE LIYE, production key NAHI hai
// ---------------------------------------------------------------------------

export type DemoKeypair = {
  /** Kaunsa algorithm use hua: "Ed25519" ya fallback "ECDSA P-256". */
  algorithm: string;
  /** Public key, base64 (SPKI format). Ise share karna safe hai. */
  publicKeyBase64: string;
  /** Private key, base64 (PKCS8 format). Ise kabhi share mat karo, kabhi localStorage me plain mat rakho. */
  privateKeyBase64: string;
  /** Hamesha yaad rakho: ye demo key hai. */
  warning: string;
};

const DEMO_KEY_WARNING =
  "DEMO KEY — Ye sirf demo ke liye hai. Ise production wallet, asli paisa ya asli key ke liye kabhi use mat karo.";

/**
 * Demo keypair banao. Pehle Ed25519 try karta hai, agar browser support
 * na kare to ECDSA P-256 use karta hai. Private key ko memory se bahar
 * plain mat rakho. Asli wallet ke liye ye kaafi nahi hai.
 */
export async function generateDemoKeypair(): Promise<DemoKeypair> {
  const cryptoObj = getCrypto();
  let keyPair: CryptoKeyPair;
  let algorithm: string;
  try {
    // Ed25519: modern Node aur naye browser me support hai.
    keyPair = await cryptoObj.subtle.generateKey(
      { name: "Ed25519" },
      true,
      ["sign", "verify"],
    );
    algorithm = "Ed25519";
  } catch {
    // Fallback: ECDSA P-256 (har modern browser me support hai).
    keyPair = await cryptoObj.subtle.generateKey(
      { name: "ECDSA", namedCurve: "P-256" },
      true,
      ["sign", "verify"],
    );
    algorithm = "ECDSA P-256";
  }
  const publicKey = await cryptoObj.subtle.exportKey("spki", keyPair.publicKey);
  const privateKey = await cryptoObj.subtle.exportKey("pkcs8", keyPair.privateKey);
  return {
    algorithm,
    publicKeyBase64: bytesToBase64(new Uint8Array(publicKey)),
    privateKeyBase64: bytesToBase64(new Uint8Array(privateKey)),
    warning: DEMO_KEY_WARNING,
  };
}

// ---------------------------------------------------------------------------
// 4. Password se key banana (PBKDF2) + AES-GCM encrypt/decrypt
//    Ye demo data ko localStorage me encrypt karke rakhne ke liye hai.
// ---------------------------------------------------------------------------

const PBKDF2_ITERATIONS = 210_000;

async function deriveKeyFromPassword(
  password: string,
  salt: Uint8Array,
): Promise<CryptoKey> {
  const cryptoObj = getCrypto();
  if (password.length < 8) {
    throw new Error("Password kam se kam 8 character ka hona chahiye.");
  }
  const baseKey = await cryptoObj.subtle.importKey(
    "raw",
    new TextEncoder().encode(password),
    "PBKDF2",
    false,
    ["deriveKey"],
  );
  return cryptoObj.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: salt as BufferSource,
      iterations: PBKDF2_ITERATIONS,
      hash: "SHA-256",
    },
    baseKey,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
}

export type EncryptedDemoPayload = {
  v: 1;
  salt: string;
  iv: string;
  data: string;
};

/**
 * Demo text ko password se encrypt karo. Har baar naya random salt aur
 * naya random IV banta hai. Result JSON string hai, use localStorage me
 * save kar sakte ho. Password bhool gaye to data kabhi nahi khulega.
 */
export async function encryptForLocalStorage(
  plaintext: string,
  password: string,
): Promise<string> {
  const cryptoObj = getCrypto();
  const salt = secureRandomBytes(16);
  const iv = secureRandomBytes(12);
  const key = await deriveKeyFromPassword(password, salt);
  const encrypted = await cryptoObj.subtle.encrypt(
    { name: "AES-GCM", iv: iv as BufferSource },
    key,
    new TextEncoder().encode(plaintext),
  );
  const payload: EncryptedDemoPayload = {
    v: 1,
    salt: bytesToBase64(salt),
    iv: bytesToBase64(iv),
    data: bytesToBase64(new Uint8Array(encrypted)),
  };
  return JSON.stringify(payload);
}

/**
 * Encrypt kiya hua demo data wapas kholo. Galat password ya badla hua
 * data hoga to error aayega — data nahi milega.
 */
export async function decryptFromLocalStorage(
  payloadJson: string,
  password: string,
): Promise<string> {
  const cryptoObj = getCrypto();
  let payload: EncryptedDemoPayload;
  try {
    payload = JSON.parse(payloadJson) as EncryptedDemoPayload;
  } catch {
    throw new Error("Payload sahi format me nahi hai.");
  }
  if (
    payload.v !== 1 ||
    typeof payload.salt !== "string" ||
    typeof payload.iv !== "string" ||
    typeof payload.data !== "string"
  ) {
    throw new Error("Payload sahi format me nahi hai.");
  }
  const salt = base64ToBytes(payload.salt);
  const iv = base64ToBytes(payload.iv);
  const data = base64ToBytes(payload.data);
  const key = await deriveKeyFromPassword(password, salt);
  let decrypted: ArrayBuffer;
  try {
    decrypted = await cryptoObj.subtle.decrypt(
      { name: "AES-GCM", iv: iv as BufferSource },
      key,
      data as BufferSource,
    );
  } catch {
    throw new Error("Decrypt nahi hua. Galat password ya data badla gaya hai.");
  }
  return new TextDecoder().decode(decrypted);
}

/**
 * Demo data ko encrypt karke localStorage me save karo.
 * Sirf browser me kaam karta hai. Sirf DEMO data ke liye hai.
 */
export async function saveEncryptedDemoData(
  key: string,
  plaintext: string,
  password: string,
): Promise<void> {
  if (typeof window === "undefined" || !window.localStorage) {
    throw new Error("localStorage is environment me available nahi hai.");
  }
  const encrypted = await encryptForLocalStorage(plaintext, password);
  window.localStorage.setItem(key, encrypted);
}

/**
 * localStorage se encrypt kiya hua demo data padho.
 * Data nahi hai to null milega. Galat password hoga to error aayega.
 */
export async function loadEncryptedDemoData(
  key: string,
  password: string,
): Promise<string | null> {
  if (typeof window === "undefined" || !window.localStorage) {
    throw new Error("localStorage is environment me available nahi hai.");
  }
  const stored = window.localStorage.getItem(key);
  if (!stored) return null;
  return decryptFromLocalStorage(stored, password);
}
