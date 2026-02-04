//! ICE agent responsible for gathering candidates and performing connectivity checks.

use std::net::{SocketAddr, UdpSocket};

use super::candidate::{CandidateType, IceCandidate};
use super::connectivity::run_connectivity_checks;
use super::gathering::{calculate_priority, create_host_candidate, create_srflx_candidate, determine_local_ipv4};
use super::pair::{CandidatePair, CandidatePairState};
use crate::stun::StunClient;

/// ICE agent that handles candidate gathering and connectivity checks.
#[warn(dead_code)]
pub struct IceAgent {
    pub(crate) ice_rol: bool,
    pub(crate) user_fragment: String,
    pub(crate) password: String,
    pub local_candidate: Vec<IceCandidate>,
    pub(crate) remote_candidate: Vec<IceCandidate>,
    pub(crate) candidate_pairs: Vec<CandidatePair>,
    pub(crate) selected_pair: Option<CandidatePair>,

    stun_client: StunClient,
}

impl Default for IceAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl IceAgent {
    /// Create a new agent with a random fragment and password.
    pub fn new() -> Self {
        Self {
            ice_rol: false,
            user_fragment: Self::generate_random_string(8),
            password: Self::generate_random_string(24),
            local_candidate: Vec::new(),
            remote_candidate: Vec::new(),
            candidate_pairs: Vec::new(),
            selected_pair: None,
            stun_client: StunClient::new(),
        }
    }

    /// Discover local candidates (host and reflexive) using STUN when possible.
    pub fn gather_candidates(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let local_socket = UdpSocket::bind("0.0.0.0:0")?;
        let local_addr = local_socket.local_addr()?;
        let host_ip = determine_local_ipv4(&self.stun_client, local_addr.ip());

        let host_candidate = create_host_candidate(
            self.local_candidate.len(),
            host_ip.to_string(),
            local_addr.port() as u32,
        );

        println!(
            " OK Host: {}: {}",
            host_candidate.address, host_candidate.port
        );
        self.local_candidate.push(host_candidate);

        match self.stun_client.query(&local_socket) {
            Ok(Some(public_addr)) => {
                let srflx_candidate = create_srflx_candidate(
                    self.local_candidate.len(),
                    public_addr.ip().to_string(),
                    public_addr.port() as u32,
                );

                println!(
                    " OK Srflx: {}:{}",
                    srflx_candidate.address, srflx_candidate.port
                );
                self.local_candidate.push(srflx_candidate);
            }
            Ok(None) => println!("STUN dont return a direction"),
            Err(e) => println!("ERROR STUN: {}", e),
        }

        println!(
            "Gathering complete: {} candidates",
            self.local_candidate.len()
        );
        Ok(())
    }

    /// Add a remote candidate and generate all possible pairs with the local ones.
    pub fn add_remote_candidate(&mut self, candidate: IceCandidate) {
        println!(
            "Adding remote candidate: {}:{}",
            candidate.address, candidate.port
        );

        self.remote_candidate.push(candidate.clone());

        for local in &self.local_candidate {
            let pair = CandidatePair {
                local_candidate: local.clone(),
                remote_candidate: candidate.clone(),
                state: CandidatePairState::Waiting,
            };
            self.candidate_pairs.push(pair);
        }

        println!("   {} candidate pairs created", self.local_candidate.len());
    }

