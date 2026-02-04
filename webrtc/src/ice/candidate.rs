//! Representations of local or remote ICE candidates.

/// ICE candidate with its basic properties and priority.
#[derive(Debug, Clone)]
pub struct IceCandidate {
    pub name: String,
    pub address: String,
    pub port: u32,
    pub candidate_type: CandidateType,
    pub priority: u32,
}

/// Types of candidates available during ICE negotiations.
#[derive(Debug, Clone, PartialEq)]
pub enum CandidateType {
    Host,
    Srflx,
    Relay,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_host_candidate() {
        let candidate = IceCandidate {
            name: "host-0".to_string(),
            address: "192.168.1.100".to_string(),
            port: 54321,
            candidate_type: CandidateType::Host,
            priority: 2130706431,
        };

        assert_eq!(candidate.name, "host-0");
        assert_eq!(candidate.address, "192.168.1.100");
        assert_eq!(candidate.port, 54321);
        assert_eq!(candidate.candidate_type, CandidateType::Host);
    }

    #[test]
    fn test_candidate_type_equality() {
        assert_eq!(CandidateType::Host, CandidateType::Host);
        assert_eq!(CandidateType::Srflx, CandidateType::Srflx);
        assert_ne!(CandidateType::Host, CandidateType::Srflx);
    }

    #[test]
    fn test_candidate_clone() {
        let original = IceCandidate {
            name: "test".to_string(),
            address: "127.0.0.1".to_string(),
            port: 8080,
            candidate_type: CandidateType::Host,
            priority: 100,
        };

        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.address, cloned.address);
        assert_eq!(original.port, cloned.port);
    }
}
