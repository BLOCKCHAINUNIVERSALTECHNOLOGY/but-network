/**
 * shared/webauthn.ts — Biometric unlock (WebAuthn PRF, web v1)
 *
 * Kaise kaam karta hai:
 * - Phone/computer ka fingerprint/Face ID ek WebAuthn credential banata
 *   hai. Credential ke saath PRF extension se 32-byte secret milta hai.
 * - Us secret se wallet password ko AES-GCM se encrypt karke (wrapped)
 *   localStorage me rakhte hain. Password kabhi plain nahi rehta.
 * - Unlock karte waqt fingerprint se wahi PRF secret dobara milta hai,
 *   wrapped password khulta hai, phir vault password se khulta hai.
 * - Private key ya seed phrase WebAuthn se kabhi nahi judti — sirf
 *   password wrap hota hai.
 *
 * LIMIT (imaandari se):
 * - PRF extension har browser/device me nahi hota (Chrome/Android me
 *   zyada, Safari/iOS me kam). Na ho to biometric button dikhega hi nahi
 *   — password se unlock karna padega.
 * - Ye web unlock hai, hardware-backed Android Keystore NAHI hai. Asli
 *   hardware-backed biometric Android app me aayega.
 * - Is code ko is sandbox me verify nahi kiya ja sakta (yahan koi real
 *   browser/biometric hardware nahi hai) — real device par test karna
 *   baaki hai. Password unlock hi verified rasta hai.
 */

const BIOMETRIC_KEY = "but-biometric-v1";

type BiometricStored = {
  credentialId: string;
  salt: string;
  iv: string;
  data: string;
};

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 8192;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function getCrypto(): Crypto {
  const cryptoObj = globalThis.crypto;
  if (!cryptoObj || !cryptoObj.subtle) {
    throw new Error("Secure crypto available nahi hai.");
  }
  return cryptoObj;
}

async function wrapWithPrfSecret(
  prfSecret: Uint8Array,
  password: string,
): Promise<{ salt: string; iv: string; data: string }> {
  const cryptoObj = getCrypto();
  const keyMaterial = await cryptoObj.subtle.digest(
    "SHA-256",
    prfSecret as BufferSource,
  );
  const key = await cryptoObj.subtle.importKey(
    "raw",
    keyMaterial,
    "AES-GCM",
    false,
    ["encrypt"],
  );
  const salt = new Uint8Array(16);
  cryptoObj.getRandomValues(salt);
  const iv = new Uint8Array(12);
  cryptoObj.getRandomValues(iv);
  const encrypted = await cryptoObj.subtle.encrypt(
    { name: "AES-GCM", iv: iv as BufferSource },
    key,
    new TextEncoder().encode(password),
  );
  return {
    salt: bytesToBase64(salt),
    iv: bytesToBase64(iv),
    data: bytesToBase64(new Uint8Array(encrypted)),
  };
}

async function unwrapWithPrfSecret(
  prfSecret: Uint8Array,
  stored: BiometricStored,
): Promise<string> {
  const cryptoObj = getCrypto();
  const keyMaterial = await cryptoObj.subtle.digest(
    "SHA-256",
    prfSecret as BufferSource,
  );
  const key = await cryptoObj.subtle.importKey(
    "raw",
    keyMaterial,
    "AES-GCM",
    false,
    ["decrypt"],
  );
  const decrypted = await cryptoObj.subtle.decrypt(
    { name: "AES-GCM", iv: base64ToBytes(stored.iv) as BufferSource },
    key,
    base64ToBytes(stored.data) as BufferSource,
  );
  return new TextDecoder().decode(decrypted);
}

function getPrfResult(
  credential: PublicKeyCredential,
): Uint8Array | null {
  const ext = credential.getClientExtensionResults() as unknown as {
    prf?: { results?: { first?: ArrayBuffer } };
  };
  const first = ext?.prf?.results?.first;
  if (!first) return null;
  return new Uint8Array(first);
}

/** Is browser me WebAuthn hai? (Sirf support check — biometric ki guarantee nahi.) */
export function isWebAuthnAvailable(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.PublicKeyCredential !== "undefined"
  );
}

/**
 * Kya biometric (PRF) unlock available hai? Browser capability check.
 * Na ho to false — UI button dikhayega hi nahi.
 */
