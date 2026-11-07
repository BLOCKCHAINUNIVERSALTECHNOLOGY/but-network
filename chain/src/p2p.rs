//! P2P network — Phase 2 (sync + tx gossip), Phase 3a (vote gossip).
//!
//! Kya kar sakta hai:
//! - Ek node doosre node se apni chain ki height aur latest hash poochh sakta hai.
//! - Node doosre node se missing blocks maang kar apni chain sync kar sakta hai
//!   (har block `Chain::add_block_bft` se poori tarah verify hokar hi judta
//!   hai — votes block ke andar embedded hote hain, isliye sync karne wala
//!   node bhi quorum check kar sakta hai).
//! - Pending transactions ek node se doosre node tak bhej sakta hai
//!   (mempool me jaane se pehle signature verify hoti hai).
//! - BFT votes gossip hote hain: validator apna vote peers ko bhejta hai,
//!   peer vote verify karke apni tally me rakhta hai, aur proposer votes
//!   maang kar quorum bana sakta hai.
//!
//! KYUN PLAIN TCP (libp2p kyun nahi)?
//! - **Std-only, koi bhaari dependency nahi**: libp2p ya axum jaise bade
//!   crates laane se build time, binary size aur attack surface — teenon
//!   badhte. Is waqt network chhota hai (kuch nodes, localhost/testnet),
//!   to plain TCP + JSON kaafi hai aur har line samajh me aati hai.
//! - **Debuggable**: wire par sirf JSON lines hain — `nc`/`telnet` se bhi
//!   node se baat ki ja sakti hai, koi binary protocol decode nahi karna.
//! - **Protocol aage bhi chalega**: messages JSON hain, isliye aage libp2p
//!   ya QUIC par bhi yahi message format chal sakta hai — transport
//!   badlega, protocol nahi.
//! - **Limit pata hai**: plain TCP me encryption nahi hai, peer discovery
//!   nahi hai (peers haath se dene padte hain), aur NAT/firewall ke peeche
//!   ke nodes nahi jud sakte. Production P2P (encrypted transport, peer
//!   discovery, NAT traversal) ke liye libp2p jaisa stack Phase 3+ me
//!   chahiye hoga — ye yahan imaandari se likha hai.
//!
//! SACH BATA RAHE HAIN — kya abhi bhi simplified hai:
//! - **No fork choice / reorg**: sync sirf apni chain ko AAGE badhata hai.
//!   Peer ki chain alag nikli (diverged) to `add_block_bft` prev-hash
//!   mismatch par reject kar dega — node apni purani chain par hi rahega,
//!   lambi chain ko apnana (reorg) abhi nahi hota.
//! - **No encryption / authentication**: koi bhi TCP client node se jud
//!   sakta hai. Blocks aur transactions khud verify hote hain (signature +
//!   hash), isliye galat data chain me nahi ghus sakta — par node ko spam
//!   requests se pareshan kiya ja sakta hai (rate limiting abhi nahi hai).
//! - **Vote gossip hai, par basic**: votes verify hokar tally me jate hain
//!   aur maange ja sakte hain. Equivocation (ek hi height par do alag
//!   blocks ko vote) par ab block-level SlashOp banta hai —
//!   `Node::report_equivocation` se (Phase 3b). Downtime par slash abhi nahi.
//! - **Registry sync hai, par basic**: BUT Pay registry peers se sync hoti
//!   hai (merge, conflict par reject — Phase 3b). Registry entries khud
//!   on-chain nahi hain (chain me sirf transactions hain) — isliye do
//!   nodes ki registry alag ho sakti hai; merge conflict ka hal manual hai.
//! - **Request-response, ek connection = ek request**: har request ke liye
//!   naya TCP connection khulta hai. Persistent peer connections abhi nahi hain.
//!
//! Wire format: har message ek JSON line hai (`\n` se khatm). Request ke
//! jawab me hamesha ek JSON line milti hai.

use crate::block::Block;
use crate::butpay::ButPayRegistry;
use crate::chain::Chain;
use crate::consensus::{Vote, VoteTally};
use crate::error::{ButError, Result};
use crate::mempool::Mempool;
use crate::slash::SlashOp;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Ek request par zyada se zyada kitne blocks bheje jayenge.
pub const MAX_BLOCKS_PER_RESPONSE: usize = 100;

/// Network timeout — peer jawab na de to kitni der wait karein.
pub const P2P_TIMEOUT: Duration = Duration::from_secs(5);

/// Nodes ke beech jaane wale messages (JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2pMessage {
    /// "Main kaun hoon, meri chain kitni badi hai" — handshake.
    Hello { height: usize, latest_hash: String },
    /// Hello ka jawab — apni height aur latest hash.
    Height { height: usize, latest_hash: String },
    /// `from_index` se aage ke blocks maango.
    GetBlocks { from_index: u64 },
    /// Maange hue blocks (verify karke hi chain me judenge).
    Blocks { blocks: Vec<Block> },
    /// Pending transaction doosre node ko bhejo.
    AnnounceTx { tx: Transaction },
    /// BFT vote doosre node ko bhejo — peer verify karke apni tally me rakhega.
    AnnounceVote { vote: Vote },
    /// Kisi height ke votes maango.
    GetVotes { height: u64 },
    /// Maange hue votes.
    Votes { votes: Vec<Vote> },
    /// Peer ki BUT Pay registry maango (ID/handle → address).
    GetRegistry,
    /// Peer ki poori BUT Pay registry (merge karke sync hogi).
    Registry { registry: ButPayRegistry },
    /// Kisi request ka seedha jawab — kaam hua ya nahi.
    Ack { ok: bool, message: String },
}

