use crate::ice::{CandidateType, IceAgent, IceCandidate};
use crate::protocols::sdp::{
    address_type::AddressType, attribute::Attribute, media_description::MediaDescription,
    media_type::MediaType, net_type::NetType, origin::Origin, sdp_version::SdpVersion, session_description::SessionDescription, time::Time, transport_protocol::TransportProtocol, value_attribute::ValueAttribute
};

/// Generates an SDP session from ICE agent state and an optional DTLS fingerprint.
pub fn ice_to_sdp(ice_agent: &IceAgent, fingerprint: Option<&str>) -> SessionDescription {
    let version = SdpVersion::new(0);

    let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => dur.as_secs() as u32,
        Err(err) => {
            eprintln!("ice_to_sdp: clock error (using 0): {}", err);
            0
        }
    };

    let origin = Origin::new(
        "-".to_string(),
        timestamp,
        timestamp,
        NetType::In,
        AddressType::IP4,
        "0.0.0.0".to_string(),
    );

    let time = Time::new(0);

    let media_desc = MediaDescription::new(
        MediaType::Video,
        9,                         //dummy port
        TransportProtocol::RtpSavp, // Usar RTP/SAVP para indicar que se usará SRTP (RTP Seguro)
        vec![96],                   // dummy payload type
    );

    // ICE attributes

    let mut attributes = Vec::new();

    attributes.push(Attribute::new(
        None,
        Some(ValueAttribute::Group("BUNDLE 0".to_string())),
    ));
    attributes.push(Attribute::new(None, Some(ValueAttribute::MsidSemantic)));

    // ICE attributes
    attributes.push(Attribute::new(
        None,
        Some(ValueAttribute::IceUfrag(ice_agent.user_fragment.clone())),
    ));

    attributes.push(Attribute::new(
        None,
        Some(ValueAttribute::IcePwd(ice_agent.password.clone())),
    ));

    // DTLS fingerprint
    if let Some(fp) = fingerprint {
        attributes.push(Attribute::new(
            None,
            Some(ValueAttribute::Fingerprint("sha-256".to_string(), fp.to_string())),
        ));
    }

    // ICE candidates

    for (idx, candidate) in ice_agent.local_candidate.iter().enumerate() {
        let typ_str = match candidate.candidate_type {
            CandidateType::Host => "host",
            CandidateType::Srflx => "srflx",
            CandidateType::Relay => "relay",
        };

        attributes.push(Attribute::new(
            None,
            Some(ValueAttribute::Candidate {
                foundation: (idx + 1) as u32,
                component: 1,
                protocol: "UDP".to_string(),
                priority: candidate.priority,
                address: candidate.address.clone(),
                port: candidate.port,
                typ: typ_str.to_string(),
            }),
        ));
    }

    SessionDescription::new(version, origin, time, vec![media_desc], attributes)
}

// gets the ICE candidates of SessionDescription
pub fn sdp_to_ice_candidates(
    sdp: &SessionDescription,
) -> Result<(String, String, Vec<IceCandidate>, Option<String>), String> {
    let (ice_ufrag, ice_pwd) = sdp.get_ice_credentials()?;

    let candidates = sdp.get_ice_candidates();

    let fingerprint = sdp.get_fingerprint();

    if candidates.is_empty() {
        return Err("No ICE candidates found in the SDP".to_string());
    }

    Ok((ice_ufrag, ice_pwd, candidates, fingerprint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_ice_to_sdp_to_ice() {
        // Create a IceAgent
        let mut ice_agent = IceAgent::new();
        ice_agent.gather_candidates().unwrap();

        // Create dummy fingerprint
        let dummy_fingerprint = "a=fingerprint:sha-256 1F:2E:3D:4C:5B:6A";


        // Convert to SDP
        let sdp = ice_to_sdp(&ice_agent, Some(dummy_fingerprint));
        let sdp_string = sdp.to_string();

        println!("SDP generated:\n{}", sdp_string);

        // parse again
        let parsed_sdp = SessionDescription::from_str(&sdp_string).unwrap();

        // extract candidates
        let (ufrag, pwd, candidates,_) = sdp_to_ice_candidates(&parsed_sdp).unwrap();

        assert_eq!(ufrag, ice_agent.user_fragment);
        assert_eq!(pwd, ice_agent.password);
        assert_eq!(candidates.len(), ice_agent.local_candidate.len());
    }
    //WIP Hacer test con fingerprint

}
