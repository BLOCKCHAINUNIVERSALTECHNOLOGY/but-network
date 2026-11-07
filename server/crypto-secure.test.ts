import { describe, expect, it } from "vitest";
import {
  decryptFromLocalStorage,
  encryptForLocalStorage,
  generateDemoKeypair,
  isSecureCryptoAvailable,
  secureRandomBytes,
  secureRandomHex,
  secureRandomId,
  sha256Hex,
} from "../shared/crypto-secure";

describe("crypto-secure (demo module)", () => {
  it("secure crypto is environment me available hai", () => {
    expect(isSecureCryptoAvailable()).toBe(true);
  });

  it("secureRandomBytes sahi length deta hai aur har baar alag hota hai", () => {
    const a = secureRandomBytes(32);
    const b = secureRandomBytes(32);
    expect(a).toBeInstanceOf(Uint8Array);
    expect(a.length).toBe(32);
    expect(Buffer.from(a).equals(Buffer.from(b))).toBe(false);
  });

  it("secureRandomBytes galat length par error deta hai", () => {
    expect(() => secureRandomBytes(0)).toThrow();
    expect(() => secureRandomBytes(-1)).toThrow();
  });

  it("secureRandomHex 32 bytes ka 64-char hex deta hai aur unique hota hai", () => {
    const a = secureRandomHex(32);
    const b = secureRandomHex(32);
    expect(a).toMatch(/^[0-9a-f]{64}$/);
    expect(a).not.toBe(b);
  });

  it("secureRandomId demo ID deta hai", () => {
    const id = secureRandomId(8);
    expect(id).toMatch(/^[0-9A-F]{8}$/);
    expect(secureRandomId(8)).not.toBe(id);
  });

  it("sha256Hex sahi hash deta hai (known vector: 'hello')", async () => {
    const hash = await sha256Hex("hello");
    expect(hash).toBe(
      "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    );
  });

  it("encrypt aur decrypt roundtrip karta hai", async () => {
    const payload = await encryptForLocalStorage(
      "demo balance 4280.5",
      "demo-password-123",
    );
    const back = await decryptFromLocalStorage(payload, "demo-password-123");
    expect(back).toBe("demo balance 4280.5");
  });

  it("har encrypt alag hota hai (random salt + IV)", async () => {
    const a = await encryptForLocalStorage("same text", "demo-password-123");
    const b = await encryptForLocalStorage("same text", "demo-password-123");
    expect(a).not.toBe(b);
  });

  it("galat password par decrypt fail hota hai", async () => {
    const payload = await encryptForLocalStorage(
      "secret demo",
      "demo-password-123",
    );
    await expect(
      decryptFromLocalStorage(payload, "wrong-password"),
    ).rejects.toThrow();
  });

  it("badla hua (tampered) data decrypt nahi hota", async () => {
    const payload = await encryptForLocalStorage(
      "secret demo",
      "demo-password-123",
    );
    const parsed = JSON.parse(payload) as { data: string };
    // Aakhri character badlo — tamper karo.
    const last = parsed.data.slice(-1);
    parsed.data = parsed.data.slice(0, -1) + (last === "A" ? "B" : "A");
    await expect(
      decryptFromLocalStorage(JSON.stringify(parsed), "demo-password-123"),
    ).rejects.toThrow();
  });

  it("kamzor password par encrypt fail hota hai", async () => {
    await expect(
      encryptForLocalStorage("x", "short"),
    ).rejects.toThrow();
  });

  it("generateDemoKeypair demo key deta hai", async () => {
    const keypair = await generateDemoKeypair();
    expect(["Ed25519", "ECDSA P-256"]).toContain(keypair.algorithm);
    expect(keypair.publicKeyBase64.length).toBeGreaterThan(10);
    expect(keypair.privateKeyBase64.length).toBeGreaterThan(10);
    expect(keypair.warning).toContain("DEMO");
  });
});
