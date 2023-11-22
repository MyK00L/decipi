use crate::connection::*;
use crate::message::*;
use crate::socket::*;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use rand::{thread_rng, Rng};
use scc::{HashMap, HashSet};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::task::Waker;
use std::time::SystemTime;
use tokio::task;
use tokio::task::AbortHandle;
use tokio::time::{sleep, Duration};

enum WaitersOrConnection {
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
    // This is needed to disregard excess messages that may come from a peer,
    // however if the PubKexKey is different, it is considered a request for a new connection
    done: HashSet<(PubSigKey, PubKexKey)>,
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
    fn new(ww: Arc<Mutex<(Option<Connection>, Option<Waker>)>>) -> impl Future<Output=Connection> {
        Self(ww)
    }
}

impl InitState {
    fn new() -> Self {
        Self {
            initting: HashMap::new(),
            wocs: HashMap::new(),
            done: HashSet::new(),
        }
    }
    async fn get_connection(
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
            WaitersOrConnection::Connection(cm) => {return cm.get_connection(socket.into()).await;}
            WaitersOrConnection::Waiters(w) => {
    //Waiters(Vec<Arc<Mutex<(Option<Connection>, Option<Waker>)>>>),
                let ww = Arc::new(Mutex::new((None, None)));
                w.push(ww.clone());
                GetConnectionFuture::new(ww)
            }
        }.await
    }

    async fn finalize_connection(
        &self,
        socket: SocketWriterBuilder,
        own_entity: Entity,
        peer_id: PubSigKey,
        peer_addr: PeerAddr,
        peer_pkk: PubKexKey,
    ) {
        let skk = self
            .initting
            .entry_async((peer_id, peer_addr))
            .await
            .or_insert(new_initting(socket.clone(), own_entity, peer_addr).await)
            .get_mut()
            .0
            .take()
            .unwrap();
        let mac_key = MacKey::from(skk.diffie_hellman(&peer_pkk));
        let connection_info = ConnectionInfo {
            mac_key,
            peer_id,
            peer_addr,
        };
        let mut cm = ConnectionManager::new(connection_info);
        let c = cm.get_connection(socket.clone().into()).await;
        // TODO: understand why if i did
        // let woc = self.wocs...get_mut();
        // let old_woc = replace(woc,...);
        // the borrow checker gets angry, but it does not like this:
        let old_woc = std::mem::replace(
            self.wocs
                .entry_async(peer_id)
                .await
                .or_insert(WaitersOrConnection::Waiters(vec![]))
                .get_mut(),
            WaitersOrConnection::Connection(cm),
        );
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
    async fn init_connection(
        &self,
        socket: SocketWriterBuilder,
        own_entity: Entity,
        peer_id: PubSigKey,
        peer_addr: PeerAddr,
    ) {
        let _ = INIT_STATE
            .initting
            .insert_async(
                (peer_id, peer_addr),
                new_initting(socket, own_entity, peer_addr).await,
            )
            .await;
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

pub static INIT_STATE: LazyLock<InitState> = LazyLock::new(InitState::new);

async fn handle_init() {
    todo!()
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
                Message::Init(InitMessage::Merkle(Signed::new(
                    ((contest_id, SystemTime::now(), pkk, obf_addr, entity), psk),
                    &ssk,
                ))),
                peer_addr,
            )
            .await;
        let interval = thread_rng().gen_range(Duration::from_millis(20)..Duration::from_millis(80));
        sleep(interval).await;
    }
}
