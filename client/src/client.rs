use net::*;
use std::sync::Arc;

pub struct Client {}

impl Client {
    pub fn new() -> Self {
        todo!()
    }
    pub async fn handle_queue_message(&self, net: Arc<Net>, m: QueueMessage, psk: PubSigKey) {
        todo!()
    }
    pub async fn handle_file_message(&self, net: Arc<Net>, m: FileMessage, psk: PubSigKey) {
        todo!()
    }
    pub async fn handle_request_message(&self, net: Arc<Net>, m: RequestMessage, psk: PubSigKey) {
        todo!()
    }
    pub async fn handle_enckey_message(&self, net: Arc<Net>, m: EncKeyInfo, psk: PubSigKey) {
        todo!()
    }
}
