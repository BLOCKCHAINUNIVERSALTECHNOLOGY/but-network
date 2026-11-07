import jsQR from "jsqr";
import QRCode from "qrcode";
import { describe, expect, it } from "vitest";
import {
  checkSend,
  parseReceiverInput,
} from "../shared/send-checks";

const OWN = "but1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER = "but1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

describe("send-checks — receiver pehchanna", () => {
  it("address, ID aur handle sahi pehchanta hai", () => {
    expect(parseReceiverInput(OWN)).toEqual({ kind: "address", value: OWN });
    expect(parseReceiverInput("100001")).toEqual({ kind: "id", value: "100001" });
    expect(parseReceiverInput("@sanjay")).toEqual({
      kind: "handle",
      value: "@sanjay",
    });
    expect(parseReceiverInput("sanjay")).toEqual({
      kind: "handle",
      value: "@sanjay",
    });
  });

  it("galat receiver invalid hota hai (Hindi error ke saath)", () => {
    expect(parseReceiverInput("").kind).toBe("invalid");
    expect(parseReceiverInput("but1abc").kind).toBe("invalid");
    expect(parseReceiverInput("hello world").kind).toBe("invalid");
    const bad = parseReceiverInput("but1abc");
    expect(bad.kind).toBe("invalid");
    if (bad.kind === "invalid") {
      expect(bad.error.length).toBeGreaterThan(5);
    }
  });
});

describe("send-checks — scam warnings", () => {
  it("khud ko bhejna aur balance se zyada block hota hai", () => {
    const self = checkSend({
      resolvedAddress: OWN,
      ownAddress: OWN,
      amountBits: 10,
      feeBits: 1,
      balanceBits: 1000,
      isKnownReceiver: true,
    });
    expect(self.some((w) => w.level === "block")).toBe(true);

    const tooMuch = checkSend({
      resolvedAddress: OTHER,
      ownAddress: OWN,
      amountBits: 1000,
      feeBits: 1,
      balanceBits: 1000,
      isKnownReceiver: true,
    });
    expect(tooMuch.some((w) => w.level === "block")).toBe(true);
  });

  it("pehli baar bade amount par warn karta hai, jaane-pehchane par sirf info", () => {
    const firstTime = checkSend({
      resolvedAddress: OTHER,
      ownAddress: OWN,
      amountBits: 600,
      feeBits: 1,
      balanceBits: 1000,
      isKnownReceiver: false,
    });
    expect(firstTime.filter((w) => w.level === "warn").length).toBe(2);
    expect(firstTime.some((w) => w.level === "info")).toBe(true);

    const known = checkSend({
      resolvedAddress: OTHER,
      ownAddress: OWN,
      amountBits: 10,
      feeBits: 1,
      balanceBits: 1000,
      isKnownReceiver: true,
    });
    expect(known.filter((w) => w.level === "warn").length).toBe(0);
    expect(known.some((w) => w.level === "info")).toBe(true);
  });

  it("receiver na mile to block", () => {
    const noReceiver = checkSend({
      resolvedAddress: null,
      ownAddress: OWN,
      amountBits: 10,
      feeBits: 1,
      balanceBits: 1000,
      isKnownReceiver: false,
    });
    expect(noReceiver[0].level).toBe("block");
  });
});

describe("QR — qrcode se bana QR jsQR se padha jata hai (scan path)", () => {
  it("address wala QR encode + decode round-trip karta hai", () => {
    const text = OWN;
    const qr = QRCode.create(text, { errorCorrectionLevel: "M" });
    const scale = 8;
    const border = 4;
    const side = (qr.modules.size + border * 2) * scale;
    const rgba = new Uint8ClampedArray(side * side * 4);
    for (let y = 0; y < side; y++) {
      for (let x = 0; x < side; x++) {
        const mx = Math.floor(x / scale) - border;
        const my = Math.floor(y / scale) - border;
        const dark =
          mx >= 0 &&
          my >= 0 &&
          mx < qr.modules.size &&
          my < qr.modules.size &&
          qr.modules.data[my * qr.modules.size + mx] === 1;
        const v = dark ? 0 : 255;
        const i = (y * side + x) * 4;
        rgba[i] = v;
        rgba[i + 1] = v;
        rgba[i + 2] = v;
        rgba[i + 3] = 255;
      }
    }
    const decoded = jsQR(rgba, side, side);
    expect(decoded).not.toBeNull();
    expect(decoded!.data).toBe(text);
  });
});
