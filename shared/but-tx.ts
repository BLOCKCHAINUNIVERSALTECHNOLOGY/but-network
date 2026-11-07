/**
 * shared/but-tx.ts — BUT transaction banana aur sign karna (web v1)
 *
 * Rust core (`chain/src/transaction.rs`) ke saath bilkul same format:
 * - Transaction ID = SHA-256("BUT-TX|sender|receiver|amount|fee|nonce")
 *   ka hex string. Sign hota hai us hex string ke bytes par (Ed25519).
 * - Signature 64 bytes (128 hex chars), public key 32 bytes (64 hex chars).
 * - Receiver address ke saath BUT Pay ID ("100001") ya handle ("@sanjay")
 *   bhi chalega — node registry se resolve karta hai, aur signature usi
 *   likhi hui string (ID/handle) par verify hoti hai.
 *
 * Units (Sanjay ka FINAL faisla — tokenomics §1):
 * - Sabse chhoti unit "bits". **1 BUT = 1,000 bits.**
 * - Chain par amount hamesha bits (smallest unit) me jata hai.
 */

import nacl from "tweetnacl";
import { sha256Hex } from "./crypto-secure";
import { parsePrivateKeyHex } from "./wallet-secure";

/** 1 BUT = 1,000 bits (smallest unit). */
export const BITS_PER_BUT = 1000;

/** Default fee: 1 bit (0.001 BUT). Bahut kam fee — Solana jaisi. */
export const DEFAULT_FEE_BITS = 1;

export type UnsignedTx = {
  sender: string;
  receiver: string;
  amount: number;
  fee: number;
  nonce: number;
};

export type SignedTx = UnsignedTx & {
  sender_public_key_hex: string;
  signature_hex: string;
};

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

/** Transaction ka canonical payload — Rust ke `canonical_payload` jaisa. */
export function txPayload(tx: UnsignedTx): string {
  return `BUT-TX|${tx.sender}|${tx.receiver}|${tx.amount}|${tx.fee}|${tx.nonce}`;
}

/** Transaction ID = SHA-256(payload) ka hex string (64 characters). */
export async function txId(tx: UnsignedTx): Promise<string> {
  return sha256Hex(new TextEncoder().encode(txPayload(tx)));
}

/**
 * Transaction sign karo. Private key (128 hex chars) se seed + public key
 * nikalo, Ed25519 secret key banao (seed + public key, 64 bytes), aur
 * transaction ID ke bytes par sign karo. Signature tweetnacl se banti hai
 * (vetted library) — Rust core (ed25519-dalek) use verify karta hai.
 */
export async function signTransaction(
  tx: UnsignedTx,
  privateKeyHex: string,
): Promise<SignedTx> {
  const { seed, publicKey } = parsePrivateKeyHex(privateKeyHex);
  const secretKey = new Uint8Array(64);
  secretKey.set(seed, 0);
  secretKey.set(publicKey, 32);
  const id = await txId(tx);
  const signature = nacl.sign.detached(
    new TextEncoder().encode(id),
    secretKey,
  );
  return {
    ...tx,
    sender_public_key_hex: bytesToHex(publicKey),
    signature_hex: bytesToHex(signature),
  };
}

/**
 * Signature verify karo (local check — node bhi khud verify karta hai).
 * Sirf ye check karta hai ki signature is public key se is tx ID par bani
 * hai ya nahi. Chain ki baaki checks (balance, nonce) node karta hai.
 */
export async function verifyTransactionSignature(
  tx: SignedTx,
): Promise<boolean> {
  try {
    const id = await txId(tx);
    const publicKey = hexToBytes(tx.sender_public_key_hex);
    const signature = hexToBytes(tx.signature_hex);
    if (publicKey.length !== 32 || signature.length !== 64) return false;
    return nacl.sign.detached.verify(
      new TextEncoder().encode(id),
      signature,
      publicKey,
    );
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Units: BUT <-> bits (1 BUT = 1,000 bits)
// ---------------------------------------------------------------------------

export type BitsResult =
  | { ok: true; bits: number }
  | { ok: false; error: string };

/**
 * User ka likha BUT amount (jaise "12.5" ya "0.001") bits me badlo.
 * Sirf 3 decimal tak chalenge (1 bit = 0.001 BUT). Galat input par
 * Hindi error milega — kabhi silently galat number nahi banega.
 */
export function butToBits(input: string): BitsResult {
  const s = input.trim();
  if (!s) return { ok: false, error: "Rakam likho." };
  if (!/^\d+(\.\d+)?$/.test(s)) {
    return { ok: false, error: "Rakam sirf number me likho (jaise 12.5)." };
  }
  const [whole, frac = ""] = s.split(".");
  if (frac.length > 3) {
    return {
      ok: false,
      error: "Sirf 3 decimal tak likh sakte ho (1 bit = 0.001 BUT).",
    };
  }
  const bits =
    Number(whole) * BITS_PER_BUT + Number((frac + "000").slice(0, 3));
  if (!Number.isSafeInteger(bits) || bits <= 0) {
    return { ok: false, error: "Rakam 0 se zyada honi chahiye." };
  }
  return { ok: true, bits };
}

/** Bits ko BUT me dikhao — jaise 1500 bits → "1.5", 1000 → "1". */
export function bitsToBut(bits: number): string {
  const whole = Math.floor(bits / BITS_PER_BUT);
  const frac = bits % BITS_PER_BUT;
  if (frac === 0) return String(whole);
  return `${whole}.${String(frac).padStart(3, "0").replace(/0+$/, "")}`;
}

/** Bits ko Indian format me BUT ke saath dikhao — jaise "1,234.5 BUT". */
export function formatBits(bits: number): string {
  const [whole, frac] = bitsToBut(bits).split(".");
  const grouped = new Intl.NumberFormat("en-IN").format(Number(whole));
  return frac ? `${grouped}.${frac} BUT` : `${grouped} BUT`;
}
