//! `RTCPeerConnection` API based in ICE.

use std::net::SocketAddr;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc, Mutex};

use crate::crypto::srtp::SrtpContext;
use crate::ice::IceAgent;
use crate::rtc::rtc_dtls::{DtlsRole, DtlsSession};
use crate::rtc::socket::peer_socket::PeerSocket;
use crate::rtc::socket::peer_socket_err::PeerSocketErr;

pub use super::peer_connection_error::PeerConnectionError;
use super::sdp_negotiation::{build_local_description, process_remote_sdp, validate_dtls_fingerprint};
use crate::rtc::rtc_sctp::SctpAssociation;

/// Defines the role assumed by the peer within the signaling flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerConnectionRole {
    Controlling,
    Controlled,
}

impl PeerConnectionRole {
    pub fn is_controlling(self) -> bool {
        matches!(self, Self::Controlling)
    }
}

pub struct RtcPeerConnection {
    role: PeerConnectionRole,
    ice_agent: IceAgent,
    socket: Arc<Mutex<PeerSocket>>,
    local_description: Option<String>,
    remote_description: Option<String>,
    remote_credentials: Option<(String, String)>,
    host_candidate_registered: bool,
    listener_started: bool,
    srtp_context: Option<SrtpContext>,
    dtls_session: Option<DtlsSession>,
    dtls_receiver: Option<Receiver<Vec<u8>>>,
    dtls_sender: Option<mpsc::SyncSender<Vec<u8>>>,
    pub sctp_association: Option<SctpAssociation>,
}

impl RtcPeerConnection {
    pub fn new(
        local_addr: Option<&str>,
        role: PeerConnectionRole,
    ) -> Result<Self, PeerConnectionError> {
        let socket = Arc::new(Mutex::new(PeerSocket::new(local_addr)?));
        let ice_agent = match role {
            PeerConnectionRole::Controlling => IceAgent::new().set_controlling(true),
            PeerConnectionRole::Controlled => IceAgent::new(),
        };

        let dtls_role = match role {
            PeerConnectionRole::Controlling => DtlsRole::Client,
            PeerConnectionRole::Controlled => DtlsRole::Server,
        };
        let dtls_session = DtlsSession::new(dtls_role).ok();
        let (dtls_tx, dtls_rx) = mpsc::sync_channel(100);

        let sctp_association = Some(SctpAssociation::new(role == PeerConnectionRole::Controlled));

        Ok(Self {
            role,
            ice_agent,
            socket,
            local_description: None,
            remote_description: None,
            remote_credentials: None,
            host_candidate_registered: false,
            listener_started: false,
            srtp_context: None,
            dtls_receiver: Some(dtls_rx),
            dtls_sender: Some(dtls_tx),
            dtls_session,
            sctp_association,
        })
    }

    // ========== Basic accessors ==========

    /// Returns the role configured for this connection.
    pub fn role(&self) -> PeerConnectionRole {
        self.role
    }

    /// Gets the local address of the underlying socket.
    pub fn local_addr(&self) -> Result<SocketAddr, PeerConnectionError> {
        let socket = self
            .socket
            .lock()
            .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?;
        Ok(socket.local_addr())
    }

    /// Returns the learned remote address, if it exists.
    pub fn remote_addr(&self) -> Result<Option<SocketAddr>, PeerConnectionError> {
        let socket = self
            .socket
            .lock()
            .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?;
        Ok(socket.remote_addr())
    }

    /// Updates the remote address if it changed (e.g., after NAT rebinding).
    pub fn update_remote_addr(&mut self, new_addr: SocketAddr) {
        if let Ok(mut socket) = self.socket.lock() {
            socket.update_remote_addr(new_addr);
        }
    }

    pub fn media_socket(&self) -> Arc<Mutex<PeerSocket>> {
        Arc::clone(&self.socket)
    }

    /// Access the generated local description.
    pub fn local_description(&self) -> Option<&str> {
        self.local_description.as_deref()
    }

    /// Access the remote description received.
    pub fn remote_description(&self) -> Option<&str> {
        self.remote_description.as_deref()
    }

    /// Indicates whether there is a candidate pair selected by ICE.
    pub fn is_connected(&self) -> bool {
        self.ice_agent.has_connection()
    }

