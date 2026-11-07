import { useEffect, useState } from "react";
import {
  ArrowDownLeft,
  ArrowUpRight,
  Blocks,
  Check,
  ChevronRight,
  Copy,
  Eye,
  EyeOff,
  HardDrive,
  LayoutDashboard,
  LockKeyhole,
  ShieldAlert,
  ShieldCheck,
  Wallet,
  X,
  Zap,
} from "lucide-react";

type Mode = "hot" | "cold";
type Tab = "wallet" | "ledger" | "storage";
type DialogKind = "send" | "receive";
type Transaction = {
  id: string;
  direction: "sent" | "received";
  amount: number;
  address: string;
  createdAt: string;
  status: "पुष्ट (डेमो)" | "लंबित (डेमो)";
  hash: string;
  mode: Mode;
};

const BALANCE_KEY = "but-demo-balance-v1";
const TRANSACTIONS_KEY = "but-demo-transactions-v1";
const MODE_KEY = "but-demo-mode-v1";
const DEMO_RECEIVE_ADDRESS = "BUT-DEMO-7J2K-Q9LX";
const DEFAULT_BALANCE = 4280.5;

const starterTransactions: Transaction[] = [
  {
    id: "demo-001",
    direction: "received",
    amount: 320,
    address: "but1a84f29c7d6310",
    createdAt: "आज, 09:42",
    status: "पुष्ट (डेमो)",
    hash: "0x4d91a7c2…8b31",
    mode: "hot",
  },
  {
    id: "demo-002",
    direction: "sent",
    amount: 48.25,
    address: "but1c03d75a9e247b",
    createdAt: "कल, 18:16",
    status: "पुष्ट (डेमो)",
    hash: "0x1f7a309e…2c14",
    mode: "hot",
  },
  {
    id: "demo-003",
    direction: "received",
    amount: 125,
    address: "but1b92e014c5608f",
    createdAt: "30 अक्तू., 12:05",
    status: "लंबित (डेमो)",
    hash: "0x9c520ab1…7f06",
    mode: "cold",
  },
];

function readStored<T>(key: string, fallback: T): T {
  if (typeof window === "undefined") return fallback;
  try {
    const stored = window.localStorage.getItem(key);
    return stored ? (JSON.parse(stored) as T) : fallback;
  } catch {
    return fallback;
  }
}

