use crate::connection::*;
use crate::message::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::task::AbortHandle;

struct InitState {
    // hashmap with things in phase 1
    a: HashMap<(PubSigKey, PeerAddr), (SecKexKey, AbortHandle)>,
    // hashmap with things done
}

async fn handle_init() {
    todo!()
}

async fn init_connection(socket: Arc<UdpSocket>) -> Connection {
    todo!()
}
