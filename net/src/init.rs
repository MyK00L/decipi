use crate::connection::*;
use crate::message::*;
use crate::socket::*;
use dashmap::{DashMap, DashSet};
use rand::{thread_rng, Rng};
use std::sync::LazyLock;
use std::time::SystemTime;
use tokio::task;
use tokio::task::AbortHandle;
use tokio::time::{sleep, Duration};

struct InitState {
    // this hashmap contains info about connecting peers
    // things are in this hashmap because you want to connect to them
    // and either:
    // - they did not send you their PubKexKey yet
    // - or they did not send you a correct KeepAlive message yet
    // once both these are satitsfied, the connection is considered established,
    // so you should abort sending your PubKexKey with the AbortHandle
    initting: DashMap<(PubSigKey, PeerAddr), (SecKexKey, AbortHandle)>,
    // This is needed to disregard excess messages that may come from a peer,
    // however if the PubKexKey is different, it is considered a request for a new connection
    done: DashSet<(PubSigKey, PubKexKey)>,
}
impl InitState {
    fn new() -> Self {
        Self {
            initting: DashMap::new(),
            done: DashSet::new(),
        }
    }
}

static INIT_STATE: LazyLock<InitState> = LazyLock::new(InitState::new);

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
/// This should only be called from [connection](crate::connection)
/// if you need a new connection, call [get_connection](crate::connection::get_connection)
pub async fn init_connection(
    socket: SocketWriterBuilder,
    own_entity: Entity,
    peer_id: PubSigKey,
    peer_addr: PeerAddr,
) {
    let skk = SecKexKey::random_from_rng(thread_rng());
    let abort_handle = task::spawn(send_kex_loop(
        socket.into(),
        own_entity,
        (&skk).into(),
        peer_addr,
    ))
    .abort_handle();
    INIT_STATE
        .initting
        .insert((peer_id, peer_addr), (skk, abort_handle));
}
