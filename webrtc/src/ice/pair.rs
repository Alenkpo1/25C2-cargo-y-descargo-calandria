//! ICE pairs that combine local and remote candidates.
//!
use super::candidate::IceCandidate;

/// Candidate pair generated from local-remote combinations.
#[derive(Debug, Clone)]
pub struct CandidatePair {
    pub local_candidate: IceCandidate,
    pub remote_candidate: IceCandidate,
    pub state: CandidatePairState,
}

/// Possible states during the life cycle of an ICE pair.
#[derive(Debug, Clone, PartialEq)]
pub enum CandidatePairState {
    Waiting,
    InProgress,
    Succeeded,
    Failed,
}
