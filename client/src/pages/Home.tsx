/**
 * BUT Wallet v1 — real wallet (Rust chain API, port 3030, se juda hua)
 *
 * V1 scope (Sanjay ka FINAL faisla, docs/WALLET-V1-SCOPE.md):
 *  1. Seed backup + verify (24 words, real BIP39)
 *  2. Password unlock + biometric (WebAuthn PRF, jahan support ho) + auto-lock
 *  3. Bhejne se pehle saaf preview (kis ko, kitna, fee)
 *  4. BUT Pay Send/Receive + QR (Receive par QR, Send par QR scan ya ID/handle)
 *  5. Scam warning (pehli baar, badi rakam, galat address)
 *
 * Balance aur history hamesha chain se aate hain — koi fake data nahi.
 * Private key sirf unlock ke waqt RAM me rehti hai; vault AES-GCM se
 * encrypt hokar localStorage me rehta hai. Seed phrase kabhi store nahi hoti.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import QRCode from "qrcode";
import jsQR from "jsqr";
import {
  ArrowDownLeft,
  ArrowUpRight,
  Check,
  ChevronRight,
  Copy,
  Eye,
  EyeOff,
  Fingerprint,
  History,
  LayoutDashboard,
  LockKeyhole,
  QrCode,
  ScanLine,
  ShieldAlert,
  ShieldCheck,
  Wallet,
  X,
} from "lucide-react";
import {
  DEFAULT_FEE_BITS,
  bitsToBut,
  butToBits,
  formatBits,
  signTransaction,
} from "@shared/but-tx";
import {
  NodeError,
  fetchBalance,
  fetchHistory,
  lookupButPay,
  mine,
  nodeInfo,
  registerButPay,
  submitTx,
  type HistoryEntry,
} from "@shared/but-node";
import {
  addKnownReceiver,
  checkSend,
  getKnownReceivers,
  parseReceiverInput,
  type SendWarning,
} from "@shared/send-checks";
import {
  decryptWalletVault,
  encryptWalletVault,
  generateWallet,
  validateMnemonic,
  walletFromMnemonic,
  type WalletVault,
} from "@shared/wallet-secure";
import {
  clearBiometricUnlock,
  isBiometricSetup,
  isBiometricUnlockAvailable,
  isWebAuthnAvailable,
  setupBiometricUnlock,
  unlockWithBiometric,
} from "@shared/webauthn";

// ---------------------------------------------------------------------------
// Storage keys (v1) — purane demo keys yahan nahi aate
// ---------------------------------------------------------------------------

const VAULT_KEY = "but-wallet-vault-v1";
const META_KEY = "but-wallet-meta-v1";
const LEGACY_DEMO_KEYS = [
  "but-demo-balance-v1",
  "but-demo-transactions-v1",
  "but-demo-mode-v1",
  "but-demo-ledger-enc-v1",
];

/** Auto-lock: 5 minute koi activity na ho to wallet lock ho jayega. */
const AUTO_LOCK_MS = 5 * 60 * 1000;

const RESERVED_HANDLES = new Set([
  "but",
  "butpay",
  "butcoin",
  "admin",
  "support",
  "official",
  "team",
  "help",
  "system",
  "root",
  "mod",
  "security",
]);

type WalletMeta = {
  address: string;
  butpayId: number | null;
  handle: string | null;
};

type Screen =
  | "boot"
  | "onboarding"
  | "create-seed"
  | "create-verify"
  | "create-password"
  | "import-seed"
  | "import-password"
  | "locked"
  | "wallet";

type Tab = "wallet" | "history";

