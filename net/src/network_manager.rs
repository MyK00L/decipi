use rand::{thread_rng, Rng};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration};

/*
 * UDP message data structure:
 *
 * 508 bytes per message (avoid fragmentation to keep packet loss to a minimum)
 *
 * "*" means every message from now on will contain this
 *
 * *message type: u8 (507 reamining bytes)
 *
 * KeepAlive:
 *  timestamp
 *
 * Init:
 *
 *  separate every message by (peer ip:port, claimed peer public key)
 *
 *  - 0: u8
 *  - own public enc key
 *  - Enc(c: rand u128, peer public enc key)
 *
 *  ms: u256 = H(c1,c2) secret for mac in every other message type
 *
 *  - 1: u8
 *  - own public enc key
 *  - Enc(ms, peer public key)
 *
 * *mac: u256 (475 remaining bytes)
 *
 * Queue:
 *
 *  - message id: u32 (incremental)
 *  - number of parts - 1: u8
 *  - part number: u8
 *  - data: 508-1-32-4-1-1
 *
 * File:
 *  - file id: u128 (hash)
 *  - part offset: u32
 *  - data: 508-1-32-16-4
 *
 * Request:
 *  TODO
 *  - data: 508-1-32 (?)
 *
 * */

// First byte of every message
#[repr(u8)]
enum MessageType {
    None = 0,
    KeepAlive = 1,
    Init = 2,
    Queue = 3,
    File = 4,
    Request = 5,
}
impl From<u8> for MessageType {
    fn from(n: u8) -> Self {
        match n {
            1 => MessageType::KeepAlive,
            2 => MessageType::Init,
            3 => MessageType::Queue,
            4 => MessageType::File,
            5 => MessageType::Request,
            _ => MessageType::None,
        }
    }
}

type PubKey = [u8; 32];
type MacKey = [u8; 32];
type Mac = blake3::Hash;

async fn keep_alive(
    socket: Arc<UdpSocket>,
    dest_addr: SocketAddr,
    interval_min: Duration,
    interval_max: Duration,
) {
    let mut rng = thread_rng();
    let mut data = [0u8; 17];
    data[0] = MessageType::KeepAlive as u8;
    loop {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        data[1..17].copy_from_slice(&timestamp.to_be_bytes());
        socket.send_to(&data, &dest_addr).await.unwrap();
        let interval = rng.gen_range(interval_min..=interval_max);
        sleep(interval).await;
    }
}

#[derive(Clone, Copy, Debug)]
struct Connection {
    mac_key: MacKey,
    peer_id: PubKey,
}

#[derive(Clone, Copy, Debug)]
enum HandleSocketError {
    SendError,
    PeerNotInWhitelist,
    PeerClaimedWrongId,
    PeerNotConnected,
    MacCheckFailed,
    UnknownMessageType,
}
// This should only read from the socket and dispatch the messages to other places
async fn handle_socket(
    socket: Arc<UdpSocket>,
    init_ch: mpsc::Sender<(SocketAddr, PubKey, Vec<u8>)>,
    queue_ch: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    file_ch: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    request_ch: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    connections: Arc<RwLock<BTreeMap<SocketAddr, Connection>>>,
    whitelist: Option<Arc<RwLock<BTreeMap<SocketAddr, PubKey>>>>,
) {
    let mut data = [0u8; 65536];
    loop {
        let (size, addr) = socket.recv_from(&mut data).await.unwrap();
        let result = match MessageType::from(data[0]) {
            MessageType::None => Err(HandleSocketError::UnknownMessageType),
            MessageType::KeepAlive => Ok(()),
            MessageType::Init => {
                let claimed_id: PubKey = data[2..34].try_into().unwrap();
                if let Some(ref wl) = whitelist {
                    if let Some(peer_id) = wl.read().await.get(&addr).copied() {
                        if peer_id == claimed_id {
                            init_ch
                                .send((addr, claimed_id, Vec::from(&data[1..size])))
                                .await
                                .map_err(|_| HandleSocketError::SendError)
                        } else {
                            Err(HandleSocketError::PeerClaimedWrongId)
                        }
                    } else {
                        Err(HandleSocketError::PeerNotInWhitelist)
                    }
                } else {
                    init_ch
                        .send((addr, claimed_id, Vec::from(&data[1..size])))
                        .await
                        .map_err(|_| HandleSocketError::SendError)
                }
            }
            mt => {
                if let Some(connection) = connections.read().await.get(&addr).copied() {
                    if hmac(&data[33..size], &connection.mac_key) == data[1..33] {
                        match mt {
                            MessageType::Queue => queue_ch
                                .send((addr, Vec::from(&data[1..size])))
                                .await
                                .map_err(|_| HandleSocketError::SendError),
                            MessageType::File => file_ch
                                .send((addr, Vec::from(&data[1..size])))
                                .await
                                .map_err(|_| HandleSocketError::SendError),
                            MessageType::Request => request_ch
                                .send((addr, Vec::from(&data[1..size])))
                                .await
                                .map_err(|_| HandleSocketError::SendError),
                            _ => unreachable!(),
                        }
                    } else {
                        Err(HandleSocketError::MacCheckFailed)
                    }
                } else {
                    Err(HandleSocketError::PeerNotConnected)
                }
            }
        };
        // TODO: log instead of panic
        result.unwrap();
    }
}
fn hmac(data: &[u8], key: &MacKey) -> Mac {
    // TODO is this really safe? implementation does not seem to be an [HMAC](https://datatracker.ietf.org/doc/html/rfc2104)
    blake3::keyed_hash(key, data)
}


pub struct NetworkManager {
    socket_v6: Option<Arc<UdpSocket>>,
    socket_v4: Option<Arc<UdpSocket>>,

    // pairs of (addr,pubkey) allowed, others will be immediately discarded
    // It will be None for servers (queue) that want to accept any connection,
    // but will always be Some for the other entities (contest master, participant, worker)
    whitelist: Option<Arc<RwLock<BTreeMap<SocketAddr, PubKey>>>>,
    
    // list of connections in the Init phase
    // there can be a connection initializing even if the address is already connected
    // if the initialization succeeds, the element in "connections" is replaced
    initializing: Arc<RwLock<BTreeMap<SocketAddr, Connection>>>,

    // list of established connections (not necessarly active, but with a shared mac secret)
    connections: Arc<RwLock<BTreeMap<SocketAddr, Connection>>>,

}
impl NetworkManager {
    pub async fn is_connected_to(peer_id: PubKey) -> bool {
        todo!()
    }
    pub async fn connect_to(addr: SocketAddr, pk: PubKey) {
        todo!()
    }
}
