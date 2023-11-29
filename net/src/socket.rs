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
    buf: [u8; MAX_MESSAGE_SIZE],
}
impl SocketReader {
    pub async fn recv_from(&mut self) -> (Message, PeerAddr) {
        loop {
            let Ok((length, addr)) = self.socket.recv_from(&mut self.buf).await else {
                continue;
            };
            let Ok(message) = Message::read_from_buffer(&self.buf[0..length]) else {
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

#[derive(Debug)]
pub struct SocketWriter<const N: usize = MAX_MESSAGE_SIZE> {
    socket: Arc<UdpSocket>,
    entity: Entity,
    ssk: SecSigKey,
    contest_id: ContestId,
    buf: [u8; N],
}
impl<const N: usize> Clone for SocketWriter<N> {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.clone(),
            entity: self.entity,
            ssk: self.ssk.clone(),
            contest_id: self.contest_id,
            buf: [0u8; N],
        }
    }
}
impl<const N: usize> SocketWriter<N> {
    pub async fn send_to(&mut self, message: Message, addr: PeerAddr) -> Result<()> {
        message.write_to_buffer(&mut self.buf).unwrap();
        self.socket
            .send_to(&self.buf, std::net::SocketAddr::from(addr))
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
impl<const N: usize> From<SocketWriterBuilder> for SocketWriter<N> {
    fn from(swb: SocketWriterBuilder) -> Self {
        Self {
            socket: swb.socket,
            entity: swb.entity,
            ssk: swb.ssk,
            contest_id: swb.contest_id,
            buf: [0u8; N],
        }
    }
}
#[derive(Debug, Clone)]
pub struct SocketWriterBuilder {
    socket: Arc<UdpSocket>,
    ssk: SecSigKey,
    entity: Entity,
    contest_id: ContestId,
}
impl SocketWriterBuilder {
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
) -> Result<(SocketReader, SocketWriterBuilder)> {
    let socket = Arc::new(UdpSocket::bind(addr).await?);
    let sr = SocketReader {
        socket: socket.clone(),
        entity,
        ssk: ssk.clone(),
        contest_id,
        buf: [0u8; MAX_MESSAGE_SIZE],
    };
    let sw = SocketWriterBuilder {
        socket: socket.clone(),
        entity,
        ssk,
        contest_id,
    };
    Ok((sr, sw))
}
