use net::*;
use scc::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::task::AbortHandle;

enum EvaluationState {
    None,
    Provisional(SubScore, DetailHash),
    Final(SubScore, DetailHash),
    Failed,
}

struct SingleEvaluationInfo {
    evaluator: PubSigKey,
    state: EvaluationState,
}
impl SingleEvaluationInfo {
    fn new(psk: PubSigKey) -> Self {
        Self {
            evaluator: psk,
            state: EvaluationState::None,
        }
    }
}

pub enum EvaluationResultScore {
    None,
    Provisional(SubScore),
    Final(SubScore),
    Failed,
}

pub struct EvaluationInfo(Vec<SingleEvaluationInfo>);
impl EvaluationInfo {
    pub fn new(evaluators: Vec<PubSigKey>) -> Self {
        Self(
            evaluators
                .into_iter()
                .map(SingleEvaluationInfo::new)
                .collect(),
        )
    }
    fn provisional_score(&self) -> Option<SubScore> {
        self.0
            .iter()
            .filter_map(|x| match x.state {
                EvaluationState::Provisional(s, h) => Some(s),
                EvaluationState::Final(s, h) => Some(s),
                _ => None,
            })
            .next()
    }
    fn final_score(&self) -> Option<SubScore> {
        let mut maj = None;
        let mut cnt = 0;
        let v: Vec<Option<(SubScore, DetailHash)>> = self
            .0
            .iter()
            .map(|x| match x.state {
                EvaluationState::Final(s, h) => Some((s, h)),
                _ => None,
            })
            .collect();
        for s in v.iter() {
            if *s == maj {
                cnt += 1;
            } else if cnt == 0 {
                maj = *s;
            } else {
                cnt -= 1;
            }
        }
        if v.iter().filter(|x| **x == maj).count() * 2 > self.0.len() {
            maj.map(|x| x.0)
        } else {
            None
        }
    }
    pub fn is_done(&self) -> bool {
        self.0.iter().all(|x| {
            matches!(
                x.state,
                EvaluationState::Final(_, _) | EvaluationState::Failed
            )
        })
    }
    pub fn score(&self) -> EvaluationResultScore {
        match self.final_score() {
            Some(s) => EvaluationResultScore::Final(s),
            None => {
                if self.is_done() {
                    EvaluationResultScore::Failed
                } else {
                    match self.provisional_score() {
                        Some(s) => EvaluationResultScore::Provisional(s),
                        None => EvaluationResultScore::None,
                    }
                }
            }
        }
    }
}

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
