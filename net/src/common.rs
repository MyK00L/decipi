use crate::message::*;

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
    fn add_evaluation(&mut self, e: QEvaluation) {
        if matches!(self.state, EvaluationState::None) {
            self.state = EvaluationState::Provisional(e.score, e.detailhs_hash);
        }
    }
    fn add_evaluation_proof(&mut self, ep: QEvaluationProof) {
        if let EvaluationState::Provisional(score, hh) = self.state {
            if ep.hash() == hh {
                self.state = EvaluationState::Final(score, ep.detailhs);
            } else {
                self.state = EvaluationState::Failed;
            }
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
                EvaluationState::Provisional(s, _h) => Some(s),
                EvaluationState::Final(s, _h) => Some(s),
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
    pub fn add_evaluation(&mut self, e: QEvaluation) {
        if let Some(x) = self
            .0
            .iter_mut()
            .find(|x| x.evaluator == e.evaluation_id.evaluator)
        {
            x.add_evaluation(e);
        }
    }
    pub fn add_evaluation_proof(&mut self, ep: QEvaluationProof) {
        if let Some(x) = self
            .0
            .iter_mut()
            .find(|x| x.evaluator == ep.evaluation_id.evaluator)
        {
            x.add_evaluation_proof(ep);
        }
    }
}