/// Ek BUT node — chain + mempool + peers + vote tallies + BUT Pay registry,
/// threads ke beech shareable.
#[derive(Debug, Clone)]
pub struct Node {
    /// Is node ki chain (server thread aur API thread dono use karte hain).
    pub chain: Arc<Mutex<Chain>>,
    /// Pending transactions.
    pub mempool: Arc<Mutex<Mempool>>,
    /// Jaane hue peers — "127.0.0.1:9001" jaise addresses.
    pub peers: Arc<Mutex<Vec<String>>>,
    /// Height ke hisaab se vote tallies — gossip se aaye votes yahan jama hote hain.
    pub tallies: Arc<Mutex<HashMap<u64, VoteTally>>>,
    /// Is node ki BUT Pay registry (ID/handle → address) — peers se sync
    /// hoti hai (Phase 3b). ID wale transactions wale blocks isi registry
    /// se verify hote hain.
    pub registry: Arc<Mutex<ButPayRegistry>>,
}

impl Node {
    /// Chain se naya node banao (registry khaali shuru hoti hai).
    pub fn new(chain: Chain) -> Self {
        Self::with_registry(chain, ButPayRegistry::new())
    }

    /// Chain + registry se naya node banao (pehle se bhari registry ke saath).
    pub fn with_registry(chain: Chain, registry: ButPayRegistry) -> Self {
        Self {
            chain: Arc::new(Mutex::new(chain)),
            mempool: Arc::new(Mutex::new(Mempool::new())),
            peers: Arc::new(Mutex::new(Vec::new())),
            tallies: Arc::new(Mutex::new(HashMap::new())),
            registry: Arc::new(Mutex::new(registry)),
        }
    }

