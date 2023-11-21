use super::message::{KeepAliveMessage, MacKey, Message, PeerAddr, PubSigKey};
use core::hash::{Hash, Hasher};
use rand::thread_rng;
use rand::Rng;
use speedy::{Readable, Writable};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, SystemTime};
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use tokio::task;
use tokio::time::sleep;

/*
 * Connection = after kex was performed
 */

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
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
    let mut buf = [0u8; 16];
    let addr: std::net::SocketAddr = dest_addr.into();
    loop {
        let message = Message::KeepAlive(KeepAliveMessage(SystemTime::now()));
        message.write_to_buffer(&mut buf).unwrap();
        let interval = if socket.send_to(&buf, &addr).await.is_ok() {
            thread_rng().gen_range(delay_min..=delay_max)
        } else {
            delay_min
        };
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
struct ConnectionManager(Mutex<(Weak<RwLock<AliveConnection>>, ConnectionInfo)>);
impl ConnectionManager {
    fn new(ci: ConnectionInfo) -> Self {
        Self(Mutex::new((Weak::new(), ci)))
    }
    async fn get_connection(&self, socket: Arc<UdpSocket>) -> Connection {
        let mut wl = self.0.lock().await;
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
    async fn update_info(&self, peer_addr: PeerAddr, mac_key: MacKey, socket: Arc<UdpSocket>) {
        let mut wl = self.0.lock().await;
        wl.1.peer_addr = peer_addr;
        wl.1.mac_key = mac_key;
        match wl.0.upgrade() {
            None => {}
            Some(acwl) => {
                let mut ac = acwl.write().await;
                let peer_id = ac.peer_id();
                *ac = AliveConnection::new(
                    ConnectionInfo {
                        mac_key,
                        peer_id,
                        peer_addr,
                    },
                    socket,
                );
            }
        }
    }
}

static CONNECTIONS: LazyLock<RwLock<HashMap<PubSigKey, ConnectionManager>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub async fn get_connection(peer_id: PubSigKey, socket: Arc<UdpSocket>) -> Connection {
    CONNECTIONS
        .read()
        .await
        .get(&peer_id)
        .unwrap()
        .get_connection(socket.clone())
        .await
}
pub async fn set_connection_info(connection_info: ConnectionInfo, socket: Arc<UdpSocket>) {
    let ConnectionInfo {
        mac_key,
        peer_id,
        peer_addr,
    } = connection_info;
    let mut hwl = CONNECTIONS.write().await;
    if let Some(cm) = hwl.get(&peer_id) {
        cm.update_info(peer_addr, mac_key, socket).await;
    } else {
        hwl.insert(peer_id, ConnectionManager::new(connection_info));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn get_dummy_mac_key() -> MacKey {
        use x25519_dalek::{EphemeralSecret, PublicKey};

        let alice_secret = EphemeralSecret::random();
        let alice_public = PublicKey::from(&alice_secret);

        let bob_secret = EphemeralSecret::random();
        let bob_public = PublicKey::from(&bob_secret);

        let alice_shared_secret = alice_secret.diffie_hellman(&bob_public);
        let bob_shared_secret = bob_secret.diffie_hellman(&alice_public);
        assert_eq!(alice_shared_secret.as_bytes(), bob_shared_secret.as_bytes());

        MacKey(alice_shared_secret.to_bytes())
    }

    #[tokio::test]
    async fn test() {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:1234").await.unwrap());
        let peer_addr = PeerAddr::from("127.0.0.1:1234".parse::<std::net::SocketAddr>().unwrap());
        let mac_key = get_dummy_mac_key();
        let peer_id = PubSigKey(ed25519_dalek::VerifyingKey::from_bytes(&[42u8; 32]).unwrap());

        let ci1 = ConnectionInfo {
            mac_key,
            peer_addr,
            peer_id,
        };

        set_connection_info(ci1.clone(), socket.clone()).await;
        let c = get_connection(peer_id, socket.clone()).await;

        sleep(std::time::Duration::from_secs(2)).await;
        set_connection_info(ci1.clone(), socket.clone()).await; // shoudl get dropped here once
        sleep(std::time::Duration::from_secs(2)).await;
        // and then here
    }
}

/*
 *
 * Keep: last message timestamp from x
 * Attempt to recreate a connection if timestamp too old?
 *
 * Connections with Server and Master: always keep alive
 * Connections Participant-Participant, Participant-Worker: keep alive until submission is
 * evaluated
 *
 */
