/**
 * shared/but-node.ts — BUT node (Rust API, port 3030) se baat karna
 *
 * Web wallet seedha local node se judta hai — balance, history, BUT Pay
 * lookup aur transaction submit, sab chain se aata hai. Koi fake data
 * yahan nahi banta.
 *
 * Node URL: `VITE_BUT_NODE_URL` env se, default `http://127.0.0.1:3030`.
 * Node band ho to har call Hindi error dega — UI use dikhayega.
 */

import type { SignedTx } from "./but-tx";

/** Sirf tests ke liye — test server ka URL set karo (null = default). */
let nodeUrlOverride: string | null = null;

export function setNodeUrlForTests(url: string | null): void {
  nodeUrlOverride = url;
}

export function getNodeUrl(): string {
  if (nodeUrlOverride) return nodeUrlOverride.replace(/\/$/, "");
  try {
    const envUrl = (import.meta as unknown as { env?: Record<string, string | undefined> })
      .env?.VITE_BUT_NODE_URL;
    if (envUrl) return envUrl.replace(/\/$/, "");
  } catch {
    // ignore — default URL use hoga
  }
  return "http://127.0.0.1:3030";
}

export class NodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "NodeError";
  }
}

async function fetchJson(
  path: string,
  init?: RequestInit,
  timeoutMs = 8000,
): Promise<unknown> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(`${getNodeUrl()}${path}`, {
      ...init,
      signal: controller.signal,
      headers: { "Content-Type": "application/json", ...(init?.headers ?? {}) },
    });
    const data = (await res.json().catch(() => null)) as {
      ok?: boolean;
      error?: string;
    } | null;
    if (!res.ok || data === null) {
      throw new NodeError(
        data?.error ?? `Node ne error diya (HTTP ${res.status}).`,
      );
    }
    if (data.ok === false) {
      throw new NodeError(data.error ?? "Node ne request reject kar di.");
    }
    return data;
  } catch (err) {
    if (err instanceof NodeError) throw err;
    if (err instanceof DOMException && err.name === "AbortError") {
      throw new NodeError(
        "Node se jawab nahi mila (timeout). Kya but-node serve chal raha hai?",
      );
    }
    throw new NodeError(
      "Node se jud nahi paya. Pehle terminal me `cargo run -- serve` chalao, phir dobara try karo.",
    );
  } finally {
    clearTimeout(timer);
  }
}

export type NodeInfo = {
  height: number;
  latestHash: string;
};

/** Node zinda hai? Height aur latest hash lo. */
export async function nodeInfo(): Promise<NodeInfo> {
  const data = (await fetchJson("/")) as {
    height: number;
    latest_hash: string;
  };
  return { height: data.height, latestHash: data.latest_hash };
}

export type BalanceInfo = {
  address: string;
  balance: number;
  nonce: number;
};

/** Balance aur nonce — hamesha chain state se aata hai. */
export async function fetchBalance(address: string): Promise<BalanceInfo> {
  const data = (await fetchJson(`/balance/${address}`)) as {
    address: string;
    balance: number;
    nonce: number;
  };
  return {
    address: data.address,
    balance: data.balance,
    nonce: data.nonce,
  };
}

export type ChainTx = {
  sender: string;
  receiver: string;
  amount: number;
  fee: number;
  nonce: number;
  sender_public_key_hex: string;
  signature_hex: string;
};

export type ChainBlock = {
  index: number;
  timestamp: number;
  previous_hash: string;
  hash: string;
  validator: string;
  transactions: ChainTx[];
};

export async function fetchChain(): Promise<ChainBlock[]> {
  const data = (await fetchJson("/chain")) as {
    chain: { blocks: ChainBlock[] };
  };
  return data.chain.blocks;
}

export type HistoryEntry = {
  /** Block index + tx index se bana stable key. */
  key: string;
  direction: "sent" | "received";
  amount: number;
  fee: number;
  otherParty: string;
  blockIndex: number;
  timestamp: number;
  status: "confirmed";
};

/**
 * Is address ki history — poori chain padh kar nikalo. Receiver BUT Pay
 * ID/handle bhi ho sakta hai (block me jaisa likha tha waisa hi milta
 * hai), isliye apni ID/handle bhi match karo.
 */
export async function fetchHistory(
  address: string,
  butPayId?: number | null,
  handle?: string | null,
): Promise<HistoryEntry[]> {
  const blocks = await fetchChain();
  const mine = new Set<string>([address]);
  if (butPayId !== undefined && butPayId !== null) mine.add(String(butPayId));
  if (handle) {
    mine.add(handle.startsWith("@") ? handle : `@${handle}`);
    mine.add(handle.replace(/^@/, ""));
  }
  const entries: HistoryEntry[] = [];
  for (const block of blocks) {
    for (let i = 0; i < block.transactions.length; i++) {
      const tx = block.transactions[i];
      const sent = tx.sender === address;
      const received = mine.has(tx.receiver) && !sent;
      if (!sent && !received) continue;
      entries.push({
        key: `${block.index}-${i}`,
        direction: sent ? "sent" : "received",
        amount: tx.amount,
        fee: tx.fee,
        otherParty: sent ? tx.receiver : tx.sender,
        blockIndex: block.index,
        timestamp: block.timestamp,
        status: "confirmed",
      });
    }
  }
  // Nayi pehle.
  entries.sort((a, b) => b.blockIndex - a.blockIndex);
  return entries;
}

export type ButPayEntry = {
  id: number;
  handle: string | null;
  address: string;
};

/** BUT Pay ID/handle se address nikalo (resolve). Na mile to error. */
export async function lookupButPay(query: string): Promise<ButPayEntry> {
  const q = encodeURIComponent(query.trim());
  const data = (await fetchJson(`/butpay/${q}`)) as {
    id: number;
    handle: string | null;
    address: string;
  };
  return { id: data.id, handle: data.handle, address: data.address };
}

/** Naya BUT Pay ID lo — address register karo, optional handle ke saath. */
export async function registerButPay(
  address: string,
  handle?: string,
): Promise<ButPayEntry> {
  const data = (await fetchJson("/butpay/register", {
    method: "POST",
    body: JSON.stringify(
      handle ? { address, handle } : { address },
    ),
  })) as { id: number; handle: string | null; address: string };
  return { id: data.id, handle: data.handle, address: data.address };
}

/** Signed transaction node ko bhejo — mempool me jayegi. */
export async function submitTx(tx: SignedTx): Promise<{ txId: string }> {
  const data = (await fetchJson("/tx", {
    method: "POST",
    body: JSON.stringify(tx),
  })) as { tx_id: string };
  return { txId: data.tx_id };
}

/**
 * Mempool se block mine karo (dev node — fixed dev validator se sign
 * hota hai). Production me validator network khud mine karega; web v1
 * me yehi tareeka hai transaction confirm karne ka.
 */
export async function mine(): Promise<{ height: number; hash: string }> {
  const data = (await fetchJson("/mine", {
    method: "POST",
    body: "",
  })) as { height: number; hash: string };
  return { height: data.height, hash: data.hash };
}