    /// Peer jodo (duplicate nahi jodega).
    pub fn add_peer(&self, addr: &str) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.iter().any(|p| p == addr) {
            peers.push(addr.to_string());
        }
    }

    /// Apni chain ki height aur latest hash.
    pub fn height_and_hash(&self) -> (usize, String) {
        let chain = self.chain.lock().unwrap();
        (chain.height(), chain.latest_hash())
    }

    /// Ek connection handle karo: ek request line padho, ek response line likho.
    pub fn handle_connection(&self, stream: TcpStream) -> Result<()> {
        let _ = stream.set_read_timeout(Some(P2P_TIMEOUT));
        let _ = stream.set_write_timeout(Some(P2P_TIMEOUT));
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
            ButError::Network(format!("connection clone nahi hua: {e}"))
        })?);
        let mut writer = stream;
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| {
            ButError::Network(format!("request padha nahi gaya: {e}"))
        })?;
        if n == 0 {
            return Ok(()); // client ne kuch bheje bina connection band kar diya
        }
        let request: P2pMessage = serde_json::from_str(line.trim()).map_err(|e| {
            ButError::Network(format!("request JSON samajh nahi aaya: {e}"))
        })?;
        let response = self.handle_message(request);
        send_message(&mut writer, &response)
    }

    /// Ek message ka jawab banao (network ke bina bhi test kar sakte hain).
    fn handle_message(&self, msg: P2pMessage) -> P2pMessage {
        match msg {
            P2pMessage::Hello { .. } => {
                let (height, latest_hash) = self.height_and_hash();
                P2pMessage::Height {
                    height,
                    latest_hash,
                }
            }
            P2pMessage::GetBlocks { from_index } => {
                let chain = self.chain.lock().unwrap();
                let from = from_index as usize;
                let blocks = if from >= chain.blocks.len() {
                    Vec::new()
                } else {
                    let end = (from + MAX_BLOCKS_PER_RESPONSE).min(chain.blocks.len());
                    chain.blocks[from..end].to_vec()
                };
                P2pMessage::Blocks { blocks }
            }
            P2pMessage::AnnounceTx { tx } => {
                let mut pool = self.mempool.lock().unwrap();
                match pool.add(tx) {
                    Ok(()) => P2pMessage::Ack {
                        ok: true,
                        message: "transaction mempool me aa gayi".to_string(),
                    },
                    Err(e) => P2pMessage::Ack {
                        ok: false,
                        message: format!("transaction reject: {e}"),
                    },
                }
            }
            P2pMessage::AnnounceVote { vote } => {
                // Vote verify karo (signature + voter active validator hai),
                // phir us height ki tally me rakho. Galat vote reject.
                match self.record_vote(vote) {
                    Ok(()) => P2pMessage::Ack {
                        ok: true,
                        message: "vote tally me aa gaya".to_string(),
                    },
                    Err(e) => P2pMessage::Ack {
                        ok: false,
                        message: format!("vote reject: {e}"),
                    },
                }
            }
            P2pMessage::GetVotes { height } => {
                let votes = self
                    .tallies
                    .lock()
                    .unwrap()
                    .get(&height)
                    .map(|t| t.votes.clone())
                    .unwrap_or_default();
                P2pMessage::Votes { votes }
            }
            P2pMessage::Votes { .. } => P2pMessage::Ack {
                ok: false,
                message: "Votes ek jawab hai, request nahi".to_string(),
            },
            P2pMessage::GetRegistry => {
                let registry = self.registry.lock().unwrap().clone();
                P2pMessage::Registry { registry }
            }
            P2pMessage::Registry { .. } => P2pMessage::Ack {
                ok: false,
                message: "Registry ek jawab hai, request nahi".to_string(),
            },
            other => P2pMessage::Ack {
                ok: false,
                message: format!("ye message request nahi hai: {other:?}"),
            },
        }
    }

    /// Ek vote verify karke is node ki tally me rakho.
    /// Equivocation (ek hi height par do alag blocks ko vote) yahan log
    /// hoti hai — vote ko reject nahi karte (conflicting vote hi saboot
    /// hai). Slash ke liye `report_equivocation` chalao — wo tally ke
    /// saboot se signed SlashOp banakar mempool me dalta hai (Phase 3b).
    pub fn record_vote(&self, vote: Vote) -> Result<()> {
        vote.verify()?;
        let validators = {
            let chain = self.chain.lock().unwrap();
            chain.validators.clone()
        };
        let mut tallies = self.tallies.lock().unwrap();
        let tally = tallies
            .entry(vote.height)
            .or_insert_with(|| VoteTally::new(vote.height));
        if tally.add_vote(vote.clone(), &validators).is_err() {
            // Duplicate ya non-validator vote — tally nahi badli.
            return Err(ButError::InvalidVote(
                "vote tally me nahi joda ja saka (duplicate ya voter validator nahi hai)"
                    .to_string(),
            ));
        }
        let bad = tally.equivocators();
        if !bad.is_empty() {
            eprintln!(
                "WARNING: equivocation dekha gaya height {} par: {:?} — slash ke liye report_equivocation chalao",
                vote.height, bad
            );
        }
        Ok(())
    }

    /// Kisi height ki tally me equivocation ho to us par slash report karo.
    /// Har equivocator ke liye ek signed SlashOp banata hai (reporter ki
    /// key se, uske current nonce ke saath) aur mempool me dalta hai —
    /// agla block banega to slash op usme jayega.
    /// Returns: banaye gaye slash ops (koi equivocation na ho to khaali).
    /// Note: reporter ka nonce chain se liya jata hai — reporter ne beech
    /// me koi tx ki ho to nonce badal jayega; op tab bhi banega, par block
    /// apply karte waqt nonce galat hua to reject hoga (dobara chalao).
    pub fn report_equivocation(&self, reporter: &Wallet, height: u64) -> Result<Vec<SlashOp>> {
        let tally = {
            let tallies = self.tallies.lock().unwrap();
            match tallies.get(&height) {
                Some(t) => t.clone(),
                None => return Ok(Vec::new()),
            }
        };
        let equivocators = tally.equivocators();
        if equivocators.is_empty() {
            return Ok(Vec::new());
        }
        let reporter_nonce = {
            let chain = self.chain.lock().unwrap();
            chain.nonces.get(&reporter.address).copied().unwrap_or(0)
        };
        let mut ops = Vec::new();
        let mut nonce = reporter_nonce;
        for target in equivocators {
            // Is target ke do conflicting votes (alag block hash) nikalo.
            let conflicting: Vec<Vote> = tally
                .votes
                .iter()
                .filter(|v| v.voter == target && v.approve)
                .cloned()
                .collect();
            let mut pair: Option<(Vote, Vote)> = None;
            'outer: for i in 0..conflicting.len() {
                for j in (i + 1)..conflicting.len() {
                    if conflicting[i].block_hash != conflicting[j].block_hash {
                        pair = Some((conflicting[i].clone(), conflicting[j].clone()));
                        break 'outer;
                    }
                }
            }
            let (vote_a, vote_b) = match pair {
                Some(p) => p,
                None => continue, // saboot adhura hai — is target ko chhodo
            };
            let mut op = SlashOp::new(
                target.clone(),
                vote_a,
                vote_b,
                format!("equivocation height {height} par do alag blocks ko vote"),
                reporter.address.clone(),
                nonce,
            );
            op.sign(&reporter.signing_key())?;
            {
                let mut pool = self.mempool.lock().unwrap();
                pool.add_slash_op(op.clone())?;
            }
            nonce += 1;
            ops.push(op);
        }
        Ok(ops)
    }

    /// Is node ke validator wallet se block par vote banao, apni tally me
    /// rakho aur saare peers ko gossip karo. Vote tabhi banta hai jab
    /// wallet active validator ho.
    pub fn vote_for_block(&self, block: &Block, wallet: &Wallet) -> Result<Vote> {
        {
            let chain = self.chain.lock().unwrap();
            chain.validators.authorize_block(&wallet.address)?;
        }
        let vote = Vote::new(
            block.hash.clone(),
            block.index,
            true,
            wallet.address.clone(),
            &wallet.signing_key(),
        )?;
        self.record_vote(vote.clone())?;
        self.gossip_vote_to_peers(&vote);
        Ok(vote)
    }

    /// Vote saare peers ko bhejo. Koi peer band ho ya reject kare to sirf
    /// log karo — gossip best-effort hai, ek peer ki galti se vote nahi rukta.
    pub fn gossip_vote_to_peers(&self, vote: &Vote) {
        let peers = self.peers.lock().unwrap().clone();
        for peer in peers {
            if let Err(e) = Self::announce_vote(&peer, vote) {
                eprintln!("peer {peer} ko vote gossip nahi hua: {e}");
            }
        }
    }

    /// Kisi height ke votes jama karo: pehle apni tally, phir har peer se
    /// GetVotes. Voter ke hisaab se duplicate hatate hain. Returns: votes.
    /// Quorum check caller karega (`VoteTally::has_quorum_for`).
    pub fn collect_votes_for_block(&self, height: u64) -> Vec<Vote> {
        let mut out: Vec<Vote> = self
            .tallies
            .lock()
            .unwrap()
            .get(&height)
            .map(|t| t.votes.clone())
            .unwrap_or_default();
        let peers = self.peers.lock().unwrap().clone();
        for peer in peers {
            match Self::request_votes(&peer, height) {
                Ok(votes) => {
                    for v in votes {
                        if !out.iter().any(|o| o.voter == v.voter) {
                            out.push(v);
                        }
                    }
                }
                Err(e) => eprintln!("peer {peer} se votes nahi mile: {e}"),
            }
        }
        out
    }

    /// Listener par `n` connections handle karke band ho jao.
    /// Tests ke liye hai — CLI me `serve_forever` chalta hai.
    pub fn serve_n(&self, listener: TcpListener, n: usize) -> Result<()> {
        for stream in listener.incoming().take(n) {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_connection(stream) {
                        eprintln!("p2p connection error: {e}");
                    }
                }
                Err(e) => {
                    return Err(ButError::Network(format!(
                        "connection accept nahi hui: {e}"
                    )))
                }
            }
        }
        Ok(())
    }

    /// Hamesha chalo — har connection ke liye naya thread.
    pub fn serve_forever(&self, listener: TcpListener) -> Result<()> {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let node = self.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = node.handle_connection(stream) {
                            eprintln!("p2p connection error: {e}");
                        }
                    });
                }
                Err(e) => {
                    return Err(ButError::Network(format!(
                        "connection accept nahi hui: {e}"
                    )))
                }
            }
        }
        Ok(())
    }

    /// Peer se height aur latest hash poochho.
    pub fn request_height(peer: &str) -> Result<(usize, String)> {
        let stream = connect(peer)?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
            ButError::Network(format!("connection clone nahi hua: {e}"))
        })?);
        let mut writer = stream;
        send_message(
            &mut writer,
            &P2pMessage::Hello {
                height: 0,
                latest_hash: String::new(),
            },
        )?;
        match read_message(&mut reader)? {
            P2pMessage::Height { height, latest_hash } => Ok((height, latest_hash)),
            other => Err(ButError::Network(format!(
                "peer se Height jawab chahiye tha, mila: {other:?}"
            ))),
        }
    }

    /// Peer se BUT Pay registry sync karo (Phase 3b).
    /// Peer ki registry apni registry me merge hoti hai (immutable-merge
    /// rules — conflict ho to poora merge reject, kuch nahi milta).
    /// Returns: kitni nayi entries judi.
    /// Best-effort hai — peer band ho ya registry na mile to error; caller
    /// (sync_from_peer) us error ko sirf log karta hai, sync nahi rokta.
    pub fn sync_registry_from_peer(&self, peer: &str) -> Result<usize> {
        let peer_registry = Self::request_registry(peer)?;
        let mut registry = self.registry.lock().unwrap();
        registry.merge(&peer_registry)
    }

    /// Peer se uski poori BUT Pay registry maango.
    pub fn request_registry(peer: &str) -> Result<ButPayRegistry> {
        let stream = connect(peer)?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
            ButError::Network(format!("connection clone nahi hua: {e}"))
        })?);
        let mut writer = stream;
        send_message(&mut writer, &P2pMessage::GetRegistry)?;
        match read_message(&mut reader)? {
            P2pMessage::Registry { registry } => Ok(registry),
            other => Err(ButError::Network(format!(
                "peer se Registry jawab chahiye tha, mila: {other:?}"
            ))),
        }
    }

    /// Peer se chain sync karo: **pehle registry, phir blocks**.
    /// Registry pehle sync hoti hai taaki ID/handle wale transactions
    /// wale blocks bhi verify hokar jud sakein (Phase 3b).
    /// Registry sync best-effort hai — peer ki registry na mile ya merge
    /// conflict ho to sirf warning log hogi, block sync phir bhi chalega
    /// (ID wale blocks tab fail ho sakte hain, registry baad me sync hogi).
    /// Returns: kitne blocks jode gaye.
    /// Peer ki chain chhoti ho ya barabar ho to 0 return karta hai.
    /// Peer ki chain alag nikli (diverged) to error — apni chain nahi badalti
    /// (jo blocks pehle se verify hokar jud gaye, wo rehte hain; dobara
    /// sync karke aage badha ja sakta hai).
    pub fn sync_from_peer(&self, peer: &str) -> Result<usize> {
        let (peer_height, _peer_hash) = Self::request_height(peer)?;
        let own_height = self.height_and_hash().0;
        if peer_height <= own_height {
            return Ok(0);
        }
        // Step 1: registry sync (best-effort).
        match self.sync_registry_from_peer(peer) {
            Ok(added) => {
                if added > 0 {
                    eprintln!("peer {peer} se {added} BUT Pay entries sync hui");
                }
            }
            Err(e) => {
                eprintln!(
                    "WARNING: peer {peer} se registry sync nahi hui: {e} — block sync phir bhi chalega"
                );
            }
        }
        let blocks = Self::request_blocks(peer, own_height as u64)?;
        let mut added = 0usize;
        for block in blocks {
            {
                let mut chain = self.chain.lock().unwrap();
                // Block ke andar embedded votes se tally banao — bina votes
                // wala block sync nahi hoga (BFT-only, sahi hai).
                // Registry node ki apni (sync hui) registry hai — ID wale
                // txs wale blocks bhi sync honge (Phase 3b).
                let registry = self.registry.lock().unwrap().clone();
                let mut tally = VoteTally::new(block.index);
                for vote in &block.votes {
                    tally.add_vote(vote.clone(), &chain.validators)?;
                }
                chain.add_block_bft_with_registry(block, &tally, &registry)?;
            }
            added += 1;
        }
        // Peer ne poori chain nahi bheji to saaf batao.
        let now_height = self.height_and_hash().0;
        if now_height < peer_height {
            return Err(ButError::Network(format!(
                "peer ne poori chain nahi bheji: peer height {peer_height}, apni height {now_height} ({} blocks jode gaye, dobara sync karo)",
                added
            )));
        }
        Ok(added)
    }

    /// Peer se `from_index` se aage ke blocks maango.
    fn request_blocks(peer: &str, from_index: u64) -> Result<Vec<Block>> {
        let stream = connect(peer)?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
            ButError::Network(format!("connection clone nahi hua: {e}"))
        })?);
        let mut writer = stream;
        send_message(&mut writer, &P2pMessage::GetBlocks { from_index })?;
        match read_message(&mut reader)? {
            P2pMessage::Blocks { blocks } => Ok(blocks),
            other => Err(ButError::Network(format!(
                "peer se Blocks jawab chahiye tha, mila: {other:?}"
            ))),
        }
    }

    /// Peer ko ek BFT vote bhejo. Returns: peer ne tally me liya ya nahi.
    pub fn announce_vote(peer: &str, vote: &Vote) -> Result<bool> {
        let stream = connect(peer)?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
            ButError::Network(format!("connection clone nahi hua: {e}"))
        })?);
        let mut writer = stream;
        send_message(
            &mut writer,
            &P2pMessage::AnnounceVote { vote: vote.clone() },
        )?;
        match read_message(&mut reader)? {
            P2pMessage::Ack { ok, message } => {
                if !ok {
                    eprintln!("peer ne vote reject kiya: {message}");
                }
                Ok(ok)
            }
            other => Err(ButError::Network(format!(
                "peer se Ack jawab chahiye tha, mila: {other:?}"
            ))),
        }
    }

    /// Peer se kisi height ke votes maango.
    pub fn request_votes(peer: &str, height: u64) -> Result<Vec<Vote>> {
        let stream = connect(peer)?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
            ButError::Network(format!("connection clone nahi hua: {e}"))
        })?);
        let mut writer = stream;
        send_message(&mut writer, &P2pMessage::GetVotes { height })?;
        match read_message(&mut reader)? {
            P2pMessage::Votes { votes } => Ok(votes),
            other => Err(ButError::Network(format!(
                "peer se Votes jawab chahiye tha, mila: {other:?}"
            ))),
        }
    }

    /// Peer ko ek transaction bhejo. Returns: peer ne mempool me li ya nahi.
    pub fn announce_tx(peer: &str, tx: &Transaction) -> Result<bool> {
        let stream = connect(peer)?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
            ButError::Network(format!("connection clone nahi hua: {e}"))
        })?);
        let mut writer = stream;
        send_message(&mut writer, &P2pMessage::AnnounceTx { tx: tx.clone() })?;
        match read_message(&mut reader)? {
            P2pMessage::Ack { ok, message } => {
                if !ok {
                    eprintln!("peer ne transaction reject ki: {message}");
                }
                Ok(ok)
            }
            other => Err(ButError::Network(format!(
                "peer se Ack jawab chahiye tha, mila: {other:?}"
            ))),
        }
    }
}

