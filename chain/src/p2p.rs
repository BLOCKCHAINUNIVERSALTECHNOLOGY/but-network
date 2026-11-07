//! P2P network — Phase 2: nodes TCP se aapas me judte hain.
//!
//! Kya kar sakta hai:
//! - Ek node doosre node se apni chain ki height aur latest hash poochh sakta hai.
//! - Node doosre node se missing blocks maang kar apni chain sync kar sakta hai
//!   (har block `Chain::add_block` se poori tarah verify hokar hi judta hai).
//! - Pending transactions ek node se doosre node tak bhej sakta hai
//!   (mempool me jaane se pehle signature verify hoti hai).
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
//!   Peer ki chain alag nikli (diverged) to `add_block` prev-hash mismatch
//!   par reject kar dega — node apni purani chain par hi rahega, lambi
//!   chain ko apnana (reorg) abhi nahi hota.
//! - **No encryption / authentication**: koi bhi TCP client node se jud
//!   sakta hai. Blocks aur transactions khud verify hote hain (signature +
//!   hash), isliye galat data chain me nahi ghus sakta — par node ko spam
//!   requests se pareshan kiya ja sakta hai (rate limiting abhi nahi hai).
//! - **No vote gossip**: BFT votes abhi P2P par nahi ghoomte — vote sirf
//!   `consensus.rs` ke andar test hote hain. Vote gossip Phase 3 ka kaam hai.
//! - **Request-response, ek connection = ek request**: har request ke liye
//!   naya TCP connection khulta hai. Persistent peer connections abhi nahi hain.
//!
//! Wire format: har message ek JSON line hai (`\n` se khatm). Request ke
//! jawab me hamesha ek JSON line milti hai.

use crate::block::Block;
use crate::chain::Chain;
use crate::error::{ButError, Result};
use crate::mempool::Mempool;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
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
    /// Kisi request ka seedha jawab — kaam hua ya nahi.
    Ack { ok: bool, message: String },
}

/// Ek BUT node — chain + mempool + peers, threads ke beech shareable.
#[derive(Debug, Clone)]
pub struct Node {
    /// Is node ki chain (server thread aur API thread dono use karte hain).
    pub chain: Arc<Mutex<Chain>>,
    /// Pending transactions.
    pub mempool: Arc<Mutex<Mempool>>,
    /// Jaane hue peers — "127.0.0.1:9001" jaise addresses.
    pub peers: Arc<Mutex<Vec<String>>>,
}

impl Node {
    /// Chain se naya node banao.
    pub fn new(chain: Chain) -> Self {
        Self {
            chain: Arc::new(Mutex::new(chain)),
            mempool: Arc::new(Mutex::new(Mempool::new())),
            peers: Arc::new(Mutex::new(Vec::new())),
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
            other => P2pMessage::Ack {
                ok: false,
                message: format!("ye message request nahi hai: {other:?}"),
            },
        }
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

    /// Peer se chain sync karo: missing blocks maango, verify karke add karo.
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
        let blocks = Self::request_blocks(peer, own_height as u64)?;
        let mut added = 0usize;
        for block in blocks {
            {
                let mut chain = self.chain.lock().unwrap();
                chain.add_block(block)?;
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
            let block = Block::new(
                chain.height() as u64,
                1_700_000_001 + i,
                chain.latest_hash(),
                vec![tx],
                validator.address.clone(),
                &validator.signing_key(),
            )
            .unwrap();
            chain.add_block(block).unwrap();
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
        // Sync me 2 connections hote hain: Hello + GetBlocks.
        let (addr_a, handle) = serve_test_node(node_a, 2);
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
        // Sync me 2 connections: Hello + GetBlocks, par block add fail hoga.
        let (addr_a, handle) = serve_test_node(node_a, 2);
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
}