    /// Retrieves the ICE credentials announced by the remote peer.
    pub fn remote_credentials(&self) -> Option<(&str, &str)> {
        self.remote_credentials
            .as_ref()
            .map(|(ufrag, pwd)| (ufrag.as_str(), pwd.as_str()))
    }

    // ========== SDP Negotiation ==========

    /// Generate an SDP offer to start the negotiation as the controlling peer.
    pub fn create_offer(&mut self) -> Result<String, PeerConnectionError> {
        if !self.role.is_controlling() {
            return Err(PeerConnectionError::InvalidRole(
                "create_offer can only be used by a controlling peer",
            ));
        }

        self.ensure_host_candidate()?;
        let offer = build_local_description(&self.ice_agent, self.dtls_session.as_ref());
        self.local_description = Some(offer.clone());

        Ok(offer)
    }

    /// Processes a remote offer and constructs the corresponding SDP response.
    pub fn process_offer(&mut self, offer_sdp: &str) -> Result<String, PeerConnectionError> {
        if self.role.is_controlling() {
            return Err(PeerConnectionError::InvalidRole(
                "process_offer can only be used by a controlled peer",
            ));
        }

        self.ensure_host_candidate()?;

        let (ufrag, pwd, fingerprint) = process_remote_sdp(&mut self.ice_agent, offer_sdp)?;
        
        println!("SDP Offer:\n{}", offer_sdp);
        
        let fp = validate_dtls_fingerprint(&fingerprint)?;
        self.set_remote_dtls_fingerprint(fp)?;

        self.remote_description = Some(offer_sdp.to_string());
        self.remote_credentials = Some((ufrag, pwd));

        let answer = build_local_description(&self.ice_agent, self.dtls_session.as_ref());
        self.local_description = Some(answer.clone());

        Ok(answer)
    }

    /// Sets the remote description when acting as a controller peer.
    pub fn set_remote_description(&mut self, remote_sdp: &str) -> Result<(), PeerConnectionError> {
        if !self.role.is_controlling() {
            return Err(PeerConnectionError::InvalidRole(
                "set_remote_description can only be used by a controlling peer",
            ));
        }

        let (ufrag, pwd, fingerprint) = process_remote_sdp(&mut self.ice_agent, remote_sdp)?;

        let fp = validate_dtls_fingerprint(&fingerprint)?;
        self.set_remote_dtls_fingerprint(fp)?;

        self.remote_description = Some(remote_sdp.to_string());
        self.remote_credentials = Some((ufrag, pwd));

        Ok(())
    }

    // ========== ICE Connectivity ==========

    /// Start ICE checks and register the selected address in the socket.
    pub fn start_connectivity_checks(&mut self) -> Result<(), PeerConnectionError> {
        self.ensure_host_candidate()?;

        {
            let socket = self
                .socket
                .lock()
                .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?;
            self.ice_agent
                .start_connectivity_checks(socket.socket())
                .map_err(|err| PeerConnectionError::Ice(err.to_string()))?;
        }

        if let Some(pair) = self.ice_agent.get_selected_pair() {
            let remote_addr = format!(
                "{}:{}",
                pair.remote_candidate.address, pair.remote_candidate.port
            );

            self.socket
                .lock()
                .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?
                .add_remote_address(&remote_addr)
                .map_err(PeerConnectionError::Io)?;
        }

        Ok(())
    }

    /// Ensures that the ICE agent knows at least one host candidate.
    fn ensure_host_candidate(&mut self) -> Result<(), PeerConnectionError> {
        if self.host_candidate_registered {
            return Ok(());
        }

        let socket = self
            .socket
            .lock()
            .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?;

        self.ice_agent.register_host_candidate(socket.local_addr());
        self.ice_agent.gather_reflexive_candidates(socket.socket());
        self.host_candidate_registered = true;
        Ok(())
    }

    // ========== Data Transfer ==========

    /// Send data once the connection has chosen a valid ICE pair.
    pub fn send(&self, data: &[u8]) -> Result<(), PeerConnectionError> {
        self.socket
            .lock()
            .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?
            .send(data)
            .map_err(PeerConnectionError::from)
    }

