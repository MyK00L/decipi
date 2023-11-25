#![feature(lazy_cell)]
#![feature(ip_bits)]
#![allow(dead_code)]
mod connection;
mod init;
mod message;
mod socket;

pub use connection::Connection;
use init::*;
pub use message::*;
pub use socket::*;

pub struct Net {
    init_state: InitState,
    socket: SocketWriterBuilder,
    own_entity: Entity,
    ssk: SecSigKey,
    accept: fn(PubSigKey, PeerAddr, Entity) -> bool,
}
impl Net {
    pub fn new(
        socket: SocketWriterBuilder,
        own_entity: Entity,
        ssk: SecSigKey,
        accept: fn(PubSigKey, PeerAddr, Entity) -> bool,
    ) -> Self {
        Self {
            init_state: InitState::new(),
            socket,
            own_entity,
            ssk,
            accept,
        }
    }
    pub async fn get_connection(_peer_id: PubSigKey) -> Connection {
        todo!()
    }
}

#[cfg(test)]
mod tests {}
