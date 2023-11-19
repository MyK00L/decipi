use rand::{thread_rng, Rng};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::task;
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
 *  timestamp: 16b
 *
 * Init:
 *
 *  separate every message by (peer ip:port, claimed peer public key)
 *
 *  - own PeerId: 32b
 *  - 0: 1b (phase 0, only when first connecting to queue)
 *  - own PubSigKey: 32b
 *  - nonce: 16b
 *  - timestamp: 16b
 *  - signature: 32b
 *
 *  - own PeerId: 32b
 *  - 1: 1b (phase 1)
 *  - KEX public: 32b
 *  - signature: 64b
 *
 *  ms: 32b = result of diffie-hellman
 *
 *  - own PeerId: 32b
 *  - 2: 1b (phase 2)
 *  - nonce: 16b
 *  - timestamp: 16b
 *  - mac: 32b
 *
 * *mac: 32b (475 remaining bytes)
 *
 * Queue:
 *
 *  - message id: 8b (incremental)
 *  - number of parts - 1: 1b
 *  - part number: 1b
 *  - data: 508-?b
 *
 * File:
 *  - file id: 16b (hash of unencrypted contents)
 *  - part offset: 8b
 *  - data: 508-?b
 *
 * Request:
 *  TODO
 *  - data: 508-?b (?)
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

