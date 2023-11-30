mod client;

use argh::FromArgs;
use client::*;
use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use net::*;
use std::sync::Arc;
use tokio::task;
use tracing::*;

#[derive(FromArgs)]
#[argh(description = "decipi")]
struct Args {
    #[argh(
        option,
        short = 'e',
        default = "Entity::Participant",
        description = "role in this contest, must be one of: worker, participant, spectator"
    )]
    entity: Entity,
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
    if args.entity == Entity::Server {
        panic!("This is the client executable, if you want to run a server, this is not what you want to run");
    }

    // get signing keypair
    let entry = keyring::Entry::new("decipi", &whoami::username()).unwrap();
    let ssk = match entry.get_password() {
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

    let client = Arc::new(Client::new());
    let net = Arc::new(Net::new(ssk, args.entity, args.contest_id, Filter {}).await);
    net.update_peer_addr(args.server_psk, args.server_addr)
        .await;
    net.inc_keepalive(args.server_psk).await;
    loop {
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let (m, psk) = net.recv(args.server_psk, &mut buf).await;
        match m {
            RecvMessage::Queue(qm) => {
                let c = client.clone();
                let n = net.clone();
                task::spawn(async move {
                    c.handle_queue_message(n, qm, psk).await;
                });
            }
            _ => todo!(),
        }
    }
}
