import { describe, expect, it } from "vitest";
import {
  DEMO_WORDLIST,
  decryptWalletVault,
  deriveWalletKey,
  encryptWalletVault,
  generateMnemonic,
  generateWallet,
  mnemonicToSeed,
  validateMnemonic,
} from "../shared/wallet-secure";

const PASSWORD = "demo-password-123";

describe("wallet-secure (demo wallet module)", () => {
  it("wordlist me 256 unique words hain", () => {
    expect(DEMO_WORDLIST.length).toBe(256);
    expect(new Set(DEMO_WORDLIST).size).toBe(256);
  });

  it("generateWallet but1 address deta hai aur har baar alag hota hai", async () => {
    const a = await generateWallet();
    const b = await generateWallet();
    expect(a.address).toMatch(/^but1[0-9a-f]{40}$/);
    expect(a.address).not.toBe(b.address);
    expect(a.publicKeyBase64).not.toBe(b.publicKeyBase64);
    expect(a.privateKeyBase64).not.toBe(b.privateKeyBase64);
    expect(a.mnemonic).not.toBe(b.mnemonic);
    expect(a.warning).toContain("DEMO");
  });

  it("mnemonic 33 words ka hota hai aur checksum sahi hota hai", async () => {
    const mnemonic = await generateMnemonic(32);
    const words = mnemonic.split(" ");
    expect(words.length).toBe(33);
    for (const word of words) {
      expect(DEMO_WORDLIST).toContain(word);
    }
    expect(await validateMnemonic(mnemonic)).toBe(true);
  });

  it("galat mnemonic reject hota hai (galat word, galat length, galat checksum)", async () => {
    const mnemonic = await generateMnemonic(32);
    const words = mnemonic.split(" ");
    // Galat word
    const badWord = words.slice();
    badWord[0] = badWord[0] === "abandon" ? "ability" : "abandon";
    expect(await validateMnemonic(badWord.join(" "))).toBe(false);
    // Galat length
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
    const mnemonic = await generateMnemonic(32);
    const seed1 = await mnemonicToSeed(mnemonic);
    const seed2 = await mnemonicToSeed(mnemonic);
    const seed3 = await mnemonicToSeed(mnemonic, "extra-word");
    expect(seed1.length).toBe(64);
    expect(Buffer.from(seed1).equals(Buffer.from(seed2))).toBe(true);
    expect(Buffer.from(seed1).equals(Buffer.from(seed3))).toBe(false);
  });

  it("vault encrypt/decrypt round-trip karta hai (fast mode)", async () => {
    const wallet = await generateWallet();
    const vault = {
      address: wallet.address,
      publicKeyBase64: wallet.publicKeyBase64,
      privateKeyBase64: wallet.privateKeyBase64,
      algorithm: wallet.algorithm,
    };
    const encrypted = await encryptWalletVault(vault, PASSWORD, "fast");
    const back = await decryptWalletVault(encrypted, PASSWORD);
    expect(back.address).toBe(vault.address);
    expect(back.privateKeyBase64).toBe(vault.privateKeyBase64);
    expect(back.publicKeyBase64).toBe(vault.publicKeyBase64);
  });

  it("galat password par vault nahi khulta", async () => {
    const wallet = await generateWallet();
    const encrypted = await encryptWalletVault(
      {
        address: wallet.address,
        publicKeyBase64: wallet.publicKeyBase64,
        privateKeyBase64: wallet.privateKeyBase64,
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
        privateKeyBase64: wallet.privateKeyBase64,
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
          privateKeyBase64: wallet.privateKeyBase64,
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
        privateKeyBase64: wallet.privateKeyBase64,
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
