#![allow(unused)]
#![feature(ip_bits)]
mod message;
mod network_manager;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn stuff() {
        use std::{io, net::SocketAddr, sync::Arc};
        use tokio::{net::UdpSocket, sync::mpsc};
        let sock = UdpSocket::bind("0.0.0.0:8080".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let r = Arc::new(sock);
        let s = r.clone();
        let r2 = r.clone();

        tokio::spawn(async move {
            loop {
                let len = s.send_to(&[42u8; 16], "127.0.0.1:8080").await.unwrap();
                println!("{:?} bytes sent", len);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        let mut buf = [0; 1024];

        tokio::spawn(async move {
            loop {
                let (len, addr) = r.recv_from(&mut buf).await.unwrap();
                println!("1: {:?} bytes received from {:?}", len, addr);
                tokio::time::sleep(tokio::time::Duration::from_millis(1010)).await;
            }
        });

        let mut buf2 = [42; 1024];
        eprintln!("sent");
        loop {
            let (len, addr) = r2.recv_from(&mut buf).await.unwrap();
            println!("2: {:?} bytes received from {:?}", len, addr);
            tokio::time::sleep(tokio::time::Duration::from_millis(1010)).await;
        }
    }
    #[test]
    fn it_works() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(stuff());
    }
}
