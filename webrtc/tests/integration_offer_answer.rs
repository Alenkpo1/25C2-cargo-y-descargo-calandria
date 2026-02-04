use room_rtc::rtc::rtc_peer_connection::{PeerConnectionRole, RtcPeerConnection};
use std::net::UdpSocket;
use std::str::FromStr;

#[test]
fn offer_answer_roundtrip_sets_descriptions() {
    let mut offerer =
        RtcPeerConnection::new(Some("127.0.0.1:0"), PeerConnectionRole::Controlling).unwrap();
    let offer = offerer.create_offer().unwrap();

    let mut answerer =
        RtcPeerConnection::new(Some("127.0.0.1:0"), PeerConnectionRole::Controlled).unwrap();
    let answer = answerer.process_offer(&offer).unwrap();

    offerer.set_remote_description(&answer).unwrap();

    assert!(offerer.local_description().is_some());
    assert!(offerer.remote_description().is_some());
    assert!(answerer.local_description().is_some());
    assert!(answerer.remote_description().is_some());
}

#[test]
fn sdp_roundtrip_preserves_candidates() {
    use room_rtc::ice::IceAgent;
    use room_rtc::sdp_helper::{ice_to_sdp, sdp_to_ice_candidates};
    let mut agent = IceAgent::new();

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    agent.register_host_candidate(socket.local_addr().unwrap());

    let sdp = ice_to_sdp(&agent, None);
    let session = room_rtc::SessionDescription::from_str(&sdp.to_string()).unwrap();
    let candidates = sdp_to_ice_candidates(&session).unwrap();

    assert!(!candidates.2.is_empty(), "candidates should be present");
}
