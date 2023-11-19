use super::message::*;
use rand::{thread_rng, Rng};
use speedy::{Readable, Writable};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::task;
use tokio::time::{sleep, Duration};

async fn keep_alive(
    socket: Arc<UdpSocket>,
    dest_addr: SocketAddr,
    interval_min: Duration,
    interval_max: Duration,
) {
    let mut rng = thread_rng();
    let mut buf = [0u8; 12];
    loop {
        let message = Message::KeepAlive(KeepAliveMessage(SystemTime::now()));
        message.write_to_buffer(&mut buf).unwrap(); // TODO dont unwrap?
        socket.send_to(&buf, &dest_addr).await.unwrap();
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
    init_ch: mpsc::Sender<(PeerAddr, InitMessage)>,
    queue_ch: mpsc::Sender<(PeerAddr, QueueMessage)>,
    file_ch: mpsc::Sender<(PeerAddr, FileMessage)>,
    request_ch: mpsc::Sender<(PeerAddr, RequestMessage)>,
    connections: Arc<RwLock<BTreeMap<PeerAddr, Connection>>>,
    whitelist: Option<Arc<RwLock<BTreeMap<PeerAddr, PeerId>>>>,
) {
    let mut data = [0u8; 65536];
    loop {
        let (size, std_addr) = socket.recv_from(&mut data).await.unwrap();
        let addr = PeerAddr::from_std(std_addr);
        let Ok(message) = Message::read_from_buffer(&data[..size]) else {
            continue;
        };
        match message {
            Message::KeepAlive(_) => {}
            Message::Init(message) => {
                init_ch.send((addr, message)).await; //.unwrap();
            }
            Message::Queue(macced_message) => {
                let Some(connection) = connections.read().await.get(&addr).copied() else {
                    continue;
                };
                let Some(message) = macced_message.inner(&connection.mac_key) else {
                    continue;
                };
                queue_ch.send((addr, message)).await; //.map_err(|_| HandleSocketError::SendError)
            }
            Message::File(macced_message) => {
                let Some(connection) = connections.read().await.get(&addr).copied() else {
                    continue;
                };
                let Some(message) = macced_message.inner(&connection.mac_key) else {
                    continue;
                };
                file_ch.send((addr, message)).await; //.map_err(|_| HandleSocketError::SendError)
            }
            Message::Request(macced_message) => {
                let Some(connection) = connections.read().await.get(&addr).copied() else {
                    continue;
                };
                let Some(message) = macced_message.inner(&connection.mac_key) else {
                    continue;
                };
                request_ch.send((addr, message)).await; //.map_err(|_| HandleSocketError::SendError)
            }
        }
        // TODO: do something with the errors
    }
}

async fn repeat_packet(socket: Arc<UdpSocket>, dest: SocketAddr, data: &[u8], delays: &[Duration]) {
    let _ = socket.send_to(data, dest).await;
    for dt in delays.iter() {
        sleep(*dt).await;
        let _ = socket.send_to(data, dest).await;
    }
}

pub struct NetworkManager {
    // bind to ::/0 for magic (it gets bound to both v4 and v6)
    socket: Option<Arc<UdpSocket>>,

    // pairs of (addr,pubkey) allowed, others will be immediately discarded
    // It will be None for servers (queue) that want to accept any connection,
    // but will always be Some for the other entities (contest master, participant, worker)
    whitelist: Option<Arc<RwLock<BTreeMap<PeerAddr, PeerId>>>>,

    // list of established connections (not necessarly active, but with a shared mac secret)
    connections: Arc<RwLock<BTreeMap<PeerAddr, Connection>>>,
}
impl NetworkManager {
    pub async fn is_connected_to(peer_id: PeerId) -> bool {
        todo!()
    }
    pub async fn connect_to(addr: SocketAddr, pk: PeerId) {
        todo!()
    }
}