    /// Gets the incoming message receiver associated with the peer.
    pub fn take_receiver(
        &mut self,
    ) -> Result<Receiver<(Vec<u8>, SocketAddr)>, PeerConnectionError> {
        self.ensure_listener_started()?;
        self.socket
            .lock()
            .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?
            .get_receiver()
            .map_err(PeerConnectionError::from)
    }

    /// Start the listening thread if it has not already been launched.
    pub fn ensure_listener_started(&mut self) -> Result<(), PeerConnectionError> {
        if !self.listener_started {
            let dtls_tx = self.dtls_sender.take().ok_or(PeerConnectionError::Dtls(
                "DTLS Sender already taken".to_string(),
            ))?;

            self.socket
                .lock()
                .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?
                .listener(Some(dtls_tx))?;

            self.listener_started = true;
        }
        Ok(())
    }

    // ========== SRTP ==========

    /// Configures the shared SRTP key (32 bytes).
    fn set_srtp_key(&mut self, key: &[u8]) {
        self.srtp_context = SrtpContext::new(key);
    }

    /// Returns the SRTP context if available.
    pub fn srtp_context(&self) -> Option<SrtpContext> {
        self.srtp_context.clone()
    }

    // ========== DTLS ==========

    /// Returns the local DTLS certificate fingerprint for SDP.
    pub fn dtls_fingerprint(&self) -> Option<String> {
        self.dtls_session
            .as_ref()
            .map(|s| s.certificate_fingerprint())
    }

    /// Sets the remote peer's DTLS fingerprint (extracted from remote SDP).
    pub fn set_remote_dtls_fingerprint(
        &mut self,
        fingerprint: &str,
    ) -> Result<(), PeerConnectionError> {
        if let Some(ref mut session) = self.dtls_session {
            session
                .set_remote_fingerprint(fingerprint)
                .map_err(|_| PeerConnectionError::Dtls("Invalid DTLS fingerprint".to_string()))
        } else {
            Err(PeerConnectionError::Dtls(
                "DTLS session not initialized".to_string(),
            ))
        }
    }

    /// DTLS handshake over the ready ICE connection.
    pub fn start_dtls_handshake(&mut self, _timeout_ms: u64) -> Result<(), PeerConnectionError> {
        if !self.is_connected() {
            return Err(PeerConnectionError::Ice(
                "No ICE connection established".to_string(),
            ));
        }

        let remote_addr = self
            .remote_addr()?
            .ok_or_else(|| PeerConnectionError::Ice("Remote address not set".to_string()))?;

        let socket_arc = {
            let peer_socket = self
                .socket
                .lock()
                .map_err(|_| PeerConnectionError::Socket(PeerSocketErr::PoisonedThread))?;

            let cloned_socket = peer_socket
                .socket()
                .try_clone()
                .map_err(PeerConnectionError::Io)?;

            Arc::new(Mutex::new(cloned_socket))
        };

        let dtls_rx = self.dtls_receiver.take().ok_or_else(|| {
            PeerConnectionError::Dtls(
                "DTLS receiver already consumed or not initialized".to_string(),
            )
        })?;

        if let Some(ref mut session) = self.dtls_session {
            session
                .perform_handshake(socket_arc, dtls_rx, remote_addr)
                .map_err(|e| PeerConnectionError::Dtls(e.to_string()))?;

            let key = session
                .export_srtp_keying_material(32)
                .map_err(|e| PeerConnectionError::Dtls(e.to_string()))?;

            self.set_srtp_key(&key);
            println!("DEBUG: SRTP key successfully exported from DTLS session.");

            Ok(())
        } else {
            Err(PeerConnectionError::Dtls(
                "DTLS session not available".to_string(),
            ))
        }
    }

    /// Checks if DTLS handshake is complete.
    pub fn is_dtls_connected(&self) -> bool {
        self.dtls_session
            .as_ref()
            .map(|s| s.is_handshake_complete())
            .unwrap_or(false)
            && self.srtp_context.is_some()
    }

    /// Returns whether a DTLS session object is present.
    pub fn has_dtls_session(&self) -> bool {
        self.dtls_session.is_some()
    }

