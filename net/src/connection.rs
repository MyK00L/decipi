use super::message::{KeepAliveMessage, MacKey, Message, PeerAddr, PeerId};
use rand::thread_rng;
use rand::Rng;
use speedy::{Readable, Writable};
use std::collections::BTreeMap;
use std::sync::{Arc, Weak};
use std::time::{Duration, SystemTime};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;

/*
 * Connection = after kex was performed
 */

#[derive(Debug)]
struct ConnectionInfo {
    pub mac_key: MacKey,
    pub peer_id: PeerId,
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
pub struct AliveConnection(ConnectionInfo, task::AbortHandle);
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
    pub fn peer_addr(&self) -> PeerAddr {
        self.0.peer_addr
    }
    #[inline]
    pub fn mac_key(&self) -> MacKey {
        self.0.mac_key
    }
    #[inline]
    pub fn peer_id(&self) -> PeerId {
        self.0.peer_id
    }
}
impl Drop for AliveConnection {
    fn drop(&mut self) {
        self.1.abort();
    }
}

pub type Connection = Arc<AliveConnection>;

// Single connection manager
#[derive(Debug)]
pub struct ConnectionManager {
    weak_connections: RwLock<BTreeMap<PeerId, RwLock<(Weak<AliveConnection>, task::AbortHandle)>>>,
}
impl ConnectionManager {
    async fn get_connection(&self, peer_id: PeerId) -> Connection {
        if let Some(rw) = self.weak_connections.read().await.get(&peer_id) {
            let wc = rw.read().await.0.clone();
            match wc.upgrade() {
                Some(c) => c,
                None => {
                    let wl = rw.write().await;
                    match wl.0.upgrade() {
                        // got to make sure this was not re-connected while the lock was released
                        Some(c) => c,
                        None => {
                            todo!();
                        }
                    }
                }
            }
        } else {
            todo!();
            //weak_connections.write().try_insert()
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeakConnectionManager {
    manager: Arc<ConnectionManager>,
}
