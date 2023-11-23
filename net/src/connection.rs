mod init;

use super::message::*;
use crate::socket::*;
use rand::thread_rng;
use rand::Rng;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;

const KEEPALIVE_MSG_SIZE: usize = 13;

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
    mut socket: SocketWriter<KEEPALIVE_MSG_SIZE>,
    dest_addr: PeerAddr,
    mac_key: MacKey,
    delay_min: Duration,
    delay_max: Duration,
) {
    loop {
        let message = Message::Init(InitMessage::KeepAlive(
            socket.psk(),
            Macced::new_from_mac_key(KeepAliveInner(SystemTime::now()), &mac_key),
        ));
        let interval = if socket.send_to(message, dest_addr).await.is_ok() {
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
    fn new(c: ConnectionInfo, socket: SocketWriter<KEEPALIVE_MSG_SIZE>) -> Self {
        let addr = c.peer_addr;
        Self(
            c.clone(),
            task::spawn(keep_alive(
                socket,
                addr,
                c.mac_key,
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
    fn new(c: ConnectionInfo, socket: SocketWriter<KEEPALIVE_MSG_SIZE>) -> Self {
        Self(Arc::new(RwLock::new(AliveConnection::new(c, socket))))
    }
}

#[derive(Debug)]
struct ConnectionManager(Weak<RwLock<AliveConnection>>, ConnectionInfo);
impl ConnectionManager {
    fn new(ci: ConnectionInfo) -> Self {
        Self(Weak::new(), ci)
    }
    async fn get_connection(&mut self, socket: SocketWriter<KEEPALIVE_MSG_SIZE>) -> Connection {
        match self.0.upgrade() {
            Some(x) => Connection(x),
            None => {
                let nc = Connection(Arc::new(RwLock::new(AliveConnection::new(
                    self.1.clone(),
                    socket,
                ))));
                self.0 = Arc::downgrade(&nc.0);
                nc
            }
        }
    }
    async fn update_info(
        &mut self,
        peer_addr: PeerAddr,
        mac_key: MacKey,
        socket: SocketWriter<KEEPALIVE_MSG_SIZE>,
    ) {
        self.1.peer_addr = peer_addr;
        self.1.mac_key = mac_key;
        match self.0.upgrade() {
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
/*
pub async fn get_connection(
    peer_id: PubSigKey,
    socket: SocketWriter<KEEPALIVE_MSG_SIZE>,
) -> Connection {
    CONNECTIONS
        .read()
        .await
        .get(&peer_id)
        .unwrap()
        .get_connection(socket.clone())
        .await
}*/
/*pub async fn set_connection_info(
    connection_info: ConnectionInfo,
    socket: SocketWriter<KEEPALIVE_MSG_SIZE>,
) {
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
}*/

#[cfg(test)]
mod test {}

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
