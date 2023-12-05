use net::*;
use scc::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::task::AbortHandle;

#[derive(Default)]
struct QueueState {
    next_message_id: u32,
    //sub_info: std::collections::HashMap<SubmissionId, SubmissionInfo>,
    problems: std::collections::HashMap<ProblemId, QProblemDesc>,
}

pub struct Client {
    net: Arc<Net>,
    server_psk: PubSigKey,
    receiving_files: HashMap<(FileHash, PubSigKey), (SystemTime, AbortHandle)>,
    queue_buffer: HashMap<QueueMessageId, QueueMessage>,
    queue: Mutex<QueueState>,
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
        Self {
            net,
            server_psk,
            receiving_files: HashMap::new(),
            queue_buffer: HashMap::new(),
            queue: Mutex::new(QueueState::default()),
        }
    }
    pub async fn recv(&self, buf: &mut [u8]) -> (RecvMessage, PubSigKey) {
        self.net.recv(self.server_psk, buf).await
    }
    pub async fn handle_queue_message(&self, m: QueueMessage, psk: PubSigKey) {
        if self.queue_buffer.insert_async(m.id, m).await.is_ok() {
            let mut qs = self.queue.lock().await;
            while let Some(m) = self.queue_buffer.get_async(&qs.next_message_id).await {
                qs.next_message_id += 1;
                let m = m.get();
            }
        }
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
