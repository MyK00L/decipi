#![feature(lazy_cell)]
#![feature(ip_bits)]
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
    accept: fn(PubSigKey, PeerAddr, Entity) -> bool,
}
impl Net {
    pub fn new(
        socket: SocketWriterBuilder,
        own_entity: Entity,
        accept: fn(PubSigKey, PeerAddr, Entity) -> bool,
    ) -> Self {
        Self {
            init_state: InitState::new(),
            socket,
            own_entity,
            accept,
        }
    }
    pub async fn get_connection(&self, peer_id: PubSigKey, peer_addr: PeerAddr) -> Connection {
        self.init_state
            .get_connection(self.socket.clone(), self.own_entity, peer_id, peer_addr)
            .await
    }
    pub async fn handle_net_message(&self, message: NetMessage, peer_addr: PeerAddr) {
        self.init_state
            .handle_net_message(
                message,
                peer_addr,
                self.own_entity,
                self.socket.clone(),
                self.accept,
            )
            .await;
    }
}

#[cfg(test)]
mod tests {}