/// Peer se TCP connection kholo (timeout ke saath).
fn connect(peer: &str) -> Result<TcpStream> {
    let addr = peer
        .to_socket_addrs()
        .map_err(|e| ButError::Network(format!("peer address galat hai '{peer}': {e}")))?
        .next()
        .ok_or_else(|| ButError::Network(format!("peer address resolve nahi hua: {peer}")))?;
    let stream = TcpStream::connect_timeout(&addr, P2P_TIMEOUT).map_err(|e| {
        ButError::Network(format!("peer se connect nahi hua '{peer}': {e}"))
    })?;
    let _ = stream.set_read_timeout(Some(P2P_TIMEOUT));
    let _ = stream.set_write_timeout(Some(P2P_TIMEOUT));
    Ok(stream)
}

/// Ek message JSON line ke roop me bhejo.
fn send_message(writer: &mut TcpStream, msg: &P2pMessage) -> Result<()> {
    let json = serde_json::to_string(msg)
        .map_err(|e| ButError::Network(format!("message JSON nahi bana: {e}")))?;
    writer
        .write_all(json.as_bytes())
        .and_then(|()| writer.write_all(b"\n"))
        .and_then(|()| writer.flush())
        .map_err(|e| ButError::Network(format!("message bheja nahi gaya: {e}")))?;
    Ok(())
}

