use crate::connection::*;
use crate::message::*;
use crate::socket::*;
use std::collections::{HashMap, HashSet};

use tokio::task::AbortHandle;

struct InitState {
    // this hashmap contains info about connecting peers
    // things are in this hashmap because you want to connect to them
    // and either:
    // - they did not send you their PubKexKey yet
    // - or they did not send you a correct KeepAlive message yet
    // once both these are satitsfied, the connection is considered established,
    // so you should abort sending your PubKexKey with the AbortHandle
    initting: HashMap<(PubSigKey, PeerAddr), (SecKexKey, AbortHandle)>,
    // This is needed to disregard excess messages that may come from a peer,
    // however if the PubKexKey is different, it is considered a request for a new connection
    done: HashSet<(PubSigKey, PubKexKey)>,
}

async fn handle_init() {
    todo!()
}

async fn init_connection(_socket: SocketReader) -> Connection {
    todo!()
}
