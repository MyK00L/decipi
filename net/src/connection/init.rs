use crate::connection::*;
use crate::message::*;
use crate::socket::*;
use rand::{thread_rng, Rng};
use scc::{HashMap, HashSet};
use std::sync::LazyLock;
use std::task::Waker;
use std::time::SystemTime;
use tokio::task;
use tokio::task::AbortHandle;
use tokio::time::{sleep, Duration};

pub struct InitState {
    // this hashmap contains info about connecting peers
    // things are in this hashmap because you want to connect to them
    // and either:
    // - they did not send you their PubKexKey yet
    // - or they did not send you a correct KeepAlive message yet
    // once both these are satitsfied, the connection is considered established,
    // so you should abort sending your PubKexKey with the AbortHandle
    initting: HashMap<(PubSigKey, PeerAddr), (Option<SecKexKey>, AbortHandle)>,
    waiters: HashMap<PubSigKey, Vec<Waker>>,
    // This is needed to disregard excess messages that may come from a peer,
    // however if the PubKexKey is different, it is considered a request for a new connection
    done: HashSet<(PubSigKey, PubKexKey)>,
}
impl InitState {
    fn new() -> Self {
        Self {
            initting: HashMap::new(),
            waiters: HashMap::new(),
            done: HashSet::new(),
        }
    }
    pub async fn wait_for(&self, peer_id: PubSigKey, waker: Waker) {
        self.waiters
            .entry_async(peer_id)
            .await
            .or_insert(vec![])
            .get_mut()
            .push(waker);
    }
    async fn wake(&self, peer_id: PubSigKey) {
        if let Some((_k, v)) = self.waiters.remove_async(&peer_id).await {
            for i in v {
                i.wake();
            }
        }
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
        set_connection_info(connection_info, socket.into()).await;
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