    /// Run connectivity checks on known peers.
    pub fn start_connectivity_checks(
        &mut self,
        socket: &UdpSocket,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match run_connectivity_checks(socket, &mut self.candidate_pairs, self.ice_rol)? {
            Some(pair) => {
                self.selected_pair = Some(pair);
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Sort the candidate pairs in descending order of priority.
    fn sort_candidate_pairs(&mut self) {
        super::connectivity::sort_pairs_by_priority(&mut self.candidate_pairs);
    }

    /// Calculate a candidate's priority according to the ICE specification.
    fn calculate_priority(&self, candidate_type: &CandidateType, local_pref: u32) -> u32 {
        calculate_priority(candidate_type, local_pref)
    }

    /// Generates pseudo-random identifiers for `ufrag` and password.
    fn generate_random_string(len: usize) -> String {
        use rand::Rng;

        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789";

        let mut rng = rand::thread_rng();

        (0..len)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Returns the candidate pair chosen after the checks.
    pub fn get_selected_pair(&self) -> Option<&CandidatePair> {
        self.selected_pair.as_ref()
    }

    /// Indicates whether the agent already has a verified pair.
    pub fn has_connection(&self) -> bool {
        self.selected_pair.is_some()
    }

    /// Configures whether the agent behaves as a controller or controlled.
    pub fn set_controlling(mut self, is_controlling: bool) -> Self {
        self.ice_rol = is_controlling;
        self
    }

    /// Ensure that the local address is registered as a host candidate.
    pub fn register_host_candidate(&mut self, addr: SocketAddr) {
        let ip = determine_local_ipv4(&self.stun_client, addr.ip());
        let address = ip.to_string();
        let port = addr.port() as u32;

        if self
            .local_candidate
            .iter()
            .any(|candidate| candidate.port == port && candidate.address == address)
        {
            return;
        }

        let host_candidate = create_host_candidate(self.local_candidate.len(), address, port);
        self.local_candidate.push(host_candidate);
    }

    /// Reuse an existing socket to attempt to obtain reflexive candidates.
    pub fn gather_reflexive_candidates(&mut self, socket: &UdpSocket) {
        match self.stun_client.query(socket) {
            Ok(Some(public_addr)) => {
                let already_present = self.local_candidate.iter().any(|candidate| {
                    candidate.address == public_addr.ip().to_string()
                        && candidate.port == public_addr.port() as u32
                        && candidate.candidate_type == CandidateType::Srflx
                });

                if !already_present {
                    let srflx_candidate = create_srflx_candidate(
                        self.local_candidate.len(),
                        public_addr.ip().to_string(),
                        public_addr.port() as u32,
                    );

                    println!(
                        " OK Srflx (re-use socket): {}:{}",
                        srflx_candidate.address, srflx_candidate.port
                    );
                    self.local_candidate.push(srflx_candidate);
                }
            }
            Ok(None) => {
                println!("STUN did not return a public address");
            }
            Err(err) => {
                println!("Error querying STUN: {}", err);
            }
        }
    }

    /// Access the `ufrag` generated for the ICE session.
    pub fn user_fragment(&self) -> &str {
        &self.user_fragment
    }

    /// Returns the password generated for ICE negotiation.
    pub fn password(&self) -> &str {
        &self.password
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ice_agent_creation() {
        let agent = IceAgent::new();

        assert_eq!(agent.ice_rol, false);
        assert_eq!(agent.user_fragment.len(), 8);
        assert_eq!(agent.password.len(), 24);
        assert_eq!(agent.local_candidate.len(), 0);
        assert_eq!(agent.remote_candidate.len(), 0);
    }

    #[test]
    fn test_user_fragment_is_unique() {
        let agent1 = IceAgent::new();
        let agent2 = IceAgent::new();

        assert_ne!(agent1.user_fragment, agent2.user_fragment);
    }

    #[test]
    fn test_password_is_unique() {
        let agent1 = IceAgent::new();
        let agent2 = IceAgent::new();

        assert_ne!(agent1.password, agent2.password);
    }

    #[test]
    fn test_calculate_priority_host() {
        let agent = IceAgent::new();
        let priority = agent.calculate_priority(&CandidateType::Host, 65535);

        let expected = (1 << 24) * 126 + (1 << 8) * 65535 + 255;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_calculate_priority_srflx() {
        let agent = IceAgent::new();
        let priority = agent.calculate_priority(&CandidateType::Srflx, 65535);

        let expected = (1 << 24) * 100 + (1 << 8) * 65535 + 255;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_host_priority_higher_than_srflx() {
        let agent = IceAgent::new();
        let host_priority = agent.calculate_priority(&CandidateType::Host, 65535);
        let srflx_priority = agent.calculate_priority(&CandidateType::Srflx, 65535);

        assert!(host_priority > srflx_priority);
    }

    #[test]
    fn test_gather_candidates_creates_host() {
        let mut agent = IceAgent::new();
        let result = agent.gather_candidates();

        assert!(result.is_ok());
        assert!(agent.local_candidate.len() >= 1);
        assert_eq!(agent.local_candidate[0].candidate_type, CandidateType::Host);
    }

    #[test]
    fn test_add_remote_candidate() {
        let mut agent = IceAgent::new();
        let _ = agent.gather_candidates();

        let remote = IceCandidate {
            name: "remote-host".to_string(),
            address: "192.168.2.100".to_string(),
            port: 60000,
            candidate_type: CandidateType::Host,
            priority: 2130706431,
        };

        agent.add_remote_candidate(remote);

        assert_eq!(agent.remote_candidate.len(), 1);
        assert!(agent.candidate_pairs.len() > 0);
    }

    #[test]
    fn test_has_connection() {
        let agent = IceAgent::new();
        assert!(!agent.has_connection());
    }

    #[test]
    fn test_connectivity_checks_no_pairs() -> Result<(), Box<dyn std::error::Error>> {
        let mut agent = IceAgent::new();
        let socket = UdpSocket::bind("127.0.0.1:0")?;

        match agent.start_connectivity_checks(&socket) {
            Ok(_) => panic!("Should fail without candidate pairs"),
            Err(err) => assert_eq!(err.to_string(), "No candidate pairs to check"),
        }
        Ok(())
    }
}
