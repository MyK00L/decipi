use net::*;
use std::sync::Arc;

pub struct Client {
    net: Arc<Net>,
    server_psk: PubSigKey,
}

impl Client {
    pub async fn new(
        server_psk: PubSigKey,
        server_addr: PeerAddr,
        contest_id: ContestId,
        entity: Entity,
        ssk: SecSigKey,
    ) -> Self {
        let net = Arc::new(Net::new(ssk, entity, contest_id, Filter {}).await);
        // connect to the server
        net.update_peer_addr(server_psk, server_addr).await;
        net.inc_keepalive(server_psk).await;
        Self { net, server_psk }
    }
    pub async fn recv(&self, buf: &mut [u8]) -> (RecvMessage, PubSigKey) {
        self.net.recv(self.server_psk, buf).await
    }
    pub async fn handle_queue_message(&self, m: QueueMessage, psk: PubSigKey) {
        todo!()
    }
    pub async fn handle_file_message(&self, m: FileMessage, psk: PubSigKey) {
        todo!()
    }
    pub async fn handle_request_message(&self, m: RequestMessage, psk: PubSigKey) {
        todo!()
    }
    pub async fn handle_enckey_message(&self, m: EncKeyInfo, psk: PubSigKey) {
        todo!()
    }
    //TODO: submit
    //TODO: question
}
