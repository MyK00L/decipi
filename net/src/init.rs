use crate::connection::*;
use crate::message::*;
use crate::socket::*;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use rand::{thread_rng, Rng};
use scc::HashMap;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::time::SystemTime;
use tokio::task;
use tokio::task::AbortHandle;
use tokio::time::{sleep, Duration};

enum WaitersOrConnection {
    #[allow(clippy::type_complexity)]
    Waiters(Vec<Arc<Mutex<(Option<Connection>, Option<Waker>)>>>),
    Connection(ConnectionManager),
}

pub struct InitState {
    // this hashmap contains info about connecting peers
    // things are in this hashmap because you want to connect to them
    // and either:
    // - they did not send you their PubKexKey yet
    // - or they did not send you a correct KeepAlive message yet
    // once both these are satitsfied, the connection is considered established,
    // so you should abort sending your PubKexKey with the AbortHandle
    initting: HashMap<(PubSigKey, PeerAddr), (Option<SecKexKey>, AbortHandle)>,
    wocs: HashMap<PubSigKey, WaitersOrConnection>,
}
struct GetConnectionFuture(Arc<Mutex<(Option<Connection>, Option<Waker>)>>);
impl Future for GetConnectionFuture {
    type Output = Connection;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.lock().unwrap();
        match state.0.take() {
            Some(c) => Poll::Ready(c),
            None => {
                state.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
impl GetConnectionFuture {
    fn new(ww: Arc<Mutex<(Option<Connection>, Option<Waker>)>>) -> Self {
        // impl Future<Output = Connection>
        Self(ww)
    }
}

impl InitState {
    pub fn new() -> Self {
        Self {
            initting: HashMap::new(),
            wocs: HashMap::new(),
        }
    }
    pub async fn get_connection(
        &self,
        socket: SocketWriterBuilder,
        own_entity: Entity,
        peer_id: PubSigKey,
        peer_addr: PeerAddr,
    ) -> Connection {
        match self
            .wocs
            .entry_async(peer_id)
            .await
            .or_insert(WaitersOrConnection::Waiters(vec![]))
            .get_mut()
        {
            WaitersOrConnection::Connection(cm) => {
                if cm.peer_addr() != peer_addr {
                    cm.update_info(peer_addr, cm.mac_key(), socket.clone().into())
                        .await;
                }
                return cm.get_connection(socket.into()).await;
            }
            WaitersOrConnection::Waiters(w) => {
                if !self.initting.contains_async(&(peer_id, peer_addr)).await {
                    self.init_connection(socket, own_entity, peer_id, peer_addr)
                        .await;
                }
                let ww = Arc::new(Mutex::new((None, None)));
                w.push(ww.clone());
                GetConnectionFuture::new(ww)
            }
        }
        .await
    }
    async fn init_connection(
        &self,
        socket: SocketWriterBuilder,
        own_entity: Entity,
        peer_id: PubSigKey,
        peer_addr: PeerAddr,
    ) {
        let _ = self
            .initting
            .insert_async(
                (peer_id, peer_addr),
                new_initting(socket, own_entity, peer_addr).await,
            )
            .await;
    }

    async fn finalize_connection(
        &self,
        socket: SocketWriterBuilder,
        own_entity: Entity,
        peer_id: PubSigKey,
        peer_addr: PeerAddr,
        peer_pkk: PubKexKey,
    ) {
        let Some(skk) = self
            .initting
            .entry_async((peer_id, peer_addr))
            .await
            .or_insert(new_initting(socket.clone(), own_entity, peer_addr).await)
            .get_mut()
            .0
            .take()
        else {
            // skk is only taken in this function,
            // if it's None it means it was already finalized
            return;
        };
        let mac_key = MacKey::from(skk.diffie_hellman(&peer_pkk.into()));
        let connection_info = ConnectionInfo {
            mac_key,
            peer_id,
            peer_addr,
        };
        // TODO: understand why it works with if let but not without
        #[allow(irrefutable_let_patterns)]
        if let woc = self
            .wocs
            .entry_async(peer_id)
            .await
            .or_insert(WaitersOrConnection::Waiters(vec![]))
            .get_mut()
        {
            if let WaitersOrConnection::Connection(ref mut cm) = woc {
                // If a connection already exists, update it
                cm.update_info(peer_addr, mac_key, socket.into()).await;
                return;
            }

            let mut cm = ConnectionManager::new(connection_info);
            let c = cm.get_connection(socket.clone().into()).await;
            let old_woc = std::mem::replace(woc, WaitersOrConnection::Connection(cm));
            if let WaitersOrConnection::Waiters(mut v) = old_woc {
                for i in std::mem::take(&mut v) {
                    let mut state = i.lock().unwrap();
                    state.0 = Some(c.clone());
                    if let Some(w) = state.1.take() {
                        w.wake();
                    }
                }
            }
        }
    }
    pub async fn handle_net_message(
        &self,
        m: NetMessage,
        peer_addr: PeerAddr,
        own_entity: Entity,
        socket: SocketWriterBuilder,
        accept: impl Fn(PubSigKey, PeerAddr, Entity) -> bool,
    ) {
        match m {
            NetMessage::Merkle(s) => {
                let peer_id = s.who();
                if let Some((
                    (_contest_id, timestamp, peer_pkk, Obfuscated(_peer_addr_local), entity),
                    peer_id,
                )) = s.inner(&peer_id)
                {
                    if is_timestamp_valid(timestamp) && accept(peer_id, peer_addr, entity) {
                        self.finalize_connection(
                            socket.clone(),
                            own_entity,
                            peer_id,
                            peer_addr,
                            peer_pkk,
                        )
                        .await;
                    }
                }
            }
            NetMessage::KeepAlive(peer_id, macced) => {
                let Some(mac_key) =
                    self.wocs
                        .get_async(&peer_id)
                        .await
                        .and_then(|x| match x.get() {
                            WaitersOrConnection::Connection(cm) => Some(cm.1.mac_key),
                            _ => None,
                        })
                else {
                    return;
                };
                let Some(timestamp) = macced.inner_from_mac_key(&mac_key).await else {
                    return;
                };
                if is_timestamp_valid(timestamp.0) {
                    if let Some((_k, v)) = self.initting.remove_async(&(peer_id, peer_addr)).await {
                        v.1.abort();
                    }
                }
            }
        }
    }
}

async fn new_initting(
    socket: SocketWriterBuilder,
    own_entity: Entity,
    peer_addr: PeerAddr,
) -> (Option<SecKexKey>, AbortHandle) {
    let skk = SecKexKey::random_from_rng(thread_rng());
    let abort_handle = task::spawn(send_kex_loop(
        socket.into(),
        own_entity,
        (&skk).into(),
        peer_addr,
    ))
    .abort_handle();
    (Some(skk), abort_handle)
}

async fn send_kex_loop(
    mut socket: SocketWriter,
    entity: Entity,
    pkk: PubKexKey,
    peer_addr: PeerAddr,
) {
    let contest_id = socket.contest_id();
    let obf_addr = Obfuscated(socket.own_addr().unwrap());
    let ssk = socket.ssk();
    let psk = socket.psk();
    loop {
        let _ = socket
            .send_to(
                Message::Net(NetMessage::Merkle(Signed::new(
                    ((contest_id, SystemTime::now(), pkk, obf_addr, entity), psk),
                    &ssk,
                ))),
                peer_addr,
            )
            .await;
        let interval = thread_rng().gen_range(Duration::from_millis(40)..Duration::from_millis(400));
        sleep(interval).await;
    }
}