export async function isBiometricUnlockAvailable(): Promise<boolean> {
  try {
    if (!isWebAuthnAvailable()) return false;
    const caps = await (
      window.PublicKeyCredential as unknown as {
        getClientCapabilities?: () => Promise<{ prf?: boolean }>;
      }
    ).getClientCapabilities?.();
    if (caps && typeof caps.prf === "boolean") return caps.prf;
    // Capability API na ho to: platform authenticator check karo.
    return await window.PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
  } catch {
    return false;
  }
}

/** Kya biometric unlock pehle se setup hai (wrapped password saved hai)? */
export function isBiometricSetup(): boolean {
  try {
    return window.localStorage.getItem(BIOMETRIC_KEY) !== null;
  } catch {
    return false;
  }
}

/**
 * Biometric setup karo: fingerprint se credential banao, PRF secret se
 * password wrap karke save karo. Pehle se setup hai to dobara setup
 * karne par purana replace hoga.
 */
export async function setupBiometricUnlock(
  password: string,
): Promise<void> {
  if (!isWebAuthnAvailable()) {
    throw new Error("Is browser me biometric unlock nahi hai.");
  }
  const challenge = new Uint8Array(32);
  getCrypto().getRandomValues(challenge);
  const userId = new Uint8Array(16);
  getCrypto().getRandomValues(userId);
  const prfSalt = new Uint8Array(32);
  getCrypto().getRandomValues(prfSalt);
  const credential = (await navigator.credentials.create({
    publicKey: {
      challenge: challenge as BufferSource,
      rp: { name: "BUT Wallet", id: window.location.hostname },
      user: {
        id: userId as BufferSource,
        name: "but-wallet-user",
        displayName: "BUT Wallet",
      },
      pubKeyCredParams: [{ type: "public-key", alg: -7 }],
      authenticatorSelection: {
        authenticatorAttachment: "platform",
        userVerification: "required",
      },
      timeout: 60000,
      extensions: { prf: { eval: { first: prfSalt as BufferSource } } },
    } as PublicKeyCredentialCreationOptions,
  })) as PublicKeyCredential | null;
  if (!credential) {
    throw new Error("Biometric setup cancel ho gaya.");
  }
  const prfSecret = getPrfResult(credential);
  if (!prfSecret) {
    throw new Error(
      "Is device par PRF support nahi hai — biometric unlock nahi ho sakta.",
    );
  }
  const wrapped = await wrapWithPrfSecret(prfSecret, password);
  const stored: BiometricStored = {
    credentialId: bytesToBase64(new Uint8Array(credential.rawId)),
    ...wrapped,
  };
  window.localStorage.setItem(BIOMETRIC_KEY, JSON.stringify(stored));
}

/**
 * Biometric se unlock karo: fingerprint maango, PRF secret se wrapped
 * password kholo. Password milega to vault usse khulega. Cancel ya fail
 * hone par null milega (password se unlock karo).
 */
export async function unlockWithBiometric(): Promise<string | null> {
  try {
    const raw = window.localStorage.getItem(BIOMETRIC_KEY);
    if (!raw) return null;
    const stored = JSON.parse(raw) as BiometricStored;
    const challenge = new Uint8Array(32);
    getCrypto().getRandomValues(challenge);
    const credential = (await navigator.credentials.get({
      publicKey: {
        challenge: challenge as BufferSource,
        allowCredentials: [
          {
            type: "public-key",
            id: base64ToBytes(stored.credentialId) as BufferSource,
          },
        ],
        userVerification: "required",
        timeout: 60000,
        extensions: {
          prf: { eval: { first: base64ToBytes(stored.salt) as BufferSource } },
        },
      } as PublicKeyCredentialRequestOptions,
    })) as PublicKeyCredential | null;
    if (!credential) return null;
    const prfSecret = getPrfResult(credential);
    if (!prfSecret) return null;
    return await unwrapWithPrfSecret(prfSecret, stored);
  } catch {
    return null;
  }
}

/** Biometric setup hatao (wrapped password delete karo). */
export function clearBiometricUnlock(): void {
  try {
    window.localStorage.removeItem(BIOMETRIC_KEY);
  } catch {
    // Ignore.
  }
}

// Test ke liye export (PRF secret fake karke wrap/unwrap check hota hai).
export const __test = { wrapWithPrfSecret, unwrapWithPrfSecret };
