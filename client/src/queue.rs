use crate::message::*;
use std::collections::HashMap;

struct State {
    messages: Vec<QueueMessage>,
    sub_info: HashMap<SubmissionId, SubmissionInfo>,
    problems: HashMap<ProblemId, QProblemDesc>,
}
impl State {
    fn next_id(&self) -> QueueMessageId {
        self.messages.len().into()
    }
    fn add_message(&mut self, message: QueueMessage) {
        assert_eq!(message.id as usize, self.messages.len());
        self.messages.push(message.clone());
    }
    fn get_submission_score(&self, id: SubmissionId) {
        todo!();
    }
}
struct SubmissionInfo {
    evaluates: Vec<(PubSigKey, Option<SubScore>, Option<DetailHash>, bool)>,
    majority: Option<DetailHash>,
    done: bool,
}
impl SubmissionInfo {
    fn new(evaluators: Vec<PubSigKey>) -> Self {
        SubmissionInfo {
            evaluates: evaluators.iter().map(|x| (*x, None, None, false)).collect(),
            majority: None,
            done: false,
        }
    }
    fn is_done(&self) -> bool {
        self.done
    }
    fn any_score(&self) -> Option<SubScore> {
        self.evaluates.iter().filter_map(|x| x.1).next()
    }
    fn add_score(&mut self, ev: QEvaluation) {
        if let Some(x) = self
            .evaluates
            .iter_mut()
            .find(|x| x.0 == ev.evaluation_id.evaluator)
        {
            x.1 = Some(ev.score);
            x.2 = Some(ev.detailhs_hash);
            x.3 = false;
        }
    }
    fn add_proof(&mut self, evp: QEvaluationProof) {
        if let Some(x) = self
            .evaluates
            .iter_mut()
            .find(|x| x.0 == evp.evaluation_id.evaluator)
        {
            if x.3 || x.1.is_none() || x.2.is_none() {
                // sanity checks
                return;
            }
            x.3 = true;
            let ev = QEvaluation {
                evaluation_id: evp.evaluation_id,
                score: x.1.unwrap(),
                detailhs_hash: x.2.take().unwrap(),
            };

            if evp.check(&ev) {
                x.2 = Some(evp.detailhs);
            }
        } else {
            return;
        }
        if self.evaluates.iter().all(|x| x.3) {
            // Search for an element that appears more than half of the times
            let mut max_el = None;
            let mut cnt = 0;
            for dh in self.evaluates.iter().map(|x| x.2) {
                if dh == max_el {
                    cnt += 1;
                } else if cnt == 0 {
                    max_el = dh;
                    cnt = 1;
                } else {
                    cnt -= 1;
                }
            }
            let max_freq = self.evaluates.iter().filter(|x| x.2 == max_el).count();
            if max_freq * 2 > self.evaluates.len() {
                self.majority = max_el;
            }
            self.done = true;
        }
    }
    fn final_score(&self) -> Option<SubScore> {
        self.majority.map(|maj| {
            self.evaluates
                .iter()
                .filter(|x| x.2 == Some(maj))
                .map(|x| x.1.unwrap())
                .next()
                .unwrap()
        })
    }
}
struct EvaluationInfo {
    evaluator: PubSigKey,
    score: Option<SubScore>,
}
