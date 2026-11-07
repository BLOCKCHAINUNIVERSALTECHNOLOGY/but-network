import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  fetchBalance,
  fetchHistory,
  lookupButPay,
  mine,
  nodeInfo,
  registerButPay,
  setNodeUrlForTests,
  submitTx,
} from "../shared/but-node";

const ALICE = "but1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const BOB = "but1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/** Canned node — asli chain ki jagah fixed JSON deta hai (client test ke liye). */
let server: Server;
let baseUrl = "";

beforeAll(async () => {
  server = createServer((req, res) => {
    const send = (status: number, obj: unknown) => {
      res.writeHead(status, { "Content-Type": "application/json" });
      res.end(JSON.stringify(obj));
    };
    const url = req.url ?? "/";
    if (url === "/") {
      send(200, { ok: true, height: 3, latest_hash: "abc123" });
    } else if (url === `/balance/${ALICE}`) {
      send(200, { ok: true, address: ALICE, balance: 999000, nonce: 2 });
    } else if (url === "/chain") {
      send(200, {
        ok: true,
        chain: {
          blocks: [
            { index: 0, timestamp: 1700000000, previous_hash: "0", hash: "h0", validator: ALICE, transactions: [] },
            {
              index: 1,
              timestamp: 1700000060,
              previous_hash: "h0",
              hash: "h1",
              validator: ALICE,
              transactions: [
                { sender: ALICE, receiver: BOB, amount: 100, fee: 1, nonce: 0, sender_public_key_hex: "00", signature_hex: "00" },
              ],
            },
            {
              index: 2,
              timestamp: 1700000120,
              previous_hash: "h1",
              hash: "h2",
              validator: ALICE,
              transactions: [
                { sender: BOB, receiver: "100001", amount: 10, fee: 0, nonce: 0, sender_public_key_hex: "00", signature_hex: "00" },
              ],
            },
          ],
        },
      });
    } else if (url === "/butpay/100001") {
      send(200, { ok: true, id: 100001, handle: "bob", address: BOB });
    } else if (url === "/butpay/register" && req.method === "POST") {
      send(200, { ok: true, id: 100002, handle: "alice", address: ALICE });
    } else if (url === "/tx" && req.method === "POST") {
      send(200, { ok: true, tx_id: "deadbeef" });
    } else if (url === "/mine" && req.method === "POST") {
      send(200, { ok: true, height: 4, hash: "h3", tx_count: 1 });
    } else if (url === "/butpay/999999") {
      send(404, { ok: false, error: "BUT Pay ID/handle '999999' nahi mila" });
    } else {
      send(404, { ok: false, error: "unknown route" });
    }
  });
  await new Promise<void>((resolve) => {
    server.listen(0, "127.0.0.1", () => {
      const port = (server.address() as AddressInfo).port;
      baseUrl = `http://127.0.0.1:${port}`;
      setNodeUrlForTests(baseUrl);
      resolve();
    });
  });
});

afterAll(async () => {
  setNodeUrlForTests(null);
  await new Promise<void>((resolve) => {
    server.close(() => resolve());
  });
});

describe("but-node — API client (canned local node)", () => {
  it("nodeInfo height deta hai", async () => {
    const info = await nodeInfo();
    expect(info.height).toBe(3);
    expect(info.latestHash).toBe("abc123");
  });

  it("fetchBalance chain se balance/nonce deta hai", async () => {
    const b = await fetchBalance(ALICE);
    expect(b.balance).toBe(999000);
    expect(b.nonce).toBe(2);
  });

  it("fetchHistory chain padh kar sent/received nikalta hai", async () => {
    const history = await fetchHistory(ALICE);
    expect(history).toHaveLength(1);
    expect(history[0].direction).toBe("sent");
    expect(history[0].amount).toBe(100);
    expect(history[0].otherParty).toBe(BOB);
    // Bob ki history: ek received (Alice se) + ek sent (ID 100001 ko).
    const bobHistory = await fetchHistory(BOB, 100001, "bob");
    expect(bobHistory).toHaveLength(2);
    expect(bobHistory[0].direction).toBe("sent");
    expect(bobHistory[1].direction).toBe("received");
  });

  it("lookupButPay ID se address deta hai, galat ID par error", async () => {
    const entry = await lookupButPay("100001");
    expect(entry.address).toBe(BOB);
    expect(entry.handle).toBe("bob");
    await expect(lookupButPay("999999")).rejects.toThrow("nahi mila");
  });

  it("registerButPay naya ID deta hai", async () => {
    const entry = await registerButPay(ALICE, "alice");
    expect(entry.id).toBe(100002);
    expect(entry.handle).toBe("alice");
  });

  it("submitTx aur mine chalte hain", async () => {
    const res = await submitTx({
      sender: ALICE,
      receiver: BOB,
      amount: 10,
      fee: 1,
      nonce: 2,
      sender_public_key_hex: "00".repeat(32),
      signature_hex: "00".repeat(64),
    });
    expect(res.txId).toBe("deadbeef");
    const mined = await mine();
    expect(mined.height).toBe(4);
  });

  it("node band ho to Hindi error milta hai", async () => {
    setNodeUrlForTests("http://127.0.0.1:1");
    await expect(nodeInfo()).rejects.toThrow("Node se jud nahi paya");
    setNodeUrlForTests(baseUrl);
  });
});
