/**
 * shared/send-checks.ts — bhejne se pehle ki jaanch (scam warning, v1)
 *
 * Ye rules sirf local heuristics hain — v1 me koi central scam database
 * nahi hai, isliye ye warning user ko sochne par majboor karti hai, par
 * har scam ko pakad nahi sakti. Ye baat UI par saaf likhi jayegi.
 * Koi bhi warning transaction ko rok nahi sakti (sirf "block" wali
 * cheezein — khud ko bhejna, balance se zyada — rokengi).
 */

import { isValidAddress } from "./wallet-secure";

export type WarningLevel = "info" | "warn" | "block";

export type SendWarning = {
  level: WarningLevel;
  text: string;
};

export type ReceiverKind = "address" | "id" | "handle" | "invalid";

export type ParsedReceiver =
  | { kind: "address"; value: string }
  | { kind: "id"; value: string }
  | { kind: "handle"; value: string }
  | { kind: "invalid"; value: string; error: string };

/**
 * Receiver ka likha hua text pehchano:
 * - "but1..." (56 chars) → address
 * - sirf number (jaise "100001") → BUT Pay ID
 * - "@sanjay" ya "sanjay" → handle
 * - kuch aur → invalid (Hindi error ke saath)
 */
export function parseReceiverInput(raw: string): ParsedReceiver {
  const value = raw.trim();
  if (!value) {
    return { kind: "invalid", value, error: "Receiver likho — address, BUT Pay ID ya naam." };
  }
  if (isValidAddress(value)) {
    return { kind: "address", value };
  }
  // "but1" se shuru ho par valid address na ho — ye handle nahi, galat
  // address hai. Ye check handle regex se PEHLE aana chahiye, warna
  // "but1abc" jaisi galti handle samajh li jayegi.
  if (value.startsWith("but1")) {
    return {
      kind: "invalid",
      value,
      error: "Address sahi nahi hai — address bilkul 56 characters ka hota hai (but1 + 52).",
    };
  }
  if (/^\d{5,}$/.test(value)) {
    return { kind: "id", value };
  }
  if (/^@?[a-z0-9][a-z0-9_.-]{2,31}$/i.test(value)) {
    return {
      kind: "handle",
      value: value.startsWith("@") ? value : `@${value}`,
    };
  }
  return {
    kind: "invalid",
    value,
    error: "Samajh nahi aaya — address (but1...), BUT Pay ID (jaise 100001) ya naam (jaise @sanjay) likho.",
  };
}

export type SendCheckInput = {
  /** Resolve kiya hua receiver address (node/registry se). */
  resolvedAddress: string | null;
  /** Apna address. */
  ownAddress: string;
  /** Bheji ja rahi rakam, bits me. */
  amountBits: number;
  /** Fee, bits me. */
  feeBits: number;
  /** Apna balance, bits me. */
  balanceBits: number;
  /** Kya ye receiver pehle se address book me hai? */
  isKnownReceiver: boolean;
};

/**
 * Bhejne se pehle ki warnings nikalo. "block" matlab bhejna rok do,
 * "warn" matlab user ko dikhao par woh phir bhi confirm kar sakta hai.
 */
export function checkSend(input: SendCheckInput): SendWarning[] {
  const warnings: SendWarning[] = [];
  const { resolvedAddress, ownAddress, amountBits, feeBits, balanceBits } = input;

  if (!resolvedAddress) {
    warnings.push({
      level: "block",
      text: "Receiver nahi mila — pehle ID/naam/address check karo.",
    });
    return warnings;
  }
  if (resolvedAddress === ownAddress) {
    warnings.push({
      level: "block",
      text: "Khud ko paise nahi bhej sakte.",
    });
    return warnings;
  }
  if (amountBits <= 0) {
    warnings.push({ level: "block", text: "Rakam 0 se zyada honi chahiye." });
    return warnings;
  }
  if (amountBits + feeBits > balanceBits) {
    warnings.push({
      level: "block",
      text: "Balance se zyada bhej rahe ho — rakam ya fee kam karo.",
    });
    return warnings;
  }
  if (amountBits > balanceBits / 2) {
    warnings.push({
      level: "warn",
      text: "Badi rakam: apne balance ka aadhe se zyada bhej rahe ho. Dobara check karo — paise bhejne ke baad wapas nahi milenge.",
    });
  }
  if (!input.isKnownReceiver) {
    warnings.push({
      level: "warn",
      text: "Pehli baar is receiver ko bhej rahe ho — address ya ID ek baar phir dhyaan se check karo. Galat address par gaya paisa wapas nahi milta.",
    });
  }
  warnings.push({
    level: "info",
    text: "Ye warning sirf tumhari madad ke liye hai — v1 me koi central scam list nahi hai, isliye anjaan link ya offer par bharosa mat karo.",
  });
  return warnings;
}

// ---------------------------------------------------------------------------
// Address book — pehle bheje gaye receivers (sirf address, koi secret nahi)
// ---------------------------------------------------------------------------

const ADDRESS_BOOK_KEY = "but-address-book-v1";

export function getKnownReceivers(): string[] {
  try {
    const raw = window.localStorage.getItem(ADDRESS_BOOK_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((x): x is string => typeof x === "string");
  } catch {
    return [];
  }
}

export function addKnownReceiver(address: string): void {
  try {
    const known = getKnownReceivers();
    if (known.includes(address)) return;
    window.localStorage.setItem(
      ADDRESS_BOOK_KEY,
      JSON.stringify([address, ...known].slice(0, 50)),
    );
  } catch {
    // Ignore — address book optional hai.
  }
}