function formatBut(value: number) {
  return new Intl.NumberFormat("en-IN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function currentTime() {
  return new Intl.DateTimeFormat("hi-IN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date());
}

// SECURITY FIX: Pehle yahan Math.random() use hota tha. Math.random()
// security ke liye safe nahi hai, isliye ab crypto.getRandomValues()
// (CSPRNG) use kar rahe hain. Ye sirf demo transaction ID ke liye hai,
// ye koi asli key ya asli hash nahi hai.
function randomDemoId() {
  const bytes = new Uint8Array(4);
  globalThis.crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0"))
    .join("")
    .toUpperCase();
}

function maskAddress(address: string) {
  if (address.length < 10) return "but••••••••";
  return `${address.slice(0, 6)}••••••${address.slice(-4)}`;
}

export default function Home() {
  const [mode, setMode] = useState<Mode>(() => {
    const saved = readStored<Mode>(MODE_KEY, "hot");
    return saved === "cold" ? "cold" : "hot";
  });
  const [tab, setTab] = useState<Tab>("wallet");
  const [balance, setBalance] = useState<number>(() =>
    readStored(BALANCE_KEY, DEFAULT_BALANCE),
  );
  const [transactions, setTransactions] = useState<Transaction[]>(() =>
    readStored(TRANSACTIONS_KEY, starterTransactions),
  );
  const [balanceVisible, setBalanceVisible] = useState(true);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [amount, setAmount] = useState("");
  const [address, setAddress] = useState("");
  const [formError, setFormError] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    window.localStorage.setItem(BALANCE_KEY, JSON.stringify(balance));
  }, [balance]);
  useEffect(() => {
    window.localStorage.setItem(TRANSACTIONS_KEY, JSON.stringify(transactions));
  }, [transactions]);
  useEffect(() => {
    window.localStorage.setItem(MODE_KEY, JSON.stringify(mode));
  }, [mode]);

  const openDialog = (kind: DialogKind) => {
    setAmount("");
    setAddress("");
    setFormError("");
    setDialog(kind);
  };

  const closeDialog = () => {
    setDialog(null);
    setFormError("");
  };

  const submitDemoTransaction = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const parsedAmount = Number(amount);
    if (!Number.isFinite(parsedAmount) || parsedAmount <= 0) {
      setFormError("कृपया शून्य से बड़ी डेमो राशि लिखें।");
      return;
    }
    if (dialog === "send" && parsedAmount > balance) {
      setFormError("डेमो बैलेंस से अधिक राशि नहीं भेज सकते।");
      return;
    }
    if (dialog === "send" && address.trim().length < 4) {
      setFormError("डेमो प्राप्तकर्ता का नाम या पता लिखें।");
      return;
    }

    const isSending = dialog === "send";
    const generated = randomDemoId();
    const entry: Transaction = {
      id: `demo-${generated}`,
      direction: isSending ? "sent" : "received",
      amount: parsedAmount,
      address: isSending ? address.trim() : DEMO_RECEIVE_ADDRESS,
      createdAt: currentTime(),
      status: "पुष्ट (डेमो)",
      hash: `0x${generated.toLowerCase()}…demo`,
      mode,
    };

    setTransactions((previous) => [entry, ...previous]);
    setBalance((previous) =>
      isSending ? previous - parsedAmount : previous + parsedAmount,
    );
    closeDialog();
    setTab("ledger");
  };

  const copyDemoAddress = async () => {
    try {
      await navigator.clipboard.writeText(DEMO_RECEIVE_ADDRESS);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch {
      setCopied(false);
    }
  };

  const resetDemo = () => {
    setBalance(DEFAULT_BALANCE);
    setTransactions(starterTransactions);
    setMode("hot");
    setTab("wallet");
  };

  const pageTitles: Record<Tab, string> = {
    wallet: "डेमो वॉलेट",
    ledger: "टेस्ट लेजर",
    storage: "ब्लॉक डेटा का आकार",
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">B</div>
          <div className="brand-copy">
            <strong>BUT</strong>
            <span>डेमो वॉलेट</span>
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
            className={`nav-item ${tab === "ledger" ? "active" : ""}`}
            onClick={() => setTab("ledger")}
            type="button"
          >
            <Blocks size={18} />
            <span>टेस्ट लेजर</span>
          </button>
          <button
            className={`nav-item ${tab === "storage" ? "active" : ""}`}
            onClick={() => setTab("storage")}
            type="button"
          >
            <HardDrive size={18} />
            <span>ब्लॉक स्पेस</span>
          </button>
        </nav>

        <div className="sidebar-status">
          <div className="status-dot" />
          <div>
            <strong>स्थानीय डेमो</strong>
            <span>किसी नेटवर्क से जुड़ा नहीं</span>
          </div>
        </div>
        <div className="sidebar-footnote">
          यह सीखने के लिए बनाया गया UI है। इसमें कोई असली कॉइन नहीं है।
        </div>
      </aside>

      <div className="workspace">
        <header className="topbar">
          <div className="breadcrumb">
            <span>प्रोटोटाइप</span>
            <ChevronRight size={15} />
            <strong>{pageTitles[tab]}</strong>
          </div>
          <div className="topbar-right">
            <span className="network-pill"><span className="status-dot" /> Demo network</span>
            <span className="avatar" aria-label="डेमो उपयोगकर्ता">D</span>
          </div>
        </header>

        <main className="page-wrap">
          <div className="demo-notice" role="status">
            <ShieldAlert size={19} />
            <span><strong>केवल डेमो</strong> — यहाँ असली पैसा, निजी कुंजी या ब्लॉकचेन नेटवर्क नहीं है।</span>
          </div>

          {tab === "wallet" && (
            <>
              <div className="page-heading">
                <div>
                  <p className="eyebrow">आपका sandbox</p>
                  <h1>नमस्ते, डेमो उपयोगकर्ता</h1>
                  <p className="subheading">BUT के Hot और Cold मोड को सुरक्षित डेमो में समझें।</p>
                </div>
                <button className="text-button" onClick={resetDemo} type="button">डेमो रीसेट</button>
              </div>

              <section className="dashboard-grid" aria-label="डेमो वॉलेट सारांश">
                <div className="balance-card">
                  <div className="balance-card-top">
                    <div className="balance-card-label"><Wallet size={17} /> कुल डेमो बैलेंस</div>
                    <span className="demo-chip">TEST DATA</span>
                  </div>
                  <div className="balance-value-row">
                    <div className="balance-value">
                      {balanceVisible ? formatBut(balance) : "••••••"}
                      <span>BUT</span>
                    </div>
                    <button
                      className="icon-button balance-eye"
                      onClick={() => setBalanceVisible((visible) => !visible)}
                      aria-label={balanceVisible ? "बैलेंस छिपाएँ" : "बैलेंस दिखाएँ"}
                      type="button"
                    >
                      {balanceVisible ? <Eye size={19} /> : <EyeOff size={19} />}
                    </button>
                  </div>
                  <p className="balance-subtext">यह संख्या सिर्फ आपके ब्राउज़र में रखा नकली डेटा है।</p>
                  <div className="balance-actions">
                    <button className="action-button action-light" onClick={() => openDialog("send")} type="button">
                      <ArrowUpRight size={17} /> डेमो भेजें
                    </button>
                    <button className="action-button action-outline" onClick={() => openDialog("receive")} type="button">
                      <ArrowDownLeft size={17} /> डेमो प्राप्त करें
                    </button>
                  </div>
                  <div className="balance-decoration" aria-hidden="true"><i /><i /><i /></div>
                </div>

                <div className="network-card">
                  <div className="card-heading-row">
                    <div>
                      <p className="eyebrow">नेटवर्क स्थिति</p>
                      <h2>सिर्फ उदाहरण</h2>
                    </div>
                    <div className="network-icon"><Blocks size={20} /></div>
                  </div>
                  <div className="network-metrics">
                    <div><span>डेमो ब्लॉक</span><strong>#00128</strong></div>
                    <div><span>डेमो लेजर</span><strong>{transactions.length} प्रविष्टियाँ</strong></div>
                  </div>
                  <div className="network-foot"><span className="status-dot" /> कोई लाइव chain data नहीं</div>
                  <button className="inline-link" onClick={() => setTab("storage")} type="button">
                    स्टोरेज कैसे घटाएँ? <ChevronRight size={16} />
                  </button>
                </div>
              </section>

              <section className="mode-section">
                <div className="section-heading">
                  <div>
                    <p className="eyebrow">लेन-देन प्रदर्शन</p>
                    <h2>मोड चुनें</h2>
                  </div>
                  <span className="helper-pill"><Zap size={14} /> मोड कभी भी बदलें</span>
                </div>
                <div className="mode-grid">
                  <button
                    className={`mode-card ${mode === "hot" ? "selected" : ""}`}
                    onClick={() => setMode("hot")}
                    aria-pressed={mode === "hot"}
                    type="button"
                  >
                    <span className="mode-icon hot-icon"><Zap size={19} /></span>
                    <span className="mode-copy"><strong>Hot Mode</strong><span>डेमो विवरण दिखेंगे</span></span>
                    <span className={`radio-dot ${mode === "hot" ? "checked" : ""}`} />
                  </button>
                  <button
                    className={`mode-card ${mode === "cold" ? "selected cold-selected" : ""}`}
                    onClick={() => setMode("cold")}
                    aria-pressed={mode === "cold"}
                    type="button"
                  >
                    <span className="mode-icon cold-icon"><LockKeyhole size={19} /></span>
                    <span className="mode-copy"><strong>Cold Mode</strong><span>स्क्रीन पर विवरण छिपेंगे</span></span>
                    <span className={`radio-dot ${mode === "cold" ? "checked" : ""}`} />
                  </button>
                </div>
                {mode === "cold" ? (
                  <div className="privacy-callout cold-callout">
                    <ShieldAlert size={19} />
                    <p><strong>सिर्फ इंटरफ़ेस छिपाव।</strong> रकम और पता स्क्रीन पर ढके जाते हैं; यह क्रिप्टोग्राफ़िक सुरक्षा नहीं है। असली गोपनीयता के लिए प्रोटोकॉल-स्तर का काम और स्वतंत्र ऑडिट चाहिए।</p>
                  </div>
                ) : (
                  <div className="privacy-callout hot-callout">
                    <ShieldCheck size={19} />
                    <p><strong>Hot Mode — सामान्य डेमो।</strong> टेस्ट लेजर में नकली रकम और नकली पते दिखाई देंगे। इनमें से कोई भी डेटा असली नेटवर्क से नहीं आया।</p>
                  </div>
                )}
              </section>

              <section className="activity-panel">
                <div className="section-heading activity-heading">
                  <div>
                    <p className="eyebrow">हाल की गतिविधि</p>
                    <h2>डेमो ट्रांसफर</h2>
                  </div>
                  <button className="inline-link" onClick={() => setTab("ledger")} type="button">
                    पूरा लेजर <ChevronRight size={16} />
                  </button>
                </div>
                <TransactionList
                  transactions={transactions.slice(0, 4)}
                  mode={mode}
                  compact
                />
              </section>
            </>
          )}

          {tab === "ledger" && (
            <section className="ledger-page">
              <div className="page-heading">
                <div>
                  <p className="eyebrow">स्थानीय नकली डेटा</p>
                  <h1>टेस्ट लेजर</h1>
                  <p className="subheading">यह सूची सिर्फ इस ब्राउज़र में चलने वाले डेमो ट्रांसफर दिखाती है।</p>
                </div>
                <button className="action-button action-primary" onClick={() => openDialog("send")} type="button">
                  <ArrowUpRight size={17} /> नया डेमो ट्रांसफर
                </button>
              </div>
              <div className="ledger-toolbar">
                <div><span className="status-dot" /> {transactions.length} नकली प्रविष्टियाँ</div>
                <span className="helper-pill">दृश्य मोड: {mode === "hot" ? "Hot" : "Cold"}</span>
              </div>
              <div className="activity-panel ledger-panel">
                <TransactionList transactions={transactions} mode={mode} />
              </div>
              <div className="privacy-callout hot-callout ledger-disclaimer">
                <ShieldAlert size={19} />
                <p><strong>ध्यान दें:</strong> Cold Mode में विवरण सिर्फ स्क्रीन पर छिपते हैं। डेमो लेजर आपके ब्राउज़र के local storage में रहता है; इसे संवेदनशील या निजी डेटा न समझें।</p>
              </div>
            </section>
          )}

          {tab === "storage" && <StorageExplainer />}
        </main>
      </div>

      {dialog && (
        <div className="modal-scrim" onMouseDown={(event) => event.target === event.currentTarget && closeDialog()}>
          <section className="demo-modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
            <div className="modal-header">
              <div>
                <p className="eyebrow">नकली लेन-देन</p>
                <h2 id="modal-title">{dialog === "send" ? "BUT डेमो भेजें" : "BUT डेमो प्राप्त करें"}</h2>
              </div>
              <button className="icon-button modal-close" onClick={closeDialog} aria-label="बंद करें" type="button"><X size={20} /></button>
            </div>
            <div className="modal-demo-alert"><ShieldAlert size={17} /> कोई असली कॉइन या भुगतान नहीं होगा।</div>
            <form onSubmit={submitDemoTransaction}>
              {dialog === "send" && (
                <label className="form-field">
                  <span>डेमो प्राप्तकर्ता का पता</span>
                  <input
                    autoFocus
                    value={address}
                    onChange={(event) => setAddress(event.target.value)}
                    placeholder="जैसे: but1demo..."
                    autoComplete="off"
                  />
                </label>
              )}
              <label className="form-field">
                <span>{dialog === "send" ? "भेजने की डेमो राशि" : "मिलने की डेमो राशि"}</span>
                <div className="amount-input-wrap">
                  <input
                    autoFocus={dialog === "receive"}
                    value={amount}
                    onChange={(event) => setAmount(event.target.value)}
                    type="number"
                    inputMode="decimal"
                    min="0.01"
                    step="0.01"
                    placeholder="0.00"
                  />
                  <span>BUT</span>
                </div>
              </label>
              {dialog === "receive" && (
                <div className="receive-address">
                  <span>सिर्फ डेमो के लिए पता</span>
                  <strong>{DEMO_RECEIVE_ADDRESS}</strong>
                  <button className="copy-button" onClick={copyDemoAddress} type="button">
                    {copied ? <Check size={15} /> : <Copy size={15} />}
                    {copied ? "कॉपी हो गया" : "डेमो पता कॉपी करें"}
                  </button>
                </div>
              )}
              {mode === "cold" && (
                <div className="modal-cold-note"><LockKeyhole size={16} /> Cold Mode में लेजर की जानकारी केवल UI पर ढकी जाएगी।</div>
              )}
              {formError && <p className="form-error" role="alert">{formError}</p>}
              <div className="modal-actions">
                <button className="action-button action-secondary" onClick={closeDialog} type="button">रद्द करें</button>
                <button className="action-button action-primary" type="submit">
                  {dialog === "send" ? "डेमो भेजें" : "डेमो जोड़ें"}
                </button>
              </div>
            </form>
          </section>
        </div>
      )}
    </div>
  );
}

