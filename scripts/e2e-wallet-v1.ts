/**
 * E2E check — web wallet ke modules (shared/*) se live Rust node tak.
 *
 * Ye script wahi raasta chalti hai jo web UI chalti hai:
 *  1. Alice ka wallet TS se banao (mnemonic se) — address Rust jaisa hona chahiye.
 *  2. Naya wallet banao (generateWallet) — seed se hi keypair banta hai.
 *  3. Alice naye wallet ko 5000 bits bheje (TS sign -> POST /tx -> POST /mine).
 *  4. Naya wallet BUT Pay ID register kare.
 *  5. Naya wallet Bob ko ID "100001" se 1000 bits bheje (TS sign -> /tx -> /mine).
 *  6. Balance + history chain se check karo.
 *
 * Chalao: pehle `cargo run -- demo` (data/chain.json banega), phir
 * `cargo run -- serve 3030 data/chain.json`, phir `npx tsx scripts/e2e-wallet-v1.ts`.
 * Sirf local dev ke liye — Alice dev wallet hai (entropy [1u8; 32]).
 */

import {
  fetchBalance,
  fetchHistory,
  lookupButPay,
  mine,
  nodeInfo,
  registerButPay,
  submitTx,
} from "../shared/but-node";
import { signTransaction } from "../shared/but-tx";
import { generateWallet, walletFromMnemonic } from "../shared/wallet-secure";

const ALICE_MNEMONIC =
  "absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice comic";
const ALICE_ADDRESS = "but1071e3bb9bb2e5acc7491f94c7526debeaaef09ee6126312824a0";
const BOB_ADDRESS = "but1af50f1cae2556c6a78024ca7b2113fc9facf59318c78303a1037";

function check(name: string, cond: boolean, detail = ""): void {
  if (!cond) {
    throw new Error(`FAIL: ${name} ${detail}`);
  }
  console.log(`PASS: ${name}${detail ? ` — ${detail}` : ""}`);
}

async function main(): Promise<void> {
  // 0. Node zinda hai?
  const info = await nodeInfo();
  console.log(`Node height: ${info.height}`);

  // 1. Alice ka wallet TS se — Rust jaisa address aana chahiye.
  const alice = await walletFromMnemonic(ALICE_MNEMONIC);
  check("Alice address Rust jaisa hai", alice.address === ALICE_ADDRESS, alice.address);

  // 2. Naya wallet — mnemonic se hi keypair (deterministic).
  const fresh = await generateWallet();
  const freshAgain = await walletFromMnemonic(fresh.mnemonic);
  check("Naya wallet deterministic hai", freshAgain.address === fresh.address, fresh.address);
  const freshStart = await fetchBalance(fresh.address);
  check("Naye wallet ka balance 0 hai", freshStart.balance === 0, String(freshStart.balance));

  // 3. Alice -> naya wallet 5000 bits.
  const aliceBalance = await fetchBalance(ALICE_ADDRESS);
  const tx1 = await signTransaction(
    { sender: ALICE_ADDRESS, receiver: fresh.address, amount: 5000, fee: 1, nonce: aliceBalance.nonce },
    alice.privateKeyHex,
  );
  const sent1 = await submitTx(tx1);
  check("Alice -> naya wallet tx submit hui", sent1.txId.length === 64);
  const mined1 = await mine();
  check("Mine hua", mined1.height > info.height, `height ${mined1.height}`);
  const freshAfter = await fetchBalance(fresh.address);
  check("Naye wallet ko 5000 bits mile", freshAfter.balance === 5000, String(freshAfter.balance));

  // 4. Naye wallet ki BUT Pay ID register karo.
  const entry = await registerButPay(fresh.address, "testuser");
  check("BUT Pay ID mili", entry.id >= 100002, `ID ${entry.id}`);
  const looked = await lookupButPay(String(entry.id));
  check("ID se address milta hai", looked.address === fresh.address);
  const lookedHandle = await lookupButPay("@testuser");
  check("Handle se address milta hai", lookedHandle.address === fresh.address);

  // 5. Naya wallet -> Bob ko ID "100001" se 1000 bits.
  const bobBefore = await fetchBalance(BOB_ADDRESS);
  const freshBalance = await fetchBalance(fresh.address);
  const tx2 = await signTransaction(
    { sender: fresh.address, receiver: "100001", amount: 1000, fee: 1, nonce: freshBalance.nonce },
    fresh.privateKeyHex,
  );
  await submitTx(tx2);
  await mine();
  const bobAfter = await fetchBalance(BOB_ADDRESS);
  check("Bob ko ID se 1000 bits mile", bobAfter.balance === bobBefore.balance + 1000, `${bobBefore.balance} -> ${bobAfter.balance}`);
  const freshEnd = await fetchBalance(fresh.address);
  check("Naye wallet ka balance 3999 hai", freshEnd.balance === 3999, String(freshEnd.balance));

  // 6. History chain se.
  const history = await fetchHistory(fresh.address, entry.id, "testuser");
  check("History me 2 entries hain", history.length === 2, history.map((h) => h.direction).join(","));
  check("Pehli entry sent hai", history[0].direction === "sent" && history[0].amount === 1000);
  check("Dusri entry received hai", history[1].direction === "received" && history[1].amount === 5000);

  // 7. Galat ID lookup error deta hai.
  try {
    await lookupButPay("999999");
    throw new Error("galat ID par error aana chahiye tha");
  } catch (err) {
    check("Galat ID par error milta hai", err instanceof Error && err.message.includes("nahi mila"));
  }

  console.log("\nE2E WALLET v1: SAB PASS");
}

main().catch((err) => {
  console.error(`\nE2E FAIL: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});
