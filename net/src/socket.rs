use crate::message::*;
use anyhow::Result;
use speedy::{Readable, Writable};
use std::sync::Arc;
use tokio::net::{ToSocketAddrs, UdpSocket};

#[derive(Debug)]
pub struct SocketReader {
    socket: Arc<UdpSocket>,
    psk: PubSigKey,
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
    pub fn psk(&self) -> PubSigKey {
        self.psk
    }
}

#[derive(Debug)]
pub struct SocketWriter<const N: usize = MAX_MESSAGE_SIZE> {
    socket: Arc<UdpSocket>,
    psk: PubSigKey,
    buf: [u8; N],
}
impl<const N: usize> Clone for SocketWriter<N> {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.clone(),
            psk: self.psk,
            buf: [0u8; N],
        }
    }
}
pub struct SocketWriterBuilder {
    socket: Arc<UdpSocket>,
    psk: PubSigKey,
}
impl<const N: usize> SocketWriter<N> {
    pub async fn send_to(&mut self, message: Message, addr: PeerAddr) -> Result<()> {
        message.write_to_buffer(&mut self.buf)?;
        self.socket
            .send_to(&self.buf, std::net::SocketAddr::from(addr))
            .await?;
        Ok(())
    }
    pub fn psk(&self) -> PubSigKey {
        self.psk
    }
}
impl<const N: usize> From<SocketWriterBuilder> for SocketWriter<N> {
    fn from(swb: SocketWriterBuilder) -> Self {
        Self {
            socket: swb.socket,
            psk: swb.psk,
            buf: [0u8; N],
        }
    }
}

pub async fn new_socket<T: ToSocketAddrs>(
    addr: T,
    psk: PubSigKey,
) -> Result<(SocketReader, SocketWriterBuilder)> {
    let socket = Arc::new(UdpSocket::bind(addr).await?);
    let sr = SocketReader {
        socket: socket.clone(),
        psk,
        buf: [0u8; MAX_MESSAGE_SIZE],
    };
    let sw = SocketWriterBuilder {
        socket: socket.clone(),
        psk,
    };
    Ok((sr, sw))
}