    /// Read decrypted data from DTLS transport.
    pub fn dtls_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.dtls_session
            .as_mut()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "DTLS not connected"))?
            .read_data(buf)
    }

    /// Write encrypted data into DTLS transport.
    pub fn dtls_write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.dtls_session
            .as_mut()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "DTLS not connected"))?
            .write_data(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn controlling_peer_generates_offer() -> Result<(), PeerConnectionError> {
        let mut pc = RtcPeerConnection::new(Some("127.0.0.1:0"), PeerConnectionRole::Controlling)?;

        let offer = pc.create_offer()?;

        assert!(offer.contains("v=0"));
        assert!(pc.local_description().is_some());
        Ok(())
    }

    #[test]
    fn controlled_peer_process_offer_and_generates_answer() -> Result<(), PeerConnectionError> {
        let mut offerer =
            RtcPeerConnection::new(Some("127.0.0.1:0"), PeerConnectionRole::Controlling)?;
        let offer = offerer.create_offer()?;

        let mut answerer =
            RtcPeerConnection::new(Some("127.0.0.1:0"), PeerConnectionRole::Controlled)?;

        let answer = answerer.process_offer(&offer)?;

        assert!(answer.contains("v=0"));
        assert!(answerer.local_description().is_some());
        assert!(answerer.remote_description().is_some());

        offerer.set_remote_description(&answer)?;

        assert!(offerer.remote_description().is_some());
        Ok(())
    }

    #[test]
    fn dtls_handshake_integration_test() -> Result<(), PeerConnectionError> {
        let offerer_pc = Arc::new(Mutex::new(RtcPeerConnection::new(
            Some("0.0.0.0:8444"),
            PeerConnectionRole::Controlling,
        )?));
        let answerer_pc = Arc::new(Mutex::new(RtcPeerConnection::new(
            Some("0.0.0.0:8445"),
            PeerConnectionRole::Controlled,
        )?));
        println!("RTC PeerConnections created.");

        let offer = offerer_pc.lock().unwrap().create_offer()?;
        let answer = answerer_pc.lock().unwrap().process_offer(&offer)?;
        offerer_pc.lock().unwrap().set_remote_description(&answer)?;
        println!("SDP Offer/Answer exchanged.");

        offerer_pc.lock().unwrap().ensure_listener_started()?;
        answerer_pc.lock().unwrap().ensure_listener_started()?;

        offerer_pc.lock().unwrap().start_connectivity_checks()?;
        answerer_pc.lock().unwrap().start_connectivity_checks()?;

        println!("Waiting for ICE connection...");
        let mut attempts = 0;
        while !offerer_pc.lock().unwrap().is_connected()
            || !answerer_pc.lock().unwrap().is_connected()
        {
            thread::sleep(Duration::from_millis(100));
            attempts += 1;
            if attempts > 50 {
                panic!("ICE connection timed out");
            }
        }
        println!("ICE connection established!");

        let offerer_clone = Arc::clone(&offerer_pc);
        let answerer_clone = Arc::clone(&answerer_pc);

        let offerer_handle = thread::spawn(move || {
            offerer_clone
                .lock()
                .unwrap()
                .start_dtls_handshake(5000)
        });

        let answerer_handle = thread::spawn(move || {
            answerer_clone
                .lock()
                .unwrap()
                .start_dtls_handshake(5000)
        });

        let offerer_result = offerer_handle.join().unwrap();
        let answerer_result = answerer_handle.join().unwrap();

        assert!(
            offerer_result.is_ok(),
            "Offerer DTLS handshake failed: {:?}",
            offerer_result.err()
        );
        assert!(
            answerer_result.is_ok(),
            "Answerer DTLS handshake failed: {:?}",
            answerer_result.err()
        );

        let offerer_lock = offerer_pc.lock().unwrap();
        let answerer_lock = answerer_pc.lock().unwrap();

        assert!(
            offerer_lock.is_dtls_connected(),
            "Offerer DTLS is not connected"
        );
        assert!(
            answerer_lock.is_dtls_connected(),
            "Answerer DTLS is not connected"
        );

        assert!(
            offerer_lock.srtp_context().is_some(),
            "Offerer SRTP context is missing"
        );
        assert!(
            answerer_lock.srtp_context().is_some(),
            "Answerer SRTP context is missing"
        );

        Ok(())
    }
}
