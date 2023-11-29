use crate::message::*;
use anyhow::Result;
use speedy::{Readable, Writable};
use std::sync::Arc;
use tokio::net::{ToSocketAddrs, UdpSocket};

#[derive(Debug)]
pub struct SocketReader {
    socket: Arc<UdpSocket>,
    entity: Entity,
    ssk: SecSigKey,
    contest_id: ContestId,
}
impl SocketReader {
    pub async fn recv_from(&self, buf: &mut [u8]) -> (Message, PeerAddr) {
        loop {
            let Ok((length, addr)) = self.socket.recv_from(buf).await else {
                continue;
            };
            let Ok(message) = Message::read_from_buffer(&buf[0..length]) else {
                continue;
            };
            return (message, addr.into());
        }
    }
    pub fn entity(&self) -> Entity {
        self.entity
    }
    pub fn ssk(&self) -> SecSigKey {
        self.ssk.clone()
    }
    pub fn psk(&self) -> PubSigKey {
        (&self.ssk).into()
    }
    pub fn own_addr(&self) -> Result<PeerAddr> {
        Ok(PeerAddr::from(self.socket.local_addr()?))
    }
    pub fn contest_id(&self) -> ContestId {
        self.contest_id
    }
}

#[derive(Debug, Clone)]
pub struct SocketWriter {
    socket: Arc<UdpSocket>,
    entity: Entity,
    ssk: SecSigKey,
    contest_id: ContestId,
}
impl SocketWriter {
    pub async fn send_to(&self, message: Message, addr: PeerAddr, buf: &mut [u8]) -> Result<()> {
        message.write_to_buffer(buf).unwrap();
        self.socket
            .send_to(buf, std::net::SocketAddr::from(addr))
            .await?;
        Ok(())
    }
    pub fn entity(&self) -> Entity {
        self.entity
    }
    pub fn ssk(&self) -> SecSigKey {
        self.ssk.clone()
    }
    pub fn psk(&self) -> PubSigKey {
        (&self.ssk).into()
    }
    pub fn own_addr(&self) -> Result<PeerAddr> {
        Ok(PeerAddr::from(self.socket.local_addr()?))
    }
    pub fn contest_id(&self) -> ContestId {
        self.contest_id
    }
}

pub async fn new_socket<T: ToSocketAddrs>(
    addr: T,
    entity: Entity,
    ssk: SecSigKey,
    contest_id: ContestId,
) -> Result<(SocketReader, SocketWriter)> {
    let socket = Arc::new(UdpSocket::bind(addr).await?);
    let sr = SocketReader {
        socket: socket.clone(),
        entity,
        ssk: ssk.clone(),
        contest_id,
    };
    let sw = SocketWriter {
        socket: socket.clone(),
        entity,
        ssk,
        contest_id,
    };
    Ok((sr, sw))
}
