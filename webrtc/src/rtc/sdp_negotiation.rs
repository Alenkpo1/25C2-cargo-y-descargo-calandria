//! SDP negotiation for RTC peer connection.

use std::str::FromStr;

use crate::ice::IceAgent;
use crate::protocols::sdp::session_description::SessionDescription;
use crate::sdp_helper::{ice_to_sdp, sdp_to_ice_candidates};

use super::peer_connection_error::PeerConnectionError;
use super::rtc_dtls::DtlsSession;

/// Process a remote SDP offer and extract ICE candidates.
/// 
/// Returns the extracted credentials (ufrag, pwd) and fingerprint.
pub fn process_remote_sdp(
    ice_agent: &mut IceAgent,
    sdp: &str,
) -> Result<(String, String, Option<String>), PeerConnectionError> {
    let remote_session = SessionDescription::from_str(sdp)
        .map_err(|err| PeerConnectionError::Sdp(err.to_string()))?;

    let (ufrag, pwd, candidates, fingerprint) =
        sdp_to_ice_candidates(&remote_session).map_err(PeerConnectionError::Sdp)?;

    for candidate in candidates {
        ice_agent.add_remote_candidate(candidate);
    }

    println!("DEBUG: Remote ICE candidates and credentials processed.");

    Ok((ufrag, pwd, fingerprint))
}

/// Build a local SDP description from the ICE agent state.
pub fn build_local_description(ice_agent: &IceAgent, dtls_session: Option<&DtlsSession>) -> String {
    let fingerprint = dtls_session.map(|s| s.certificate_fingerprint());
    let session = ice_to_sdp(ice_agent, fingerprint.as_deref());
    session.to_string()
}

/// Validate that the remote SDP contains a DTLS fingerprint.
pub fn validate_dtls_fingerprint(fingerprint: &Option<String>) -> Result<&str, PeerConnectionError> {
    fingerprint
        .as_deref()
        .ok_or_else(|| PeerConnectionError::Sdp("Remote SDP is missing DTLS fingerprint".to_string()))
}
