use argh::FromArgs;
use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use net::*;
use std::sync::Arc;
use tokio::task;
use tracing::*;

#[derive(FromArgs)]
#[argh(description = "decipi")]
struct Args {
    #[argh(
        switch,
        short = 'w',
        description = "wether you are a worker or a participant"
    )]
    worker: bool,
    #[argh(option, description = "id of the contest to connect to")]
    contest_id: ContestId,
    #[argh(option, description = "server address for the contest to connect to")]
    server_addr: PeerAddr,
    #[argh(option, description = "public key for the contest to connect to")]
    server_psk: PubSigKey,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(Level::DEBUG)
        .init();
    debug!("starting");
    let args: Args = argh::from_env();

    // get signing keypair
    let entry = keyring::Entry::new("decipi", &whoami::username()).unwrap();
    let key = match entry.get_password() {
        Err(_) => {
            info!("generating new ed25519 keypair");
            let key = ed25519_dalek::SigningKey::generate(&mut rand::thread_rng());
            entry
                .set_password(
                    &key.to_pkcs8_pem(ed25519_dalek::pkcs8::spki::der::pem::LineEnding::default())
                        .unwrap(),
                )
                .unwrap();
            key
        }
        Ok(pkcs8) => ed25519_dalek::SigningKey::from_pkcs8_pem(&pkcs8).unwrap(),
    };

    let (mut socket_reader, socket_writer_builder) =
        new_socket("0.0.0.0:0", key, args.contest_id).await.unwrap();

    let own_entity = match args.worker {
        true => Entity::Worker,
        false => Entity::Participant,
    };

    let net = Arc::new(Net::new(
        socket_writer_builder.clone(),
        own_entity,
        |_, _, _| false,
    ));

    let server_connection: Connection = {
        let mnet = net.clone();
        let scjh =
            task::spawn(
                async move { mnet.get_connection(args.server_psk, args.server_addr).await },
            );

        while !scjh.is_finished() {
            let (message, peer_addr) = socket_reader.recv_from().await;
            if let Message::Net(m) = message {
                let mnet = net.clone();
                task::spawn(async move {
                    mnet.handle_net_message(m, peer_addr).await;
                });
            }
        }
        scjh.await.unwrap()
    };

    loop {
        let (message, peer_addr) = socket_reader.recv_from().await;
        match message {
            Message::Net(m) => {
                let mnet = net.clone();
                task::spawn(async move {
                    mnet.handle_net_message(m, peer_addr).await;
                });
            }
            Message::Queue(m) => {
                todo!()
            }
            Message::File(m) => {
                todo!()
            }
            Message::EncKey(m) => {
                todo!()
            }
            Message::Request(m) => {
                todo!()
            }
            _ => {}
        }
    }
}
