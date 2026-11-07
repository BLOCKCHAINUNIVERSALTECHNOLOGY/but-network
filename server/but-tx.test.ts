import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  BITS_PER_BUT,
  bitsToBut,
  butToBits,
  formatBits,
  signTransaction,
  txId,
  txPayload,
  verifyTransactionSignature,
  type SignedTx,
} from "../shared/but-tx";
import { walletFromMnemonic } from "../shared/wallet-secure";

/** BIP39 test vector: "abandon" x23 + "art". */
const VECTOR_MNEMONIC = `${"abandon ".repeat(23)}art`;

describe("but-tx — units (1 BUT = 1,000 bits)", () => {
  it("BITS_PER_BUT 1000 hai (Sanjay ka final faisla)", () => {
    expect(BITS_PER_BUT).toBe(1000);
  });

  it("butToBits sahi convert karta hai", () => {
    expect(butToBits("1")).toEqual({ ok: true, bits: 1000 });
    expect(butToBits("0.001")).toEqual({ ok: true, bits: 1 });
    expect(butToBits("12.5")).toEqual({ ok: true, bits: 12500 });
    expect(butToBits(" 2.5 ")).toEqual({ ok: true, bits: 2500 });
    expect(butToBits("0.000")).toEqual({
      ok: false,
      error: expect.any(String),
    });
  });

  it("butToBits galat input reject karta hai", () => {
    expect(butToBits("")).toEqual({ ok: false, error: expect.any(String) });
    expect(butToBits("abc")).toEqual({
      ok: false,
      error: expect.any(String),
    });
    expect(butToBits("1.2345")).toEqual({
      ok: false,
      error: expect.any(String),
    });
    expect(butToBits("-5")).toEqual({
      ok: false,
      error: expect.any(String),
    });
    expect(butToBits("0")).toEqual({
      ok: false,
      error: expect.any(String),
    });
  });

  it("bitsToBut aur formatBits sahi dikhate hain", () => {
    expect(bitsToBut(1000)).toBe("1");
    expect(bitsToBut(1500)).toBe("1.5");
    expect(bitsToBut(1)).toBe("0.001");
    expect(bitsToBut(0)).toBe("0");
    expect(formatBits(1234500)).toBe("1,234.5 BUT");
    expect(formatBits(1000)).toBe("1 BUT");
  });
});

describe("but-tx — signing (Rust core ke saath same format)", () => {
  it("tx payload aur ID Rust jaisi banti hai", async () => {
    const tx = {
      sender: "but1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      receiver: "but1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      amount: 100,
      fee: 1,
      nonce: 0,
    };
    expect(txPayload(tx)).toBe(
      "BUT-TX|but1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|but1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb|100|1|0",
    );
    // Independent check: Node ke crypto se SHA-256.
    const expected = createHash("sha256")
      .update(txPayload(tx), "utf8")
      .digest("hex");
    expect(await txId(tx)).toBe(expected);
  });

  it("signTransaction: signature verify hoti hai, tamper pakda jata hai", async () => {
    const wallet = await walletFromMnemonic(VECTOR_MNEMONIC);
    const { generateWallet } = await import("../shared/wallet-secure");
    const other = await generateWallet();
    const tx = {
      sender: wallet.address,
      receiver: other.address,
      amount: 100,
      fee: 1,
      nonce: 0,
    };
    const signed: SignedTx = await signTransaction(tx, wallet.privateKeyHex);
    expect(signed.sender_public_key_hex).toHaveLength(64);
    expect(signed.signature_hex).toHaveLength(128);
    expect(await verifyTransactionSignature(signed)).toBe(true);
    // Amount badla to signature fail.
    expect(
      await verifyTransactionSignature({ ...signed, amount: 999 }),
    ).toBe(false);
    // Receiver badla to signature fail.
    expect(
      await verifyTransactionSignature({
        ...signed,
        receiver: wallet.address,
      }),
    ).toBe(false);
  });

  it("BUT Pay ID receiver par bhi sign hota hai (signature ID wali string par)", async () => {
    const wallet = await walletFromMnemonic(VECTOR_MNEMONIC);
    const tx = {
      sender: wallet.address,
      receiver: "100001",
      amount: 50,
      fee: 1,
      nonce: 0,
    };
    const signed = await signTransaction(tx, wallet.privateKeyHex);
    expect(await verifyTransactionSignature(signed)).toBe(true);
    expect(signed.receiver).toBe("100001");
  });

  it("galat private key se sign nahi hota, dusri key ki signature sender se match nahi karti", async () => {
    const wallet = await walletFromMnemonic(VECTOR_MNEMONIC);
    const { generateWallet } = await import("../shared/wallet-secure");
    const { sha256Hex } = await import("../shared/crypto-secure");
    const other = await generateWallet();
    const tx = {
      sender: wallet.address,
      receiver: other.address,
      amount: 10,
      fee: 0,
      nonce: 0,
    };
    // Galat length wali key reject hoti hai.
    await expect(
      signTransaction(tx, wallet.privateKeyHex.slice(0, 127)),
    ).rejects.toThrow();
    // Dusre wallet ki sahi-format key se sign hoga, par uski public key
    // se bana address sender se match nahi karega — node ise reject karega.
    const wrongSigned = await signTransaction(tx, other.privateKeyHex);
    const pubkeyBytes = new Uint8Array(
      wrongSigned.sender_public_key_hex
        .match(/../g)!
        .map((h) => Number.parseInt(h, 16)),
    );
    const derivedAddress = `but1${(await sha256Hex(pubkeyBytes)).slice(0, 52)}`;
    expect(derivedAddress).not.toBe(tx.sender);
    expect(derivedAddress).toBe(other.address);
  });
});