/// Ek JSON line padho aur message banao.
fn read_message(reader: &mut BufReader<TcpStream>) -> Result<P2pMessage> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| {
        ButError::Network(format!("jawab padha nahi gaya: {e}"))
    })?;
    if line.trim().is_empty() {
        return Err(ButError::Network(
            "peer ne jawab diye bina connection band kar diya".to_string(),
        ));
    }
    serde_json::from_str(line.trim())
        .map_err(|e| ButError::Network(format!("jawab JSON samajh nahi aaya: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;
    use crate::transaction::Transaction;
    use crate::wallet::Wallet;

    fn test_wallet(seed: u8) -> Wallet {
        Wallet::from_entropy(&[seed; 32]).unwrap()
    }

    /// Genesis + `n_blocks` transfer blocks wali chain.
    /// Sab nodes ke liye wahi genesis params — taaki genesis hash ek ho.
    /// Blocks BFT se judte hain (validator ka self-vote, bootstrap quorum).
    fn chain_with_blocks(n_blocks: u64) -> Chain {
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let mut chain = Chain::new_genesis(
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
        )
        .unwrap();
        let mut nonce_alice = 0u64;
        for i in 0..n_blocks {
            let mut tx = Transaction::new(
                alice.address.clone(),
                bob.address.clone(),
                100,
                0,
                nonce_alice,
            );
            tx.sign(&alice.signing_key()).unwrap();
            nonce_alice += 1;
            let height = chain.height() as u64;
            let block = Block::new(
                height,
                1_700_000_001 + i,
                chain.latest_hash(),
                vec![tx],
                validator.address.clone(),
                &validator.signing_key(),
            )
            .unwrap();
            let mut tally = VoteTally::new(height);
            let vote = Vote::new(
                block.hash.clone(),
                height,
                true,
                validator.address.clone(),
                &validator.signing_key(),
            )
            .unwrap();
            tally.add_vote(vote, &chain.validators).unwrap();
            chain.add_block_bft(block, &tally).unwrap();
        }
        chain
    }

    /// Node ko 127.0.0.1:0 par sunao; thread me `n` connections handle karo.
    /// Returns: (peer address "127.0.0.1:<port>", JoinHandle).
    fn serve_test_node(node: Node, n: usize) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let handle = std::thread::spawn(move || {
            node.serve_n(listener, n).unwrap();
        });
        (addr, handle)
    }

    #[test]
    fn do_nodes_sync_karte_hain() {
        // Node A: genesis + 2 blocks. Node B: sirf genesis.
        let node_a = Node::new(chain_with_blocks(2));
        let node_b = Node::new(chain_with_blocks(0));
        assert_eq!(node_b.height_and_hash().0, 1);
        // Sync me 3 connections hote hain: Hello + GetRegistry + GetBlocks.
        let (addr_a, handle) = serve_test_node(node_a, 3);
        let added = node_b.sync_from_peer(&addr_a).unwrap();
        handle.join().unwrap();
        assert_eq!(added, 2);
        assert_eq!(node_b.height_and_hash().0, 3);
        assert_eq!(
            node_b.height_and_hash().1,
            chain_with_blocks(2).latest_hash()
        );
        // Balances bhi sync hue.
        let bob = test_wallet(2);
        let chain_b = node_b.chain.lock().unwrap();
        assert_eq!(chain_b.balance(&bob.address), 200);
        chain_b.verify_chain().unwrap();
    }

    #[test]
    fn sync_pehle_se_barabar_ho_to_zero() {
        let node_a = Node::new(chain_with_blocks(1));
        let node_b = Node::new(chain_with_blocks(1));
        // Sirf Hello connection hogi — peer height barabar hai.
        let (addr_a, handle) = serve_test_node(node_a, 1);
        let added = node_b.sync_from_peer(&addr_a).unwrap();
        handle.join().unwrap();
        assert_eq!(added, 0);
        assert_eq!(node_b.height_and_hash().0, 2);
    }

    #[test]
    fn height_handshake_sahi_hai() {
        let node_a = Node::new(chain_with_blocks(2));
        let (addr_a, handle) = serve_test_node(node_a, 1);
        let (height, hash) = Node::request_height(&addr_a).unwrap();
        handle.join().unwrap();
        assert_eq!(height, 3);
        assert_eq!(hash, chain_with_blocks(2).latest_hash());
    }

    #[test]
    fn transaction_gossip_mempool_tak_pahunchti_hai() {
        let node_a = Node::new(chain_with_blocks(0));
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 50, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        // 2 connections: valid tx + unsigned tx.
        let (addr_a, handle) = serve_test_node(node_a.clone(), 2);
        let ok = Node::announce_tx(&addr_a, &tx).unwrap();
        assert!(ok);
        assert_eq!(node_a.mempool.lock().unwrap().len(), 1);
        // Bina signature wali tx peer reject karega.
        let bad_tx = Transaction::new(alice.address.clone(), bob.address.clone(), 50, 0, 0);
        let ok2 = Node::announce_tx(&addr_a, &bad_tx).unwrap();
        assert!(!ok2);
        assert_eq!(node_a.mempool.lock().unwrap().len(), 1);
        handle.join().unwrap();
    }

    #[test]
    fn alag_genesis_wali_chain_sync_reject() {
        // Node B ki genesis allocations alag hain — uska genesis hash hi alag hai.
        let validator = test_wallet(9);
        let other = test_wallet(3);
        let chain_b = Chain::new_genesis(
            vec![(other.address.clone(), 5_000)],
            &validator,
            1_700_000_000,
        )
        .unwrap();
        let node_a = Node::new(chain_with_blocks(1));
        let node_b = Node::new(chain_b);
        // Sync me 3 connections: Hello + GetRegistry + GetBlocks, par block add fail hoga.
        let (addr_a, handle) = serve_test_node(node_a, 3);
        let result = node_b.sync_from_peer(&addr_a);
        handle.join().unwrap();
        assert!(result.is_err());
        // B ki chain bilkul nahi badli.
        assert_eq!(node_b.height_and_hash().0, 1);
    }

    #[test]
    fn band_peer_se_sync_error_aata_hai() {
        // Koi server nahi sun raha — connect fail hona chahiye, hang nahi.
        let node_b = Node::new(chain_with_blocks(0));
        let result = node_b.sync_from_peer("127.0.0.1:1");
        assert!(result.is_err());
    }

    #[test]
    fn vote_gossip_dusre_node_tak_pahunchta_hai() {
        // Node A aur B dono sirf genesis par hain.
        let node_a = Node::new(chain_with_blocks(0));
        let node_b = Node::new(chain_with_blocks(0));
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        // A block 1 banata hai (abhi add nahi karta) aur us par vote karta hai.
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            1,
            1_700_000_001,
            node_a.height_and_hash().1,
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        let vote = Vote::new(
            block.hash.clone(),
            1,
            true,
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        // B ke paas abhi koi vote nahi hai.
        assert!(node_b
            .tallies
            .lock()
            .unwrap()
            .get(&1)
            .map(|t| t.votes.len())
            .unwrap_or(0)
            == 0);
        // 1 connection: A ka AnnounceVote B tak.
        let (addr_b, handle) = serve_test_node(node_b.clone(), 1);
        let ok = Node::announce_vote(&addr_b, &vote).unwrap();
        handle.join().unwrap();
        assert!(ok);
        // B ki tally me vote aa gaya.
        let tally = node_b.tallies.lock().unwrap();
        let votes = &tally.get(&1).unwrap().votes;
        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].voter, validator.address);
        assert_eq!(votes[0].block_hash, block.hash);
    }

    #[test]
    fn votes_maange_ja_sakte_hain_aur_gossip_se_block_judta_hai() {
        let node_a = Node::new(chain_with_blocks(0));
        let node_b = Node::new(chain_with_blocks(0));
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        // A block 1 banata hai, vote karta hai, apni tally me rakhta hai.
        let mut tx = Transaction::new(alice.address.clone(), bob.address.clone(), 100, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            1,
            1_700_000_001,
            node_a.height_and_hash().1,
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        let vote = node_a.vote_for_block(&block, &validator).unwrap();
        assert_eq!(vote.voter, validator.address);
        // B A se votes maangta hai — 1 vote milega.
        // 2 connections: GetVotes + (baad me) GetBlocks/Height sync ke liye
        // nahi — yahan sirf GetVotes ke liye 1 connection.
        let (addr_a, handle) = serve_test_node(node_a.clone(), 1);
        let votes = Node::request_votes(&addr_a, 1).unwrap();
        handle.join().unwrap();
        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].voter, validator.address);
        // B vote ko apni tally me record kare, phir wahi block BFT se jode.
        node_b.record_vote(votes[0].clone()).unwrap();
        let tally = node_b.tallies.lock().unwrap().get(&1).unwrap().clone();
        let mut chain_b = node_b.chain.lock().unwrap();
        chain_b.add_block_bft(block, &tally).unwrap();
        assert_eq!(chain_b.height(), 2);
        assert_eq!(chain_b.balance(&bob.address), 100);
        chain_b.verify_chain().unwrap();
    }

    #[test]
    fn sync_karte_time_embedded_votes_bhi_verify_hote_hain() {
        // Node A ke blocks me votes embedded hain — B sync karke quorum
        // check ke saath judta hai.
        let node_a = Node::new(chain_with_blocks(2));
        let node_b = Node::new(chain_with_blocks(0));
        let (addr_a, handle) = serve_test_node(node_a, 3);
        let added = node_b.sync_from_peer(&addr_a).unwrap();
        handle.join().unwrap();
        assert_eq!(added, 2);
        assert_eq!(node_b.height_and_hash().0, 3);
        let chain_b = node_b.chain.lock().unwrap();
        // Har synced block me vote embedded hai.
        assert_eq!(chain_b.blocks[1].votes.len(), 1);
        assert_eq!(chain_b.blocks[2].votes.len(), 1);
        chain_b.verify_chain().unwrap();
    }

    #[test]
    fn registry_sync_aur_id_tx_block_sync() {
        // Node A: genesis + Bob ki registry entry (100001, @bob) + ID-tx block.
        let validator = test_wallet(9);
        let alice = test_wallet(1);
        let bob = test_wallet(2);
        let mut chain_a = Chain::new_genesis(
            vec![(alice.address.clone(), 1_000_000)],
            &validator,
            1_700_000_000,
        )
        .unwrap();
        let mut registry_a = ButPayRegistry::new();
        let bob_id = registry_a.register(&bob.address, Some("bob")).unwrap();
        assert_eq!(bob_id, 100001);
        // Block 1: Alice → ID 100001 (Bob) 500 bits.
        let mut tx = Transaction::new(alice.address.clone(), "100001".to_string(), 500, 0, 0);
        tx.sign(&alice.signing_key()).unwrap();
        let block = Block::new(
            1,
            1_700_000_001,
            chain_a.latest_hash(),
            vec![tx],
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        let mut tally = VoteTally::new(1);
        let vote = Vote::new(
            block.hash.clone(),
            1,
            true,
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        tally.add_vote(vote, &chain_a.validators).unwrap();
        chain_a
            .add_block_bft_with_registry(block, &tally, &registry_a)
            .unwrap();
        let node_a = Node::with_registry(chain_a, registry_a);
        // Node B: sirf genesis, registry khaali.
        let node_b = Node::new(chain_with_blocks(0));
        assert!(node_b.registry.lock().unwrap().is_empty());
        // Sync me 3 connections: Hello + GetRegistry + GetBlocks.
        let (addr_a, handle) = serve_test_node(node_a, 3);
        let added = node_b.sync_from_peer(&addr_a).unwrap();
        handle.join().unwrap();
        assert_eq!(added, 1);
        assert_eq!(node_b.height_and_hash().0, 2);
        // Registry sync hui — Bob ki ID aur handle milti hai.
        {
            let reg_b = node_b.registry.lock().unwrap();
            assert_eq!(reg_b.resolve("100001").unwrap(), bob.address);
            assert_eq!(reg_b.resolve("@bob").unwrap(), bob.address);
        }
        // ID-tx block bhi sync hua — Bob ko 500 mile.
        let chain_b = node_b.chain.lock().unwrap();
        assert_eq!(chain_b.balance(&bob.address), 500);
        // NOTE: khaali-registry wala `verify_chain()` ID-tx wali chain ko
        // verify nahi kar sakta — synced registry ke saath verify karo.
        let reg_b = node_b.registry.lock().unwrap().clone();
        chain_b.verify_chain_with_registry(&reg_b).unwrap();
    }

    #[test]
    fn registry_sync_bina_block_sync_ke_bhi_hoti_hai() {
        // Node A ki registry me Bob hai, par chain sirf genesis par hai.
        let bob = test_wallet(2);
        let mut registry_a = ButPayRegistry::new();
        registry_a.register(&bob.address, Some("bob")).unwrap();
        let node_a = Node::with_registry(chain_with_blocks(0), registry_a);
        // Node B bhi genesis par hai — height barabar, isliye sync_from_peer
        // 0 dega, par sync_registry_from_peer seedha chalao.
        let node_b = Node::new(chain_with_blocks(0));
        // 1 connection: GetRegistry.
        let (addr_a, handle) = serve_test_node(node_a, 1);
        let added = node_b.sync_registry_from_peer(&addr_a).unwrap();
        handle.join().unwrap();
        assert_eq!(added, 1);
        assert_eq!(
            node_b.registry.lock().unwrap().resolve("100001").unwrap(),
            bob.address
        );
    }

    #[test]
    fn equivocation_report_se_slash_op_mempool_me_aata_hai() {
        let node = Node::new(chain_with_blocks(0));
        let validator = test_wallet(9);
        let bob = test_wallet(2);
        // Validator height 1 par do alag blocks ko vote deta hai.
        let v1 = Vote::new(
            "a".repeat(64),
            1,
            true,
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        let v2 = Vote::new(
            "b".repeat(64),
            1,
            true,
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        node.record_vote(v1).unwrap();
        node.record_vote(v2).unwrap();
        // Bob report karta hai — 1 slash op mempool me aata hai.
        let ops = node.report_equivocation(&bob, 1).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].target, validator.address);
        assert_eq!(node.mempool.lock().unwrap().slash_ops_len(), 1);
        // Slash op wala block banao aur jodo.
        let slash_ops = node
            .mempool
            .lock()
            .unwrap()
            .take_slash_ops_for_block(10);
        let block = Block::new_with_ops(
            1,
            1_700_000_001,
            node.height_and_hash().1,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            slash_ops,
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        let mut tally = VoteTally::new(1);
        let vote = Vote::new(
            block.hash.clone(),
            1,
            true,
            validator.address.clone(),
            &validator.signing_key(),
        )
        .unwrap();
        tally
            .add_vote(vote, &node.chain.lock().unwrap().validators)
            .unwrap();
        node.chain.lock().unwrap().add_block_bft(block, &tally).unwrap();
        // Slash apply hua: validator jail hua (stake 0 < min stake) aur
        // saboot applied set me hai — dobara slash nahi hoga.
        let chain = node.chain.lock().unwrap();
        assert!(chain.validators.is_jailed(&validator.address));
        assert_eq!(chain.applied_slash_ops.len(), 1);
        chain.verify_chain().unwrap();
    }

    #[test]
    fn report_equivocation_bina_saboot_khaali() {
        let node = Node::new(chain_with_blocks(0));
        let bob = test_wallet(2);
        // Koi vote hi nahi hai — kuch report nahi hoga.
        let ops = node.report_equivocation(&bob, 1).unwrap();
        assert!(ops.is_empty());
        assert_eq!(node.mempool.lock().unwrap().slash_ops_len(), 0);
    }

    #[test]
    fn galat_vote_reject_hota_hai() {
        let node_b = Node::new(chain_with_blocks(0));
        let alice = test_wallet(1); // validator nahi hai
        let vote = Vote::new(
            "koi_hash".to_string(),
            1,
            true,
            alice.address.clone(),
            &alice.signing_key(),
        )
        .unwrap();
        // Alice active validator nahi hai — vote tally me nahi jayega.
        assert!(node_b.record_vote(vote).is_err());
        assert!(node_b
            .tallies
            .lock()
            .unwrap()
            .get(&1)
            .map(|t| t.votes.len())
            .unwrap_or(0)
            == 0);
    }
}