function readMeta(): WalletMeta | null {
  try {
    const raw = window.localStorage.getItem(META_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as WalletMeta;
    if (typeof parsed.address !== "string") return null;
    return parsed;
  } catch {
    return null;
  }
}

function saveMeta(meta: WalletMeta): void {
  try {
    window.localStorage.setItem(META_KEY, JSON.stringify(meta));
  } catch {
    // Ignore — meta optional hai.
  }
}

function shortAddress(address: string): string {
  if (address.length < 12) return address;
  return `${address.slice(0, 8)}…${address.slice(-6)}`;
}

function formatTime(timestampSecs: number): string {
  return new Intl.DateTimeFormat("hi-IN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestampSecs * 1000));
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

export default function Home() {
  const [screen, setScreen] = useState<Screen>("boot");
  const [tab, setTab] = useState<Tab>("wallet");
  const [hasVault, setHasVault] = useState(false);

  // Unlock ke baad RAM me rehne wala wallet (kabhi localStorage me plain nahi).
  const [vault, setVault] = useState<WalletVault | null>(null);
  const [password, setPassword] = useState<string | null>(null);
  const [meta, setMeta] = useState<WalletMeta | null>(null);

  // Chain data (hamesha node se).
  const [balanceBits, setBalanceBits] = useState<number | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [nodeHeight, setNodeHeight] = useState<number | null>(null);
  const [nodeOnline, setNodeOnline] = useState<boolean | null>(null);
  const [chainError, setChainError] = useState("");

  // Onboarding state.
  const [newMnemonic, setNewMnemonic] = useState("");
  const [verifyIndices, setVerifyIndices] = useState<number[]>([]);
  const [verifyAnswers, setVerifyAnswers] = useState<string[]>(["", "", ""]);
  const [verifyError, setVerifyError] = useState("");
  const [seedConfirmed, setSeedConfirmed] = useState(false);
  const [importMnemonic, setImportMnemonic] = useState("");
  const [importError, setImportError] = useState("");
  const [pendingMnemonic, setPendingMnemonic] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [handleInput, setHandleInput] = useState("");
  const [passwordError, setPasswordError] = useState("");
  const [creating, setCreating] = useState(false);
  const [createNote, setCreateNote] = useState("");

  // Lock screen state.
  const [passwordInput, setPasswordInput] = useState("");
  const [unlockError, setUnlockError] = useState("");
  const [unlocking, setUnlocking] = useState(false);
  const [biometricAvailable, setBiometricAvailable] = useState(false);
  const [biometricReady, setBiometricReady] = useState(false);
  const [biometricNote, setBiometricNote] = useState("");

  // Wallet screen state.
  const [balanceVisible, setBalanceVisible] = useState(true);
  const [copied, setCopied] = useState("");
  const [refreshing, setRefreshing] = useState(false);
  const [registeringId, setRegisteringId] = useState(false);
  const [idNote, setIdNote] = useState("");

  // Send / Receive modals.
  const [dialog, setDialog] = useState<"send" | "receive" | "scan" | null>(null);
  const [receiverInput, setReceiverInput] = useState("");
  const [amountInput, setAmountInput] = useState("");
  const [sendError, setSendError] = useState("");
  const [sendStep, setSendStep] = useState<"form" | "preview" | "sending" | "done">("form");
  const [resolvedAddress, setResolvedAddress] = useState("");
  const [resolvedLabel, setResolvedLabel] = useState("");
  const [previewBits, setPreviewBits] = useState(0);
  const [previewWarnings, setPreviewWarnings] = useState<SendWarning[]>([]);
  const [sentTxId, setSentTxId] = useState("");
  const [sentBlockHeight, setSentBlockHeight] = useState(0);
  const [qrDataUrl, setQrDataUrl] = useState("");

  const lockTimerRef = useRef<number | null>(null);

  // ---- Pehli baar: purane demo keys hatao, vault hai ya nahi dekho ----
  useEffect(() => {
    try {
      for (const key of LEGACY_DEMO_KEYS) {
        window.localStorage.removeItem(key);
      }
    } catch {
      // Ignore.
    }
    const stored = window.localStorage.getItem(VAULT_KEY);
    setHasVault(stored !== null);
    setMeta(readMeta());
    setScreen(stored ? "locked" : "onboarding");
    // Biometric capability check (sirf availability — koi prompt nahi).
    isBiometricUnlockAvailable()
      .then(setBiometricAvailable)
      .catch(() => setBiometricAvailable(false));
    setBiometricReady(isBiometricSetup());
  }, []);

  // ---- Chain data refresh (balance + history + height) ----
  const refreshChain = useCallback(async () => {
    if (!vault) return;
    setRefreshing(true);
    try {
      const [info, balance] = await Promise.all([
        nodeInfo(),
        fetchBalance(vault.address),
      ]);
      setNodeOnline(true);
      setNodeHeight(info.height);
      setBalanceBits(balance.balance);
      setChainError("");
      try {
        const h = await fetchHistory(
          vault.address,
          meta?.butpayId ?? null,
          meta?.handle ?? null,
        );
        setHistory(h);
      } catch {
        // History na mile to balance phir bhi dikhega.
      }
    } catch (err) {
      setNodeOnline(false);
      setNodeHeight(null);
      setChainError(
        err instanceof NodeError
          ? err.message
          : "Node se jud nahi paya.",
      );
    } finally {
      setRefreshing(false);
    }
  }, [vault, meta]);

  useEffect(() => {
    if (screen !== "wallet" || !vault) return;
    void refreshChain();
    const timer = window.setInterval(() => void refreshChain(), 15000);
    return () => window.clearInterval(timer);
  }, [screen, vault, refreshChain]);

  // ---- Auto-lock: 5 minute koi activity na ho to lock karo ----
  const lockWallet = useCallback(() => {
    setVault(null);
    setPassword(null);
    setPasswordInput("");
    setUnlockError("");
    setTab("wallet");
    setDialog(null);
    setScreen("locked");
  }, []);

  useEffect(() => {
    if (screen !== "wallet") return;
    const resetTimer = () => {
      if (lockTimerRef.current !== null) {
        window.clearTimeout(lockTimerRef.current);
      }
      lockTimerRef.current = window.setTimeout(lockWallet, AUTO_LOCK_MS);
    };
    resetTimer();
    window.addEventListener("pointerdown", resetTimer);
    window.addEventListener("keydown", resetTimer);
    return () => {
      if (lockTimerRef.current !== null) {
        window.clearTimeout(lockTimerRef.current);
      }
      window.removeEventListener("pointerdown", resetTimer);
      window.removeEventListener("keydown", resetTimer);
    };
  }, [screen, lockWallet]);

  // ---- Wallet banao / import karo ----
  async function startCreate() {
    const wallet = await generateWallet();
    setNewMnemonic(wallet.mnemonic);
    const indices = new Set<number>();
    while (indices.size < 3) {
      const bytes = new Uint8Array(1);
      globalThis.crypto.getRandomValues(bytes);
      indices.add(bytes[0] % 24);
    }
    setVerifyIndices([...indices].sort((a, b) => a - b));
    setVerifyAnswers(["", "", ""]);
    setVerifyError("");
    setSeedConfirmed(false);
    setScreen("create-seed");
  }

  function checkSeedVerify(): boolean {
    const words = newMnemonic.split(" ");
    for (let i = 0; i < verifyIndices.length; i++) {
      if (
        verifyAnswers[i].trim().toLowerCase() !==
        words[verifyIndices[i]].toLowerCase()
      ) {
        setVerifyError(
          "Ek word galat hai — seed phrase dobara dekho aur sahi word likho.",
        );
        return false;
      }
    }
    setVerifyError("");
    return true;
  }

  function handleHandleInput(value: string): string | null {
    const handle = value.trim().replace(/^@/, "");
    if (!handle) return null;
    if (!/^[a-z0-9][a-z0-9_.-]{2,31}$/i.test(handle)) {
      return "Naam 3–32 akshar ka hona chahiye (sirf a-z, 0-9, _, -, .).";
    }
    if (RESERVED_HANDLES.has(handle.toLowerCase())) {
      return "Ye naam system ke liye reserved hai — koi aur naam chuno.";
    }
    return null;
  }

  async function finishCreateOrImport(mnemonic: string) {
    const pw = newPassword;
    if (pw.length < 12) {
      setPasswordError("Password kam se kam 12 akshar ka hona chahiye.");
      return;
    }
    if (pw !== confirmPassword) {
      setPasswordError("Dono password ek jaise nahi hain.");
      return;
    }
    const handleErr = handleHandleInput(handleInput);
    if (handleErr) {
      setPasswordError(handleErr);
      return;
    }
    setPasswordError("");
    setCreating(true);
    setCreateNote("");
    try {
      const wallet = await walletFromMnemonic(mnemonic);
      const vaultData: WalletVault = {
        address: wallet.address,
        publicKeyBase64: wallet.publicKeyBase64,
        privateKeyHex: wallet.privateKeyHex,
        algorithm: wallet.algorithm,
      };
      const encrypted = await encryptWalletVault(vaultData, pw, "fast");
      window.localStorage.setItem(VAULT_KEY, encrypted);
      let newMeta: WalletMeta = {
        address: wallet.address,
        butpayId: null,
        handle: null,
      };
      // BUT Pay ID lo (node online ho to). Na mile to baad me "ID lo" se milega.
      try {
        const handle = handleInput.trim()
          ? handleInput.trim().replace(/^@/, "")
          : undefined;
        const entry = await registerButPay(wallet.address, handle);
        newMeta = {
          address: wallet.address,
          butpayId: entry.id,
          handle: entry.handle,
        };
      } catch {
        setCreateNote(
          "Node se jud nahi paya, isliye BUT Pay ID abhi nahi mili — wallet khulne ke baad 'BUT Pay ID lo' dabao.",
        );
      }
      saveMeta(newMeta);
      setMeta(newMeta);
      setVault(vaultData);
      setPassword(pw);
      setHasVault(true);
      setNewPassword("");
      setConfirmPassword("");
      setHandleInput("");
      setPendingMnemonic("");
      setScreen("wallet");
    } catch (err) {
      setPasswordError(
        err instanceof Error ? err.message : "Wallet ban nahi paya.",
      );
    } finally {
      setCreating(false);
    }
  }

  async function handleImportSeed(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const words = importMnemonic.trim().split(/\s+/);
    if (words.length !== 24) {
      setImportError("Seed phrase bilkul 24 words ki honi chahiye.");
      return;
    }
    if (!(await validateMnemonic(importMnemonic))) {
      setImportError(
        "Seed phrase sahi nahi hai — koi word galat hai ya checksum galat hai. Dobara check karo.",
      );
      return;
    }
    setImportError("");
    setPendingMnemonic(importMnemonic.trim().split(/\s+/).join(" "));
    setScreen("import-password");
  }

  // ---- Unlock ----
  async function unlockWithPassword(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const pw = passwordInput;
    if (pw.length < 12) {
      setUnlockError("Password kam se kam 12 akshar ka hona chahiye.");
      return;
    }
    setUnlocking(true);
    setUnlockError("");
    try {
      const stored = window.localStorage.getItem(VAULT_KEY);
      if (!stored) {
        setUnlockError("Vault nahi mila — seed se wallet wapas lao.");
        return;
      }
      const opened = await decryptWalletVault(stored, pw);
      setVault(opened);
      setPassword(pw);
      setPasswordInput("");
      setScreen("wallet");
    } catch {
      setUnlockError(
        "Vault nahi khula — galat password ya data badla gaya hai.",
      );
    } finally {
      setUnlocking(false);
    }
  }

  async function unlockWithFingerprint() {
    setUnlocking(true);
    setUnlockError("");
    try {
      const pw = await unlockWithBiometric();
      if (!pw) {
        setUnlockError(
          "Biometric unlock nahi hua — password se unlock karo.",
        );
        return;
      }
      const stored = window.localStorage.getItem(VAULT_KEY);
      if (!stored) {
        setUnlockError("Vault nahi mila — seed se wallet wapas lao.");
        return;
      }
      const opened = await decryptWalletVault(stored, pw);
      setVault(opened);
      setPassword(pw);
      setScreen("wallet");
    } catch {
      setUnlockError("Biometric unlock nahi hua — password se unlock karo.");
    } finally {
      setUnlocking(false);
    }
  }

  async function enableBiometric() {
    if (!password) return;
    setBiometricNote("");
    try {
      await setupBiometricUnlock(password);
      setBiometricReady(true);
      setBiometricNote("Biometric unlock chalu ho gaya.");
    } catch (err) {
      setBiometricNote(
        err instanceof Error
          ? err.message
          : "Biometric setup nahi hua — password se unlock karte raho.",
      );
    }
  }

  function resetToImport() {
    if (
      !window.confirm(
        "Vault delete karke seed se wallet wapas laoge? Vault delete hone ke baad sirf seed phrase se wallet wapas milega.",
      )
    ) {
      return;
    }
    try {
      window.localStorage.removeItem(VAULT_KEY);
      window.localStorage.removeItem(META_KEY);
    } catch {
      // Ignore.
    }
    clearBiometricUnlock();
    setHasVault(false);
    setMeta(null);
    setVault(null);
    setPassword(null);
    setScreen("onboarding");
  }

  // ---- BUT Pay ID dobara lo ----
  async function claimButPayId() {
    if (!vault) return;
    setRegisteringId(true);
    setIdNote("");
    try {
      const entry = await registerButPay(vault.address, undefined);
      const newMeta: WalletMeta = {
        address: vault.address,
        butpayId: entry.id,
        handle: entry.handle,
      };
      saveMeta(newMeta);
      setMeta(newMeta);
      setIdNote(`BUT Pay ID mil gayi: ${entry.id}`);
    } catch (err) {
      setIdNote(
        err instanceof NodeError
          ? err.message
          : "ID nahi mili — node online hai ya nahi check karo.",
      );
    } finally {
      setRegisteringId(false);
    }
  }

  // ---- Receive: QR banao ----
  async function openReceive() {
    setDialog("receive");
    setQrDataUrl("");
    if (!vault) return;
    try {
      const url = await QRCode.toDataURL(vault.address, {
        errorCorrectionLevel: "M",
        margin: 1,
        width: 220,
      });
      setQrDataUrl(url);
    } catch {
      setQrDataUrl("");
    }
  }

  // ---- Send ----
  function openSend() {
    setReceiverInput("");
    setAmountInput("");
    setSendError("");
    setSendStep("form");
    setDialog("send");
  }

  async function resolveAndPreview() {
    if (!vault) return;
    const parsed = parseReceiverInput(receiverInput);
    if (parsed.kind === "invalid") {
      setSendError(parsed.error);
      return;
    }
    const amount = butToBits(amountInput);
    if (!amount.ok) {
      setSendError(amount.error);
      return;
    }
    if (balanceBits === null) {
      setSendError("Balance abhi pata nahi — pehle node se judo.");
      return;
    }
    setSendError("");
    try {
      let address = "";
      let label = "";
      if (parsed.kind === "address") {
        address = parsed.value;
        label = parsed.value;
      } else {
        const entry = await lookupButPay(parsed.value);
        address = entry.address;
        label = entry.handle
          ? `@${entry.handle} (ID ${entry.id})`
          : `ID ${entry.id}`;
      }
      const known = getKnownReceivers();
      const warnings = checkSend({
        resolvedAddress: address,
        ownAddress: vault.address,
        amountBits: amount.bits,
        feeBits: DEFAULT_FEE_BITS,
        balanceBits,
        isKnownReceiver: known.includes(address),
      });
      setResolvedAddress(address);
      setResolvedLabel(label);
      setPreviewBits(amount.bits);
      setPreviewWarnings(warnings);
      setSendStep("preview");
    } catch (err) {
      setSendError(
        err instanceof NodeError
          ? err.message
          : "Receiver nahi mila — ID/naam/address check karo.",
      );
    }
  }

  async function confirmAndSend() {
    if (!vault || !password) return;
    if (previewWarnings.some((w) => w.level === "block")) return;
    setSendStep("sending");
    setSendError("");
    try {
      const balance = await fetchBalance(vault.address);
      // NOTE: receiver hamesha user ka likha hua string hota hai (address
      // ya ID/handle) — signature usi par verify hoti hai, aur credit
      // resolve kiye gaye address ko milta hai.
      const tx = await signTransaction(
        {
          sender: vault.address,
          receiver: receiverInput.trim(),
          amount: previewBits,
          fee: DEFAULT_FEE_BITS,
          nonce: balance.nonce,
        },
        vault.privateKeyHex,
      );
      const submitted = await submitTx(tx);
      const mined = await mine();
      addKnownReceiver(resolvedAddress);
      setSentTxId(submitted.txId);
      setSentBlockHeight(mined.height);
      setSendStep("done");
      setBalanceBits(balance.balance - previewBits - DEFAULT_FEE_BITS);
      void refreshChain();
    } catch (err) {
      setSendError(
        err instanceof NodeError
          ? err.message
          : "Bheja nahi gaya — node ya balance check karo.",
      );
      setSendStep("preview");
    }
  }

  function closeDialog() {
    setDialog(null);
    setSendError("");
    setSendStep("form");
  }

  async function copyValue(text: string, label: string) {
    const ok = await copyText(text);
    if (ok) {
      setCopied(label);
      window.setTimeout(() => setCopied(""), 1800);
    }
  }

  const words = newMnemonic ? newMnemonic.split(" ") : [];
  const blocked = previewWarnings.some((w) => w.level === "block");

  // ---------------------------------------------------------------------------
  // Render — boot
  // ---------------------------------------------------------------------------
  if (screen === "boot") {
    return (
      <div className="app-shell">
        <main className="page-wrap">
          <p className="subheading">BUT Wallet khul raha hai…</p>
        </main>
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Render — onboarding (naya wallet / import)
  // ---------------------------------------------------------------------------
  if (
    screen === "onboarding" ||
    screen === "create-seed" ||
    screen === "create-verify" ||
    screen === "create-password" ||
    screen === "import-seed" ||
    screen === "import-password"
  ) {
    return (
      <div className="app-shell">
        <div className="workspace">
          <main className="page-wrap">
            <div className="onboarding-wrap">
              <div className="page-heading">
                <div>
                  <p className="eyebrow">BUT Wallet</p>
                  <h1>
                    {screen === "onboarding"
                      ? "Apna wallet banao"
                      : screen.startsWith("create")
                        ? "Naya wallet"
                        : "Seed se wallet wapas lao"}
                  </h1>
                  <p className="subheading">
                    Private key sirf tumhare phone par rehti hai — server par
                    kabhi nahi jati. Seed phrase kagaz par likhna, photo ya
                    cloud me kabhi nahi.
                  </p>
                </div>
              </div>

              {screen === "onboarding" && (
                <div className="choice-grid">
                  <button
                    className="choice-card"
                    onClick={() => void startCreate()}
                    type="button"
                  >
                    <Wallet size={22} />
                    <strong>Naya wallet banao</strong>
                    <span>24 words ki seed phrase milegi — use kagaz par likho.</span>
                  </button>
                  <button
                    className="choice-card"
                    onClick={() => {
                      setImportMnemonic("");
                      setImportError("");
                      setScreen("import-seed");
                    }}
                    type="button"
                  >
                    <History size={22} />
                    <strong>Seed se wallet wapas lao</strong>
                    <span>Apni 24 words ki seed phrase likho — wahi wallet wapas milega.</span>
                  </button>
                </div>
              )}

              {screen === "create-seed" && (
                <section className="activity-panel">
                  <div className="section-heading">
                    <div>
                      <p className="eyebrow">Step 1 — Seed backup</p>
                      <h2>Ye 24 words kagaz par likho</h2>
                    </div>
                  </div>
                  <div className="privacy-callout hot-callout">
                    <ShieldAlert size={19} />
                    <p>
                      <strong>Ye words hi tumhara wallet hain.</strong> Jiske
                      paas ye words hain, uske paas tumhare paise hain. Photo
                      mat lo, cloud ya chat me mat likho — sirf kagaz par.
                    </p>
                  </div>
                  <div className="seed-grid">
                    {words.map((word, i) => (
                      <span className="seed-word" key={i}>
                        <i>{i + 1}</i> {word}
                      </span>
                    ))}
                  </div>
                  <label className="check-row">
                    <input
                      type="checkbox"
                      checked={seedConfirmed}
                      onChange={(e) => setSeedConfirmed(e.target.checked)}
                    />
                    Maine saare 24 words kagaz par likh liye hain.
                  </label>
                  <div className="modal-actions">
                    <button
                      className="action-button action-secondary"
                      onClick={() => setScreen("onboarding")}
                      type="button"
                    >
                      Wapas
                    </button>
                    <button
                      className="action-button action-primary"
                      disabled={!seedConfirmed}
                      onClick={() => setScreen("create-verify")}
                      type="button"
                    >
                      Aage — Verify karo
                    </button>
                  </div>
                </section>
              )}

              {screen === "create-verify" && (
                <section className="activity-panel">
                  <div className="section-heading">
                    <div>
                      <p className="eyebrow">Step 2 — Verify karo</p>
                      <h2>Seed sahi likhi ya nahi, check karo</h2>
                    </div>
                  </div>
                  <p className="subheading">
                    Neeche puche gaye number wale words likho — isse pata
                    chalega ki tumne seed sahi likhi hai.
                  </p>
                  <div className="verify-grid">
                    {verifyIndices.map((wordIndex, i) => (
                      <label className="form-field" key={wordIndex}>
                        <span>Word #{wordIndex + 1}</span>
                        <input
                          value={verifyAnswers[i]}
                          onChange={(event) => {
                            const next = [...verifyAnswers];
                            next[i] = event.target.value;
                            setVerifyAnswers(next);
                          }}
                          placeholder={`Word ${wordIndex + 1} likho`}
                          autoComplete="off"
                        />
                      </label>
                    ))}
                  </div>
                  {verifyError && (
                    <p className="form-error" role="alert">{verifyError}</p>
                  )}
                  <div className="modal-actions">
                    <button
                      className="action-button action-secondary"
                      onClick={() => setScreen("create-seed")}
                      type="button"
                    >
                      Wapas — Seed dekho
                    </button>
                    <button
                      className="action-button action-primary"
                      onClick={() => {
                        if (checkSeedVerify()) {
                          setPendingMnemonic(newMnemonic);
                          setScreen("create-password");
                        }
                      }}
                      type="button"
                    >
                      Verify karo
                    </button>
                  </div>
                </section>
              )}

              {screen === "import-seed" && (
                <section className="activity-panel">
                  <form onSubmit={handleImportSeed}>
                    <div className="section-heading">
                      <div>
                        <p className="eyebrow">Seed se restore</p>
                        <h2>Apni 24 words likho</h2>
                      </div>
                    </div>
                    <label className="form-field">
                      <span>Seed phrase (24 words, space se alag)</span>
                      <textarea
                        className="seed-textarea"
                        value={importMnemonic}
                        onChange={(event) => setImportMnemonic(event.target.value)}
                        placeholder="word1 word2 word3 … (kul 24 words)"
                        rows={4}
                      />
                    </label>
                    {importError && (
                      <p className="form-error" role="alert">{importError}</p>
                    )}
                    <div className="modal-actions">
                      <button
                        className="action-button action-secondary"
                        onClick={() => setScreen("onboarding")}
                        type="button"
                      >
                        Wapas
                      </button>
                      <button className="action-button action-primary" type="submit">
                        Seed check karo
                      </button>
                    </div>
                  </form>
                </section>
              )}

              {(screen === "create-password" || screen === "import-password") && (
                <section className="activity-panel">
                  <div className="section-heading">
                    <div>
                      <p className="eyebrow">Step 3 — Password</p>
                      <h2>Wallet ka password banao</h2>
                    </div>
                  </div>
                  <p className="subheading">
                    Ye password tumhara unlock PIN hai — wallet kholne ke liye
                    yehi chahiye hoga. Password kahin save nahi hota, sirf
                    tumhari memory me rehta hai.
                  </p>
                  <label className="form-field">
                    <span>Password (kam se kam 12 akshar)</span>
                    <input
                      type="password"
                      value={newPassword}
                      onChange={(event) => setNewPassword(event.target.value)}
                      placeholder="Mazboot password likho"
                      autoComplete="new-password"
                    />
                  </label>
                  <label className="form-field">
                    <span>Password dobara likho</span>
                    <input
                      type="password"
                      value={confirmPassword}
                      onChange={(event) => setConfirmPassword(event.target.value)}
                      placeholder="Password confirm karo"
                      autoComplete="new-password"
                    />
                  </label>
                  <label className="form-field">
                    <span>BUT Pay naam (optional — jaise @sanjay)</span>
                    <input
                      value={handleInput}
                      onChange={(event) => setHandleInput(event.target.value)}
                      placeholder="Naam chuno ya khaali chhodo"
                      autoComplete="off"
                    />
                  </label>
                  {passwordError && (
                    <p className="form-error" role="alert">{passwordError}</p>
                  )}
                  {createNote && (
                    <div className="privacy-callout hot-callout" role="status">
                      <ShieldCheck size={19} />
                      <p>{createNote}</p>
                    </div>
                  )}
                  <div className="modal-actions">
                    <button
                      className="action-button action-secondary"
                      onClick={() =>
                        setScreen(
                          screen === "create-password"
                            ? "create-verify"
                            : "import-seed",
                        )
                      }
                      type="button"
                    >
                      Wapas
                    </button>
                    <button
                      className="action-button action-primary"
                      disabled={creating}
                      onClick={() => void finishCreateOrImport(pendingMnemonic)}
                      type="button"
                    >
                      {creating ? "Wallet ban raha hai…" : "Wallet banao"}
                    </button>
                  </div>
                </section>
              )}
            </div>
          </main>
        </div>
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Render — locked
  // ---------------------------------------------------------------------------
  if (screen === "locked") {
    return (
      <div className="app-shell">
        <div className="workspace">
          <main className="page-wrap">
            <section className="ledger-page" aria-label="Wallet lock">
              <div className="page-heading">
                <div>
                  <p className="eyebrow">BUT Wallet</p>
                  <h1>Wallet lock hai</h1>
                  <p className="subheading">
                    Khulne ke liye password likho. Password sirf tumhari
                    memory me rehta hai — bhool gaye to sirf seed phrase se
                    wallet wapas milega.
                  </p>
                </div>
              </div>
              <form onSubmit={unlockWithPassword} className="activity-panel">
                <label className="form-field">
                  <span>Password (kam se kam 12 akshar)</span>
                  <input
                    autoFocus
                    type="password"
                    value={passwordInput}
                    onChange={(event) => setPasswordInput(event.target.value)}
                    placeholder="Password likho"
                    autoComplete="current-password"
                  />
                </label>
                {unlockError && (
                  <p className="form-error" role="alert">{unlockError}</p>
                )}
                <div className="modal-actions">
                  <button
                    className="action-button action-primary"
                    type="submit"
                    disabled={unlocking}
                  >
                    {unlocking ? "Unlock ho raha hai…" : "Unlock karo"}
                  </button>
                </div>
              </form>

              {biometricAvailable && (
                <div className="activity-panel">
                  <div className="section-heading">
                    <div>
                      <p className="eyebrow">Biometric unlock</p>
                      <h2>Fingerprint / Face ID</h2>
                    </div>
                  </div>
                  {biometricReady ? (
                    <button
                      className="action-button action-outline"
                      onClick={() => void unlockWithFingerprint()}
                      disabled={unlocking}
                      type="button"
                    >
                      <Fingerprint size={17} /> Fingerprint se unlock karo
                    </button>
                  ) : (
                    <p className="balance-subtext">
                      Is browser/device me biometric available hai. Unlock
                      karne ke baad wallet screen se biometric chalu kar sakte
                      ho.
                    </p>
                  )}
                </div>
              )}
              {!biometricAvailable && isWebAuthnAvailable() && (
                <p className="balance-subtext">
                  Is browser/device me biometric (PRF) support nahi hai —
                  password se unlock karo. Android app me hardware-backed
                  biometric aayega.
                </p>
              )}
              <p className="balance-subtext">
                Password bhool gaye?{" "}
                <button className="inline-link" onClick={resetToImport} type="button">
                  Seed se wallet wapas lao
                </button>
              </p>
            </section>
          </main>
        </div>
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Render — wallet (unlocked)
  // ---------------------------------------------------------------------------
  const address = vault?.address ?? meta?.address ?? "";
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">B</div>
          <div className="brand-copy">
            <strong>BUT</strong>
            <span>Wallet</span>
          </div>
        </div>

        <div className="sidebar-caption">मेन्यू</div>
        <nav className="sidebar-nav" aria-label="मुख्य नेविगेशन">
          <button
            className={`nav-item ${tab === "wallet" ? "active" : ""}`}
            onClick={() => setTab("wallet")}
            type="button"
          >
            <LayoutDashboard size={18} />
            <span>वॉलेट</span>
          </button>
          <button
            className={`nav-item ${tab === "history" ? "active" : ""}`}
            onClick={() => setTab("history")}
            type="button"
          >
            <History size={18} />
            <span>लेन-देन</span>
          </button>
        </nav>

        <div className="sidebar-status">
          <div className={`status-dot ${nodeOnline ? "ok" : "bad"}`} />
          <div>
            <strong>
              {nodeOnline === null
                ? "Node check ho raha hai…"
                : nodeOnline
                  ? `Node se juda — Block #${nodeHeight ?? "?"}`
                  : "Node se juda nahi"}
            </strong>
            <span>
              {nodeOnline
                ? "Balance chain se aa raha hai"
                : "Node chalao: cargo run -- serve"}
            </span>
          </div>
        </div>
        <div className="sidebar-footnote">
          <button className="text-button" onClick={lockWallet} type="button">
            <LockKeyhole size={15} /> Lock karo
          </button>
          <span>5 minute bina use ke apne aap lock hoga।</span>
        </div>
      </aside>

      <div className="workspace">
        <header className="topbar">
          <div className="breadcrumb">
            <span>BUT Wallet</span>
            <ChevronRight size={15} />
            <strong>{tab === "wallet" ? "वॉलेट" : "लेन-देन"}</strong>
          </div>
          <div className="topbar-right">
            <span className={`network-pill ${nodeOnline ? "ok" : "bad"}`}>
              <span className="status-dot" />
              {nodeOnline ? "Chain se juda" : "Node offline"}
            </span>
            <span className="avatar" aria-label="Wallet">
              {(meta?.handle?.[0] ?? "B").toUpperCase()}
            </span>
          </div>
        </header>

        <main className="page-wrap">
          {chainError && (
            <div className="warn-box" role="alert">
              <ShieldAlert size={19} />
              <p>
                <strong>Node se jud nahi paya.</strong> {chainError} Balance
                aur history tabhi dikhenge jab node chal raha hoga.
              </p>
            </div>
          )}
          {createNote && (
            <div className="privacy-callout hot-callout" role="status">
              <ShieldCheck size={19} />
              <p>{createNote}</p>
            </div>
          )}

          {tab === "wallet" && (
            <>
              <div className="page-heading">
                <div>
                  <p className="eyebrow">आपका वॉलेट</p>
                  <h1>नमस्ते</h1>
                  <p className="subheading">
                    {meta?.handle ? `@${meta.handle}` : "BUT Wallet"} — balance
                    aur lenden seedha chain se aate hain।
                  </p>
                </div>
                <button
                  className="text-button"
                  onClick={() => void refreshChain()}
                  disabled={refreshing}
                  type="button"
                >
                  {refreshing ? "Refresh ho raha hai…" : "Refresh karo"}
                </button>
              </div>

              <section className="dashboard-grid" aria-label="वॉलेट सारांश">
                <div className="balance-card">
                  <div className="balance-card-top">
                    <div className="balance-card-label">
                      <Wallet size={17} /> कुल बैलेंस
                    </div>
                    <span className="demo-chip">CHAIN SE</span>
                  </div>
                  <div className="balance-value-row">
                    <div className="balance-value">
                      {balanceVisible
                        ? balanceBits === null
                          ? "—"
                          : formatBits(balanceBits).replace(" BUT", "")
                        : "••••••"}
                      <span>BUT</span>
                    </div>
                    <button
                      className="icon-button balance-eye"
                      onClick={() => setBalanceVisible((v) => !v)}
                      aria-label={balanceVisible ? "बैलेंस छिपाएँ" : "बैलेंस दिखाएँ"}
                      type="button"
                    >
                      {balanceVisible ? <Eye size={19} /> : <EyeOff size={19} />}
                    </button>
                  </div>
                  <p className="balance-subtext">
                    1 BUT = 1,000 bits. Fee sirf 1 bit (0.001 BUT)।
                  </p>
                  <div className="balance-actions">
                    <button
                      className="action-button action-light"
                      onClick={openSend}
                      disabled={!nodeOnline}
                      type="button"
                    >
                      <ArrowUpRight size={17} /> भेजें
                    </button>
                    <button
                      className="action-button action-outline"
                      onClick={() => void openReceive()}
                      type="button"
                    >
                      <ArrowDownLeft size={17} /> प्राप्त करें
                    </button>
                  </div>
                  <div className="balance-decoration" aria-hidden="true"><i /><i /><i /></div>
                </div>

                <div className="network-card">
                  <div className="card-heading-row">
                    <div>
                      <p className="eyebrow">BUT Pay</p>
                      <h2>तुम्हारी ID</h2>
                    </div>
                    <div className="network-icon"><QrCode size={20} /></div>
                  </div>
                  {meta?.butpayId ? (
                    <>
                      <div className="butpay-id">
                        <strong>#{meta.butpayId}</strong>
                        {meta.handle && <span>@{meta.handle}</span>}
                      </div>
                      <p className="balance-subtext">
                        Koi bhi is number ya naam se tumhe paise bhej sakta
                        hai — address likhne ki zaroorat nahi।
                      </p>
                    </>
                  ) : (
                    <>
                      <p className="balance-subtext">
                        BUT Pay ID abhi nahi mili — node se judte hi milegi।
                      </p>
                      <button
                        className="action-button action-primary"
                        onClick={() => void claimButPayId()}
                        disabled={registeringId || !nodeOnline}
                        type="button"
                      >
                        {registeringId ? "ID mil rahi hai…" : "BUT Pay ID lo"}
                      </button>
                    </>
                  )}
                  {idNote && <p className="balance-subtext">{idNote}</p>}
                  <div className="network-foot">
                    <span className="status-dot" />
                    Address: {shortAddress(address)}
                    <button
                      className="copy-button"
                      onClick={() => void copyValue(address, "address")}
                      type="button"
                    >
                      {copied === "address" ? <Check size={15} /> : <Copy size={15} />}
                      {copied === "address" ? "कॉपी हो गया" : "कॉपी करें"}
                    </button>
                  </div>
                </div>
              </section>

              <section className="activity-panel">
                <div className="section-heading activity-heading">
                  <div>
                    <p className="eyebrow">हाल की गतिविधि</p>
                    <h2>लेन-देन</h2>
                  </div>
                  <button className="inline-link" onClick={() => setTab("history")} type="button">
                    पूरा इतिहास <ChevronRight size={16} />
                  </button>
                </div>
                <HistoryList entries={history.slice(0, 4)} />
              </section>

              <section className="activity-panel">
                <div className="section-heading">
                  <div>
                    <p className="eyebrow">सुरक्षा</p>
                    <h2>Lock aur biometric</h2>
                  </div>
                </div>
                <p className="balance-subtext">
                  Wallet 5 minute bina use ke apne aap lock ho jata hai।
                  {biometricAvailable
                    ? biometricReady
                      ? " Biometric unlock chalu hai।"
                      : " Is device par biometric available hai — neeche se chalu karo।"
                    : " Is browser/device par biometric (PRF) support nahi hai — Android app me hardware-backed biometric aayega।"}
                </p>
                {biometricAvailable && !biometricReady && (
                  <button
                    className="action-button action-outline"
                    onClick={() => void enableBiometric()}
                    type="button"
                  >
                    <Fingerprint size={17} /> Biometric unlock chalu karo
                  </button>
                )}
                {biometricNote && <p className="balance-subtext">{biometricNote}</p>}
              </section>
            </>
          )}

          {tab === "history" && (
            <section className="ledger-page">
              <div className="page-heading">
                <div>
                  <p className="eyebrow">Chain se</p>
                  <h1>लेन-देन का इतिहास</h1>
                  <p className="subheading">
                    Ye list chain ke blocks se banti hai — browser me kuch
                    save nahi hota।
                  </p>
                </div>
                <button
                  className="action-button action-primary"
                  onClick={openSend}
                  disabled={!nodeOnline}
                  type="button"
                >
                  <ArrowUpRight size={17} /> नया भेजें
                </button>
              </div>
              <div className="activity-panel ledger-panel">
                <HistoryList entries={history} />
              </div>
            </section>
          )}
        </main>
      </div>

      {/* ---- Send modal ---- */}
      {dialog === "send" && (
        <div className="modal-scrim" onMouseDown={(event) => event.target === event.currentTarget && closeDialog()}>
          <section className="wallet-modal" role="dialog" aria-modal="true" aria-labelledby="send-title">
            <div className="modal-header">
              <div>
                <p className="eyebrow">BUT Pay — Send</p>
                <h2 id="send-title">BUT भेजें</h2>
              </div>
              <button className="icon-button modal-close" onClick={closeDialog} aria-label="बंद करें" type="button"><X size={20} /></button>
            </div>

            {sendStep === "form" && (
              <>
                <label className="form-field">
                  <span>किसको भेजना है — Address, BUT Pay ID ya naam</span>
                  <input
                    autoFocus
                    value={receiverInput}
                    onChange={(event) => setReceiverInput(event.target.value)}
                    placeholder="but1… ya 100001 ya @sanjay"
                    autoComplete="off"
                  />
                </label>
                <button
                  className="action-button action-outline"
                  onClick={() => setDialog("scan")}
                  type="button"
                >
                  <ScanLine size={17} /> QR scan karo
                </button>
                <label className="form-field">
                  <span>कितना भेजना है (BUT)</span>
                  <div className="amount-input-wrap">
                    <input
                      value={amountInput}
                      onChange={(event) => setAmountInput(event.target.value)}
                      inputMode="decimal"
                      placeholder="0.00"
                      autoComplete="off"
                    />
                    <span>BUT</span>
                  </div>
                </label>
                <p className="balance-subtext">
                  Fee: {DEFAULT_FEE_BITS} bit (0.001 BUT)। Balance:{" "}
                  {balanceBits === null ? "—" : formatBits(balanceBits)}।
                </p>
                {sendError && <p className="form-error" role="alert">{sendError}</p>}
                <div className="modal-actions">
                  <button className="action-button action-secondary" onClick={closeDialog} type="button">रद्द करें</button>
                  <button
                    className="action-button action-primary"
                    onClick={() => void resolveAndPreview()}
                    type="button"
                  >
                    Preview dikhao
                  </button>
                </div>
              </>
            )}

            {sendStep === "preview" && (
              <>
                <div className="preview-table">
                  <div><span>किसको</span><strong>{resolvedLabel}</strong></div>
                  <div><span>Address</span><strong>{shortAddress(resolvedAddress)}</strong></div>
                  <div><span>रकम</span><strong>{bitsToBut(previewBits)} BUT ({previewBits} bits)</strong></div>
                  <div><span>Fee</span><strong>{DEFAULT_FEE_BITS} bit (0.001 BUT)</strong></div>
                  <div><span>Kul (रकम + fee)</span><strong>{formatBits(previewBits + DEFAULT_FEE_BITS)}</strong></div>
                  <div><span>Bhejne ke baad balance</span><strong>{balanceBits === null ? "—" : formatBits(balanceBits - previewBits - DEFAULT_FEE_BITS)}</strong></div>
                </div>
                {previewWarnings.map((w, i) => (
                  <div
                    key={i}
                    className={w.level === "block" ? "block-box" : w.level === "warn" ? "warn-box" : "info-box"}
                    role="alert"
                  >
                    <ShieldAlert size={19} />
                    <p>{w.text}</p>
                  </div>
                ))}
                {sendError && <p className="form-error" role="alert">{sendError}</p>}
                <div className="modal-actions">
                  <button
                    className="action-button action-secondary"
                    onClick={() => setSendStep("form")}
                    type="button"
                  >
                    Wapas
                  </button>
                  <button
                    className="action-button action-primary"
                    disabled={blocked}
                    onClick={() => void confirmAndSend()}
                    type="button"
                  >
                    Confirm karke bhejo
                  </button>
                </div>
              </>
            )}

            {sendStep === "sending" && (
              <p className="subheading">
                Transaction sign ho rahi hai aur node ko bheji ja rahi hai…
              </p>
            )}

            {sendStep === "done" && (
              <>
                <div className="privacy-callout hot-callout" role="status">
                  <ShieldCheck size={19} />
                  <p>
                    <strong>Paise bhej diye gaye!</strong> Block #{sentBlockHeight}{" "}
                    me confirm hua। Transaction ID: {sentTxId.slice(0, 16)}…
                  </p>
                </div>
                <div className="modal-actions">
                  <button className="action-button action-primary" onClick={closeDialog} type="button">
                    Ho gaya
                  </button>
                </div>
              </>
            )}
          </section>
        </div>
      )}

      {/* ---- Receive modal ---- */}
      {dialog === "receive" && (
        <div className="modal-scrim" onMouseDown={(event) => event.target === event.currentTarget && closeDialog()}>
          <section className="wallet-modal" role="dialog" aria-modal="true" aria-labelledby="receive-title">
            <div className="modal-header">
              <div>
                <p className="eyebrow">BUT Pay — Receive</p>
                <h2 id="receive-title">BUT प्राप्त करें</h2>
              </div>
              <button className="icon-button modal-close" onClick={closeDialog} aria-label="बंद करें" type="button"><X size={20} /></button>
            </div>
            <div className="qr-box">
              {qrDataUrl ? (
                <img src={qrDataUrl} alt="Receive QR code" width={220} height={220} />
              ) : (
                <p className="balance-subtext">QR code ban raha hai…</p>
              )}
            </div>
            <p className="balance-subtext">
              QR scan karte hi paise bhej sakte hain। QR me sirf public
              info hai — private key ya seed phrase kabhi nahi।
            </p>
            <div className="receive-address">
              <span>तुम्हारा Address</span>
              <strong>{address}</strong>
              <button className="copy-button" onClick={() => void copyValue(address, "recv-address")} type="button">
                {copied === "recv-address" ? <Check size={15} /> : <Copy size={15} />}
                {copied === "recv-address" ? "कॉपी हो गया" : "Address कॉपी करें"}
              </button>
            </div>
            {meta?.butpayId && (
              <div className="receive-address">
                <span>तुम्हारी BUT Pay ID</span>
                <strong>#{meta.butpayId}{meta.handle ? ` (@${meta.handle})` : ""}</strong>
                <button
                  className="copy-button"
                  onClick={() => void copyValue(String(meta.butpayId), "recv-id")}
                  type="button"
                >
                  {copied === "recv-id" ? <Check size={15} /> : <Copy size={15} />}
                  {copied === "recv-id" ? "कॉपी हो गया" : "ID कॉपी करें"}
                </button>
              </div>
            )}
            <div className="modal-actions">
              <button className="action-button action-primary" onClick={closeDialog} type="button">
                Ho gaya
              </button>
            </div>
          </section>
        </div>
      )}

      {/* ---- QR scan modal ---- */}
      {dialog === "scan" && (
        <div className="modal-scrim" onMouseDown={(event) => event.target === event.currentTarget && closeDialog()}>
          <section className="wallet-modal" role="dialog" aria-modal="true" aria-labelledby="scan-title">
            <div className="modal-header">
              <div>
                <p className="eyebrow">BUT Pay — QR Scan</p>
                <h2 id="scan-title">QR scan karo</h2>
              </div>
              <button className="icon-button modal-close" onClick={closeDialog} aria-label="बंद करें" type="button"><X size={20} /></button>
            </div>
            <QrScanner
              onResult={(text) => {
                setReceiverInput(text.trim());
                setDialog("send");
              }}
              onCancel={() => setDialog("send")}
            />
          </section>
        </div>
      )}
    </div>
  );
}

function HistoryList({ entries }: { entries: HistoryEntry[] }) {
  if (entries.length === 0) {
    return (
      <div className="empty-ledger">
        Abhi koi lenden nahi hai। Paise bhejte hi yahan dikhega।
      </div>
    );
  }
  return (
    <div className="transaction-list">
      {entries.map((entry) => {
        const isSent = entry.direction === "sent";
        return (
          <article className="transaction-row" key={entry.key}>
            <div className={`transaction-icon ${isSent ? "sent" : "received"}`}>
              {isSent ? <ArrowUpRight size={18} /> : <ArrowDownLeft size={18} />}
            </div>
            <div className="transaction-main">
              <div className="transaction-name">
                <strong>{isSent ? "आपने भेजा" : "आपको मिला"}</strong>
              </div>
              <span className="transaction-address">{shortAddress(entry.otherParty)}</span>
              <span className="transaction-hash">Block #{entry.blockIndex}</span>
            </div>
            <div className="transaction-side">
              <strong className={`transaction-amount ${isSent ? "negative" : "positive"}`}>
                {`${isSent ? "−" : "+"}${formatBits(entry.amount)}`}
              </strong>
              <span>{formatTime(entry.timestamp)}</span>
              <span className="status-label"><span className="status-dot" />पुष्ट</span>
            </div>
          </article>
        );
      })}
    </div>
  );
}

function QrScanner({
  onResult,
  onCancel,
}: {
  onResult: (text: string) => void;
  onCancel: () => void;
}) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState("");
  const onResultRef = useRef(onResult);
  onResultRef.current = onResult;

  useEffect(() => {
    let stream: MediaStream | null = null;
    let raf = 0;
    let stopped = false;
    async function start() {
      try {
        if (!navigator.mediaDevices?.getUserMedia) {
          throw new Error("no camera");
        }
        stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: "environment" },
        });
        const video = videoRef.current;
        if (!video) return;
        video.srcObject = stream;
        await video.play();
        const tick = () => {
          if (stopped) return;
          const canvas = canvasRef.current;
          if (video.readyState === video.HAVE_ENOUGH_DATA && canvas) {
            canvas.width = video.videoWidth;
            canvas.height = video.videoHeight;
            const ctx = canvas.getContext("2d", { willReadFrequently: true });
            if (ctx) {
              ctx.drawImage(video, 0, 0);
              const img = ctx.getImageData(0, 0, canvas.width, canvas.height);
              const code = jsQR(img.data, img.width, img.height);
              if (code?.data) {
                onResultRef.current(code.data);
                return;
              }
            }
          }
          raf = requestAnimationFrame(tick);
        };
        raf = requestAnimationFrame(tick);
      } catch {
        setError(
          "Camera nahi khula — permission do, ya ID/address type karke bhejo।",
        );
      }
    }
    void start();
    return () => {
      stopped = true;
      cancelAnimationFrame(raf);
      stream?.getTracks().forEach((t) => t.stop());
    };
  }, []);

  return (
    <div className="scan-box">
      {error ? (
        <p className="form-error" role="alert">{error}</p>
      ) : (
        <>
          <video ref={videoRef} className="scan-video" muted playsInline />
          <canvas ref={canvasRef} style={{ display: "none" }} />
          <p className="balance-subtext">
            QR code ko camera ke saamne rakho — milte hi receiver bhar jayega।
          </p>
        </>
      )}
      <div className="modal-actions">
        <button className="action-button action-secondary" onClick={onCancel} type="button">
          Cancel
        </button>
      </div>
    </div>
  );
}