pub type PeerId = [u8; 32];
pub type PubSigKey = ed25519_dalek::VerifyingKey;
pub type MacKey = [u8; 32];
pub type Mac = blake3::Hash;

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
    peer_id: PeerId,
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
    init_ch: mpsc::Sender<(SocketAddr, PeerId, Vec<u8>)>,
    queue_ch: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    file_ch: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    request_ch: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    connections: Arc<RwLock<BTreeMap<SocketAddr, Connection>>>,
    whitelist: Option<Arc<RwLock<BTreeMap<SocketAddr, PeerId>>>>,
) {
    let mut data = [0u8; 65536];
    loop {
        let (size, addr) = socket.recv_from(&mut data).await.unwrap();
        if size < 1 {
            continue;
        }
        let result = match MessageType::from(data[0]) {
            MessageType::None => Err(HandleSocketError::UnknownMessageType),
            MessageType::KeepAlive => Ok(()),
            MessageType::Init => {
                if size < 33 {
                    continue;
                }
                let claimed_id: PeerId = data[1..33].try_into().unwrap();
                if let Some(ref wl) = whitelist {
                    if let Some(peer_id) = wl.read().await.get(&addr).copied() {
                        if peer_id == claimed_id {
                            init_ch
                                .send((addr, claimed_id, Vec::from(&data[33..size])))
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
                        .send((addr, claimed_id, Vec::from(&data[33..size])))
                        .await
                        .map_err(|_| HandleSocketError::SendError)
                }
            }
            mt => {
                if size < 33 {
                    continue;
                }
                let claimed_id: PeerId = data[1..33].try_into().unwrap();
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

async fn repeat_message(
    socket: Arc<UdpSocket>,
    dest: SocketAddr,
    data: &[u8],
    delays: &[Duration],
) {
    let _ = socket.send_to(data, dest).await;
    for dt in delays.iter() {
        sleep(*dt).await;
        let _ = socket.send_to(data, dest).await;
    }
}
/*
 * Init:
 *
 *  separate every message by (peer ip:port, claimed peer public key)
 *
 *
 *
 *  ms: 32b = result of diffie-hellman
 *
 */

/*#[derive(Debug,Clone,Serialize,Deserialize)]
enum InitMessage {
    ConnectToQueue{
        pub_sig_key: ed25519_dalek::VerifyingKey,
        timestamp: SystemTime,
        signature: ed25519_dalek::Signature,
    },
    Exchange{
        p
    },
    Finalize{},
}*/
async fn initter(
    socket: Arc<UdpSocket>,
    own_id: PeerId,
    init_ch: &mut mpsc::Receiver<(SocketAddr, PeerId, Vec<u8>)>,
    phase_1: Arc<
        RwLock<
            BTreeMap<(SocketAddr, PeerId), (task::JoinHandle<()>, x25519_dalek::EphemeralSecret)>,
        >,
    >,
    connections: Arc<RwLock<BTreeMap<SocketAddr, Connection>>>,
    id_to_key: Arc<RwLock<BTreeMap<PeerId, PubSigKey>>>,
    accept_phase_0: bool,
) {
    let mut phase_2 = BTreeMap::<(SocketAddr, PeerId), task::JoinHandle<()>>::new();

    loop {
        if let Some((addr, peer_id, data)) = init_ch.recv().await {
            if data.is_empty() {
                continue;
            }
            let phase = data[0];
            match phase {
                0 => {
                    //TODO handle peer_id better
                    if data.len() != 1 + 32 + 16 + 64 {
                        continue;
                    }
                    if accept_phase_0 {
                        let pkr = ed25519_dalek::VerifyingKey::from_bytes(
                            &data[1..33].try_into().unwrap(),
                        );
                        if let Ok(pk) = pkr {
                            let tsb: [u8; 16] = data[33..49].try_into().unwrap();
                            let nanos = u128::from_be_bytes(tsb);
                            const NANOS_IN_SEC: u128 = 1_000_000_000u128;
                            let ts = SystemTime::UNIX_EPOCH
                                + Duration::new(
                                    (nanos / NANOS_IN_SEC) as u64,
                                    (nanos % NANOS_IN_SEC) as u32,
                                );
                            let now = SystemTime::now();
                            let dt = if ts > now {
                                ts.duration_since(now).unwrap()
                            } else {
                                now.duration_since(ts).unwrap()
                            };
                            if dt > Duration::from_secs(60) {
                                continue;
                            }
                            let signature = ed25519_dalek::Signature::from_bytes(
                                &data[49..113].try_into().unwrap(),
                            );
                            let ipb: Vec<u8> = match addr {
                                SocketAddr::V4(a4) => a4.ip().to_bits().to_be_bytes().to_vec(),
                                SocketAddr::V6(a6) => a6.ip().to_bits().to_be_bytes().to_vec(),
                            };
                            let message: Vec<u8> = [
                                &ipb,
                                &addr.port().to_be_bytes() as &[u8],
                                &peer_id as &[u8],
                                &data[..49],
                            ]
                            .concat();
                            if pk.verify_strict(&message, &signature).is_ok() {
                                id_to_key.write().await.insert(peer_id, pk);
                            }
                        }
                    }
                }
                1 => {
                    /*
                     *  / own PeerId: 32b
                     *  - 1: 1b (phase 1)
                     *  - KEX public: 32b
                     *  - signature: 64b
                     */
                    //phase_1: Arc<RwLock<BTreeMap<(SocketAddr, PeerId), (task::JoinHandle<()>,x25519_dalek::EphemeralSecret)>>>,
                    //id_to_key: Arc<RwLock<BTreeMap<PeerId, PubSigKey>>>,
                    if data.len() != 1 + 32 + 64 {
                        let peer_kex_public = x25519_dalek::PublicKey::from(
                            *<&[u8; 32]>::try_from(&data[1..33]).unwrap(),
                        );
                        let signature = ed25519_dalek::Signature::from_bytes(
                            <&[u8; 64]>::try_from(&data[33..97]).unwrap(),
                        );
                        if let Some(peer_sig_public) = id_to_key.read().await.get(&peer_id).copied()
                        {
                            if peer_sig_public
                                .verify_strict(&data[..33], &signature)
                                .is_ok()
                            {
                                if let Some((task, sk)) =
                                    phase_1.write().await.remove(&(addr, peer_id))
                                {
                                    let shared = sk.diffie_hellman(&peer_kex_public);
                                    let sock = socket.clone();
                                    //sock, dest, data, delays
                                    let message = [0u8]; //[MessageType::Init, own_peer_id, 2, timestamp];
                                                         /*let mut message = [0u8;1+32+1+16+16+32];
                                                         message[0] = MessageType::Init;
                                                         message[1..33] = own_id;
                                                         message[33] = 2;
                                                         let ts = SystemTime::now();
                                                         message[34..40]*/
                                    let delays = [
                                        Duration::from_millis(125),
                                        Duration::from_millis(500),
                                        Duration::from_secs(1),
                                        Duration::from_secs(2),
                                        Duration::from_secs(8),
                                    ];
                                    tokio::task::spawn(async move {
                                        repeat_message(sock, addr, &message, &delays).await;
                                    });
                                }
                            }
                        }
                    }
                }
                2 => {
                    /*
                     *  / own PeerId: 32b
                     *  - 2: 1b (phase 2)
                     *  - timestamp: 16b
                     *  - mac: 32b
                     */
                    if data.len() != 1 + 16 + 16 + 32 {
                        continue;
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct NetworkManager {
    // bind to ::/0 for magic
    socket: Option<Arc<UdpSocket>>,

    // pairs of (addr,pubkey) allowed, others will be immediately discarded
    // It will be None for servers (queue) that want to accept any connection,
    // but will always be Some for the other entities (contest master, participant, worker)
    whitelist: Option<Arc<RwLock<BTreeMap<SocketAddr, PeerId>>>>,

    // list of established connections (not necessarly active, but with a shared mac secret)
    connections: Arc<RwLock<BTreeMap<SocketAddr, Connection>>>,
}
impl NetworkManager {
    pub async fn is_connected_to(peer_id: PeerId) -> bool {
        todo!()
    }
    pub async fn connect_to(addr: SocketAddr, pk: PeerId) {
        todo!()
    }
}