function TransactionList({
  transactions,
  mode,
  compact = false,
}: {
  transactions: Transaction[];
  mode: Mode;
  compact?: boolean;
}) {
  if (transactions.length === 0) {
    return <div className="empty-ledger">अभी कोई डेमो ट्रांसफर नहीं है। ऊपर से एक नकली प्रविष्टि जोड़ें।</div>;
  }

  return (
    <div className={`transaction-list ${compact ? "compact" : ""}`}>
      {transactions.map((transaction) => {
        const isMasked = mode === "cold" || transaction.mode === "cold";
        const isSent = transaction.direction === "sent";
        return (
          <article className="transaction-row" key={transaction.id}>
            <div className={`transaction-icon ${isSent ? "sent" : "received"}`}>
              {isSent ? <ArrowUpRight size={18} /> : <ArrowDownLeft size={18} />}
            </div>
            <div className="transaction-main">
              <div className="transaction-name">
                <strong>{isMasked ? "एक डेमो ट्रांसफर" : isSent ? "आपने भेजा" : "आपको मिला"}</strong>
                {transaction.mode === "cold" && <span className="cold-mini-badge"><LockKeyhole size={11} /> Cold UI</span>}
              </div>
              <span className="transaction-address">{isMasked ? maskAddress(transaction.address) : transaction.address}</span>
              {!compact && <span className="transaction-hash">ID: {transaction.hash}</span>}
            </div>
            <div className="transaction-side">
              <strong className={`transaction-amount ${isSent ? "negative" : "positive"}`}>
                {isMasked ? "•••• BUT" : `${isSent ? "−" : "+"}${formatBut(transaction.amount)} BUT`}
              </strong>
              <span>{transaction.createdAt}</span>
              <span className="status-label"><span className="status-dot" />{transaction.status}</span>
            </div>
          </article>
        );
      })}
    </div>
  );
}

