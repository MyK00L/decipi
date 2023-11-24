#![feature(lazy_cell)]
#![feature(ip_bits)]
#![allow(dead_code)]
mod connection;
mod message;
mod queue;
mod socket;

pub use connection::Connection;
pub use message::*;
use tokio::sync::mpsc;

pub struct Net {}
impl Net {
    pub fn new(server: PeerAddr, own_entity: Entity, sx: mpsc::Sender<()>) -> Self {
        todo!()
    }
    pub async fn get_connection(peer_id: PubSigKey) -> Connection {
        todo!()
    }
}

#[cfg(test)]
mod tests {}
