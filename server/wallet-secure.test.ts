import { describe, expect, it } from "vitest";
import {
  ADDRESS_LENGTH,
  ADDRESS_PREFIX,
  BIP39_WORDLIST,
  MNEMONIC_WORD_COUNT,
  PRIVATE_KEY_LENGTH,
  assertFixedLengths,
  decryptWalletVault,
  deriveWalletKey,
  encryptWalletVault,
  generateMnemonic,
  generateWallet,
  isValidAddress,
  isValidPrivateKeyFormat,
  mnemonicToSeed,
  parsePrivateKeyHex,
  validateMnemonic,
  walletFromMnemonic,
} from "../shared/wallet-secure";
import { BIP39_WORDLIST as WORDLIST } from "../shared/bip39-wordlist";

const PASSWORD = "demo-password-123";

/** BIP39 ka mashhoor test vector: 32 zero bytes entropy (24 words). */
const VECTOR_MNEMONIC = `${"abandon ".repeat(23)}art`;

describe("wallet-secure — fixed-length spec (24 / 128 / 56)", () => {
  it("BIP39 wordlist me 2048 unique words hain", () => {
    expect(BIP39_WORDLIST.length).toBe(2048);
    expect(new Set(BIP39_WORDLIST).size).toBe(2048);
    expect(WORDLIST.length).toBe(2048);
  });

  it("generateWallet: address bilkul 56, private key bilkul 128, mnemonic bilkul 24", async () => {
    const a = await generateWallet();
    const b = await generateWallet();
    expect(a.address).toMatch(/^but1[0-9a-f]{52}$/);
    expect(a.address.length).toBe(ADDRESS_LENGTH);
    expect(a.address.startsWith(ADDRESS_PREFIX)).toBe(true);
    expect(a.address).not.toBe(b.address);
    expect(a.privateKeyHex).toMatch(/^[0-9a-f]{128}$/);
    expect(a.privateKeyHex.length).toBe(PRIVATE_KEY_LENGTH);
    expect(a.privateKeyHex).not.toBe(b.privateKeyHex);
    expect(a.publicKeyBase64).not.toBe(b.publicKeyBase64);
    expect(a.algorithm).toBe("Ed25519");
    expect(a.mnemonic.split(" ").length).toBe(MNEMONIC_WORD_COUNT);
    expect(a.mnemonic).not.toBe(b.mnemonic);
    // Real wallet — warning me "DEMO" nahi hona chahiye.
    expect(a.warning).not.toContain("DEMO");
    expect(a.warning).toContain("audit");
  });

  it("wahi mnemonic hamesha wahi wallet deta hai (deterministic — seed se wallet wapas milega)", async () => {
    const a = await generateWallet();
    const again = await walletFromMnemonic(a.mnemonic);
    expect(again.address).toBe(a.address);
    expect(again.privateKeyHex).toBe(a.privateKeyHex);
    expect(again.publicKeyBase64).toBe(a.publicKeyBase64);
    const other = await generateWallet();
    expect(other.address).not.toBe(a.address);
  });

  it("BIP39 test vector 'abandon' x23 + 'art' — Rust core jaisa hi address/private key", async () => {
    // Ye values Rust core (chain/examples/print_wallet) se li gayi hain —
    // TS aur Rust ka derivation bilkul same hona chahiye.
    const w = await walletFromMnemonic(VECTOR_MNEMONIC);
    expect(w.address).toBe(
      "but1e28c3608979d45d2c7dc74b1c19519e54ce8961c882f79b20b6b",
    );
    expect(w.privateKeyHex).toBe(
      "408b285c123836004f4b8842c89324c1f01382450c0d439af345ba7fc49acf701de352e44cd333672593f2334a730e180aaf290de89aa16d480de594e34e2961",
    );
    expect(w.publicKeyBase64).toMatch(/^[A-Za-z0-9+/]+={0,2}$/);
  });

  it("mnemonic bilkul 24 words ka hota hai, saare BIP39 list me, checksum sahi", async () => {
    const mnemonic = await generateMnemonic();
    const words = mnemonic.split(" ");
    expect(words.length).toBe(24);
    for (const word of words) {
      expect(BIP39_WORDLIST).toContain(word);
    }
    expect(await validateMnemonic(mnemonic)).toBe(true);
  });

  it("BIP39 test vector: 'abandon' x23 + 'art' valid hai (24 words)", async () => {
    expect(await validateMnemonic(VECTOR_MNEMONIC)).toBe(true);
    const seed = await mnemonicToSeed(VECTOR_MNEMONIC, "TREZOR");
    expect(seed.length).toBe(64);
    const seed2 = await mnemonicToSeed(VECTOR_MNEMONIC, "TREZOR");
    expect(Buffer.from(seed).equals(Buffer.from(seed2))).toBe(true);
  });

  it("galat mnemonic reject hota hai (galat word, galat length, galat checksum)", async () => {
    const mnemonic = await generateMnemonic();
    const words = mnemonic.split(" ");
    // Galat word
    const badWord = words.slice();
    badWord[0] = badWord[0] === "abandon" ? "ability" : "abandon";
    expect(await validateMnemonic(badWord.join(" "))).toBe(false);
    // Galat length — 23 words aur 25 words dono reject
    expect(await validateMnemonic(words.slice(0, 23).join(" "))).toBe(false);
    expect(await validateMnemonic([...words, "abandon"].join(" "))).toBe(false);
    // Purana 33-word style bhi reject (ab demo me nahi chalega)
    expect(await validateMnemonic(words.slice(0, 10).join(" "))).toBe(false);
    // Galat checksum (aakhri word badlo)
    const badChecksum = words.slice();
    const last = badChecksum[badChecksum.length - 1];
    badChecksum[badChecksum.length - 1] =
      last === "abandon" ? "ability" : "abandon";
    expect(await validateMnemonic(badChecksum.join(" "))).toBe(false);
    // Khaali string
    expect(await validateMnemonic("")).toBe(false);
  });

  it("mnemonicToSeed deterministic hai aur passphrase se badalta hai", async () => {
    const mnemonic = await generateMnemonic();
    const seed1 = await mnemonicToSeed(mnemonic);
    const seed2 = await mnemonicToSeed(mnemonic);
    const seed3 = await mnemonicToSeed(mnemonic, "extra-word");
    expect(seed1.length).toBe(64);
    expect(Buffer.from(seed1).equals(Buffer.from(seed2))).toBe(true);
    expect(Buffer.from(seed1).equals(Buffer.from(seed3))).toBe(false);
  });

  it("fixed-length validation: sahi length pass, galat length reject", async () => {
    const wallet = await generateWallet();
    // Sahi
    expect(isValidAddress(wallet.address)).toBe(true);
    expect(isValidPrivateKeyFormat(wallet.privateKeyHex)).toBe(true);
    expect(() =>
      assertFixedLengths(wallet.address, wallet.privateKeyHex),
    ).not.toThrow();
    // Address: 55 aur 57 characters reject
    expect(isValidAddress(wallet.address.slice(0, 55))).toBe(false);
    expect(isValidAddress(`${wallet.address}a`)).toBe(false);
    // Galat prefix reject
    expect(isValidAddress(`but2${wallet.address.slice(4)}`)).toBe(false);
    // Non-hex reject
    expect(isValidAddress(`but1${"g".repeat(52)}`)).toBe(false);
    // Private key: 127 aur 129 characters reject
    expect(isValidPrivateKeyFormat(wallet.privateKeyHex.slice(0, 127))).toBe(
      false,
    );
    expect(isValidPrivateKeyFormat(`${wallet.privateKeyHex}a`)).toBe(false);
    expect(isValidPrivateKeyFormat("g".repeat(128))).toBe(false);
    expect(isValidPrivateKeyFormat("")).toBe(false);
    // assertFixedLengths galat par throw kare
    expect(() =>
      assertFixedLengths(wallet.address.slice(0, 55), wallet.privateKeyHex),
    ).toThrow();
    expect(() =>
      assertFixedLengths(wallet.address, wallet.privateKeyHex.slice(0, 127)),
    ).toThrow();
  });

  it("parsePrivateKeyHex: 128 chars se 32-byte seed + 32-byte public key", async () => {
    const wallet = await generateWallet();
    const parsed = parsePrivateKeyHex(wallet.privateKeyHex);
    expect(parsed.seed.length).toBe(32);
    expect(parsed.publicKey.length).toBe(32);
    expect(() => parsePrivateKeyHex(wallet.privateKeyHex.slice(0, 127))).toThrow();
  });

  it("vault encrypt/decrypt round-trip karta hai (fast mode)", async () => {
    const wallet = await generateWallet();
    const vault = {
      address: wallet.address,
      publicKeyBase64: wallet.publicKeyBase64,
      privateKeyHex: wallet.privateKeyHex,
      algorithm: wallet.algorithm,
    };
    const encrypted = await encryptWalletVault(vault, PASSWORD, "fast");
    const back = await decryptWalletVault(encrypted, PASSWORD);
    expect(back.address).toBe(vault.address);
    expect(back.privateKeyHex).toBe(vault.privateKeyHex);
    expect(back.publicKeyBase64).toBe(vault.publicKeyBase64);
  });

  it("vault galat-length address ya private key se encrypt nahi hota", async () => {
    const wallet = await generateWallet();
    await expect(
      encryptWalletVault(
        {
          address: wallet.address.slice(0, 55),
          publicKeyBase64: wallet.publicKeyBase64,
          privateKeyHex: wallet.privateKeyHex,
          algorithm: wallet.algorithm,
        },
        PASSWORD,
        "fast",
      ),
    ).rejects.toThrow();
    await expect(
      encryptWalletVault(
        {
          address: wallet.address,
          publicKeyBase64: wallet.publicKeyBase64,
          privateKeyHex: wallet.privateKeyHex.slice(0, 127),
          algorithm: wallet.algorithm,
        },
        PASSWORD,
        "fast",
      ),
    ).rejects.toThrow();
  });

  it("galat password par vault nahi khulta", async () => {
    const wallet = await generateWallet();
    const encrypted = await encryptWalletVault(
      {
        address: wallet.address,
        publicKeyBase64: wallet.publicKeyBase64,
        privateKeyHex: wallet.privateKeyHex,
        algorithm: wallet.algorithm,
      },
      PASSWORD,
      "fast",
    );
    await expect(
      decryptWalletVault(encrypted, "wrong-password-123"),
    ).rejects.toThrow();
  });

  it("badla hua (tampered) vault nahi khulta", async () => {
    const wallet = await generateWallet();
    const encrypted = await encryptWalletVault(
      {
        address: wallet.address,
        publicKeyBase64: wallet.publicKeyBase64,
        privateKeyHex: wallet.privateKeyHex,
        algorithm: wallet.algorithm,
      },
      PASSWORD,
      "fast",
    );
    const parsed = JSON.parse(encrypted) as { data: string };
    const last = parsed.data.slice(-1);
    parsed.data = parsed.data.slice(0, -1) + (last === "A" ? "B" : "A");
    await expect(
      decryptWalletVault(JSON.stringify(parsed), PASSWORD),
    ).rejects.toThrow();
  });

  it("kamzor password (12 se kam akshar) reject hota hai", async () => {
    const wallet = await generateWallet();
    await expect(
      encryptWalletVault(
        {
          address: wallet.address,
          publicKeyBase64: wallet.publicKeyBase64,
          privateKeyHex: wallet.privateKeyHex,
          algorithm: wallet.algorithm,
        },
        "short",
        "fast",
      ),
    ).rejects.toThrow();
  });

  it("slow mode fast mode se zyada iterations use karta hai aur phir bhi khulta hai", async () => {
    // Test me target chhota rakhte hain (300ms) taaki test tez chale.
    // Asli slow mode me target 2 minute hota hai.
    const wallet = await generateWallet();
    const encrypted = await encryptWalletVault(
      {
        address: wallet.address,
        publicKeyBase64: wallet.publicKeyBase64,
        privateKeyHex: wallet.privateKeyHex,
        algorithm: wallet.algorithm,
      },
      PASSWORD,
      "slow",
      300,
    );
    const parsed = JSON.parse(encrypted) as { iterations: number; mode: string };
    expect(parsed.mode).toBe("slow");
    expect(parsed.iterations).toBeGreaterThan(210_000);
    const back = await decryptWalletVault(encrypted, PASSWORD);
    expect(back.address).toBe(wallet.address);
  });

  it("deriveWalletKey test mode fast hota hai aur slow mode time-targeted hota hai", async () => {
    const salt = new Uint8Array(16).fill(7);
    const fast = await deriveWalletKey(PASSWORD, salt, "fast");
    expect(fast.iterations).toBe(210_000);
    const slow = await deriveWalletKey(PASSWORD, salt, "slow", 300);
    expect(slow.iterations).toBeGreaterThan(210_000);
    expect(slow.elapsedMs).toBeGreaterThanOrEqual(150);
  });
});