function StorageExplainer() {
  return (
    <section className="storage-page">
      <div className="page-heading">
        <div>
          <p className="eyebrow">नेटवर्क बढ़े तो क्या करें?</p>
          <h1>ब्लॉक का आकार घट सकता है?</h1>
          <p className="subheading">हाँ—लेकिन कंप्रेस करना, किसी नोड से डेटा हटाना और इतिहास को पूरी तरह मिटाना अलग-अलग बातें हैं।</p>
        </div>
        <span className="storage-page-icon"><HardDrive size={24} /></span>
      </div>

      <div className="storage-demo-banner">
        <div className="storage-visual" aria-hidden="true">
          <span className="storage-block block-one" />
          <span className="storage-block block-two" />
          <span className="storage-block block-three" />
          <ChevronRight size={19} />
          <span className="storage-stack"><i /><i /><i /></span>
        </div>
        <div>
          <p className="eyebrow">सही सवाल</p>
          <h2>क्या हर नोड को पूरा इतिहास रखना चाहिए?</h2>
          <p>नहीं। कुछ नोड पूरा archive रखते हैं; कुछ पुराने डेटा को स्थानीय रूप से prune करते हैं। नेटवर्क की उपयोगिता इस पर निर्भर करती है कि ज़रूरी इतिहास की प्रतियाँ कहीं उपलब्ध रहें।</p>
        </div>
      </div>

      <div className="method-grid">
        <article className="method-card">
          <div className="method-number">01</div>
          <div className="method-icon"><Blocks size={20} /></div>
          <h3>Lossless compression</h3>
          <p>डेटा को कम जगह में पैक करना, बिना जानकारी खोए। इसे वापस खोलने पर मूल bytes मिलते हैं। इतिहास फिर भी मौजूद रहता है—बस उसका संग्रह छोटा हो सकता है।</p>
          <span className="method-tag">इतिहास सुरक्षित</span>
        </article>
        <article className="method-card featured-method">
          <div className="method-number">02</div>
          <div className="method-icon"><HardDrive size={20} /></div>
          <h3>Node pruning</h3>
          <p>नोड पुराने ब्लॉक डेटा को सत्यापित करने के बाद अपनी स्थानीय कॉपी से हटा सकता है, और आगे चलने के लिए ज़रूरी हाल का डेटा/state रख सकता है। दूसरे archive nodes पुराने blocks उपलब्ध रखते हैं।</p>
          <span className="method-tag">हर नोड का अपना चुनाव</span>
        </article>
        <article className="method-card">
          <div className="method-number">03</div>
          <div className="method-icon"><ShieldCheck size={20} /></div>
          <h3>State snapshot</h3>
          <p>किसी ऊँचाई पर नेटवर्क की वर्तमान स्थिति का प्रमाणित snapshot नए नोड को तेज़ी से sync करा सकता है। यह पुराने हर लेन-देन का पूरा डेटा अपने-आप उपलब्ध नहीं कराता।</p>
          <span className="method-tag">तेज़ शुरुआत, भरोसे का आधार ज़रूरी</span>
        </article>
      </div>

      <section className="integrity-panel">
        <div className="integrity-icon"><ShieldAlert size={21} /></div>
        <div>
          <h2>क्या पुराने ब्लॉक सबके लिए मिटा सकते हैं?</h2>
          <p>मनमाने ढंग से पूरे नेटवर्क का इतिहास मिटाना सही समाधान नहीं है। अगर सभी नोड पुराना डेटा हटा दें, तो पुराने लेन-देन डाउनलोड या स्वतंत्र रूप से जाँचना कठिन हो जाएगा। ब्लॉक-हैश और state commitments बदलाव पकड़ने में मदद करते हैं, लेकिन hash अकेले हटाए गए transaction bytes वापस नहीं ला सकता।</p>
          <p className="integrity-recommendation"><strong>जिम्मेदार रास्ता:</strong> archive nodes इतिहास उपलब्ध रखें; pruning की सुविधा नोड चलाने वाले के चुनाव पर हो; snapshots के साथ साफ़ verification/trust नियम हों।</p>
        </div>
      </section>

      <div className="storage-demo-footnote">
        <span className="status-dot" /> यह शैक्षिक व्याख्या है—BUT का कोई वास्तविक नेटवर्क, block size या live storage आँकड़ा नहीं।
      </div>
    </section>
  );
}
