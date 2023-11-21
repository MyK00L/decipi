use super::message::{KeepAliveMessage, MacKey, Message, PeerAddr, PubSigKey};
use core::hash::{Hash, Hasher};
use rand::thread_rng;
use rand::Rng;
use speedy::{Readable, Writable};
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::{Duration, SystemTime};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;

/*
 * Connection = after kex was performed
 */

#[derive(Debug, Clone)]
struct ConnectionInfo {
    pub mac_key: MacKey,
    pub peer_id: PubSigKey,
    pub peer_addr: PeerAddr,
}
async fn keep_alive(
    socket: Arc<UdpSocket>,
    dest_addr: PeerAddr,
    delay_min: Duration,
    delay_max: Duration,
) {
    let mut buf = [0u8; 12];
    let addr: std::net::SocketAddr = dest_addr.into();
    loop {
        let message = Message::KeepAlive(KeepAliveMessage(SystemTime::now()));
        message.write_to_buffer(&mut buf).unwrap(); // TODO dont unwrap?
        socket.send_to(&buf, &addr).await.unwrap();
        let interval = thread_rng().gen_range(delay_min..=delay_max);
        sleep(interval).await;
    }
}
#[derive(Debug)]
struct AliveConnection(ConnectionInfo, task::AbortHandle);
impl AliveConnection {
    fn new(c: ConnectionInfo, socket: Arc<UdpSocket>) -> Self {
        let addr = c.peer_addr;
        Self(
            c,
            task::spawn(keep_alive(
                socket,
                addr,
                Duration::from_secs(15),
                Duration::from_secs(28),
            ))
            .abort_handle(),
        )
    }
    #[inline]
    fn peer_addr(&self) -> PeerAddr {
        self.0.peer_addr
    }
    #[inline]
    fn mac_key(&self) -> MacKey {
        self.0.mac_key
    }
    #[inline]
    fn peer_id(&self) -> PubSigKey {
        self.0.peer_id
    }
    #[inline]
    fn is_alive(&self) -> bool {
        !self.1.is_finished()
    }
}
impl Drop for AliveConnection {
    fn drop(&mut self) {
        self.1.abort();
    }
}

#[derive(Clone, Debug)]
pub struct Connection(Arc<RwLock<AliveConnection>>);
impl Connection {
    #[inline]
    pub async fn peer_addr(&self) -> PeerAddr {
        self.0.read().await.peer_addr()
    }
    #[inline]
    pub async fn mac_key(&self) -> MacKey {
        self.0.read().await.mac_key()
    }
    #[inline]
    pub async fn peer_id(&self) -> PubSigKey {
        self.0.read().await.peer_id()
    }
    #[inline]
    pub async fn is_alive(&self) -> bool {
        self.0.read().await.is_alive()
    }
    /// Only call this if a Connection to this peer does not already exist
    fn new(c: ConnectionInfo, socket: Arc<UdpSocket>) -> Self {
        Self(Arc::new(RwLock::new(AliveConnection::new(c, socket))))
    }
}

#[derive(Debug)]
struct ConnectionManager(RwLock<(Weak<RwLock<AliveConnection>>, ConnectionInfo)>);
impl ConnectionManager {
    fn new(ci: ConnectionInfo) -> Self {
        Self(RwLock::new((Weak::new(), ci)))
    }
    async fn get_connection(&self, socket: Arc<UdpSocket>) -> Connection {
        match self.0.read().await.0.upgrade() {
            Some(x) => Connection(x),
            None => {
                let mut wl = self.0.write().await;
                match wl.0.upgrade() {
                    Some(x) => Connection(x),
                    None => {
                        let nc = Connection(Arc::new(RwLock::new(AliveConnection::new(
                            wl.1.clone(),
                            socket,
                        ))));
                        wl.0 = Arc::downgrade(&nc.0);
                        nc
                    }
                }
            }
        }
    }
    async fn update_info(&self, peer_addr: PeerAddr, mac_key: MacKey) {
        let mut wl = self.0.write().await;
        wl.1.peer_addr = peer_addr;
        wl.1.mac_key = mac_key;
    }
}

// Single connection manager
#[derive(Debug)]
struct ConnectionsManager {
    connections: HashMap<PubSigKey, ConnectionManager>,
    socket: Arc<UdpSocket>,
}
impl ConnectionsManager {
    async fn get_connection(&self, peer_id: PubSigKey) -> Connection {
        self.connections
            .get(&peer_id)
            .unwrap()
            .get_connection(self.socket.clone())
            .await
    }
}

#[derive(Debug, Clone)]
pub struct WeakConnectionManager {
    manager: Arc<ConnectionManager>,
}

/*
 * Redo connection because:
 * can't invalidate connection if someone has an arc to it
 * Arc<RwLock> ?
 *
 * Keep: last message timestamp from x
 * Attempt to recreate a connection if timestamp too old?
 *
 * Connections with Server and Master: always keep alive
 * Connections Participant-Participant, Participant-Worker: keep alive until submission is
 * evaluated
 *
 */
