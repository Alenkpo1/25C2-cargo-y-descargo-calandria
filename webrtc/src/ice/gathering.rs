//! Candidate gathering functionality for ICE agent.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};

use super::candidate::{CandidateType, IceCandidate};
use crate::stun::StunClient;

/// Trait for gathering ICE candidates.
pub trait CandidateGathering {
    /// Discover local candidates (host and reflexive) using STUN when possible.
    fn gather_candidates(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Reuse an existing socket to attempt to obtain reflexive candidates.
    fn gather_reflexive_candidates(&mut self, socket: &UdpSocket);
    
    /// Ensure that the local address is registered as a host candidate.
    fn register_host_candidate(&mut self, addr: SocketAddr);
}

/// Helper functions for candidate gathering.
pub(crate) fn determine_local_ipv4(stun_client: &StunClient, fallback: IpAddr) -> IpAddr {
    match fallback {
        IpAddr::V4(ipv4) if !ipv4.is_unspecified() => IpAddr::V4(ipv4),
        _ => probe_default_ipv4(stun_client).unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
    }
}

/// Attempt to determine the primary interface by performing a synthetic connection.
pub(crate) fn probe_default_ipv4(stun_client: &StunClient) -> Option<IpAddr> {
    let pick_target = |address: &str| -> Option<SocketAddr> {
        address
            .to_socket_addrs()
            .ok()
            .and_then(|mut iter| iter.find(|candidate| candidate.is_ipv4()))
    };

    let target =
        pick_target(&stun_client.default_server).or_else(|| pick_target("8.8.8.8:80"))?;

    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(target).ok()?;
    socket.local_addr().ok().map(|addr| addr.ip())
}

/// Calculate a candidate's priority according to the ICE specification.
pub fn calculate_priority(candidate_type: &CandidateType, local_pref: u32) -> u32 {
    let type_pref = match candidate_type {
        CandidateType::Host => 126,
        CandidateType::Srflx => 100,
        CandidateType::Relay => 0,
    };

    (1 << 24) * type_pref + (1 << 8) * local_pref + (256 - 1)
}

/// Create a host candidate from the given address.
pub fn create_host_candidate(
    idx: usize,
    address: String,
    port: u32,
) -> IceCandidate {
    IceCandidate {
        name: format!("host-{}", idx),
        address,
        port,
        candidate_type: CandidateType::Host,
        priority: calculate_priority(&CandidateType::Host, 65535),
    }
}

/// Create a server-reflexive candidate from the given address.
pub fn create_srflx_candidate(
    idx: usize,
    address: String,
    port: u32,
) -> IceCandidate {
    IceCandidate {
        name: format!("srflx-{}", idx),
        address,
        port,
        candidate_type: CandidateType::Srflx,
        priority: calculate_priority(&CandidateType::Srflx, 65535),
    }
}
