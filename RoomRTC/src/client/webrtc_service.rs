use std::sync::{Arc, Mutex};

use crate::client::p2p_client::P2PClient;
use room_rtc::rtc::rtc_peer_connection::{PeerConnectionError, PeerConnectionRole};

pub trait WebRTCHandler {
    fn client(&mut self) -> &mut Option<P2PClient>;
    fn role(&self) -> PeerConnectionRole;
    fn received_msgs(&self) -> &Arc<Mutex<Vec<String>>>;

    // Starts peer
    fn initialize_peer(&mut self) -> Result<(), PeerConnectionError> {
        if self.client().is_some() {
            return Ok(());
        }

        let client = P2PClient::new(self.role())?;
        *self.client() = Some(client);
        Ok(())
    }

    // Generates SDP offer
    fn generate_offer(&mut self) -> Result<String, PeerConnectionError> {
        let client = self
            .client()
            .as_mut()
            .ok_or_else(|| PeerConnectionError::Sdp("Client not initialized".into()))?;

        client.create_offer()
    }

    // Applies peer's sdp
    fn apply_remote_description(&mut self, sdp: &str) -> Result<(), PeerConnectionError> {
        let client = self
            .client()
            .as_mut()
            .ok_or_else(|| PeerConnectionError::Sdp("Client not initialized".into()))?;
        if let Err(e) = client.set_remote_description(sdp) {
            eprintln!("REMOTE DESCRIPTION ERROR: {}", e);
        };

        Ok(())
    }

    // Inicia el listener de mensajes (hilo)
    fn start_listener(
        &mut self,
        callback: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<(), PeerConnectionError> {
        let client = self
            .client()
            .as_mut()
            .ok_or_else(|| PeerConnectionError::Sdp("Client not initialized".into()))?;

        client.start_listener(callback)
    }

    // Sends a message to the other peer
    fn send_message(&mut self, msg: &str) -> Result<(), PeerConnectionError> {
        let client = self
            .client()
            .as_mut()
            .ok_or_else(|| PeerConnectionError::Sdp("Client not initialized".into()))?;

        client.send_msg(msg)
    }

    // Starts ice checks
    fn start_ice(&mut self) -> Result<(), PeerConnectionError> {
        let client = self
            .client()
            .as_mut()
            .ok_or_else(|| PeerConnectionError::Sdp("Client not initialized".into()))?;

        client.establish_connection()?; //Starts ICE and DTLS handshake

        // Also starts the listener
        let inbox = Arc::clone(self.received_msgs());
        self.start_listener(move |msg| {
            if let Ok(mut buffer) = inbox.lock() {
                buffer.push(msg);
            }
        })?;

        Ok(())
    }

    //Join meet screen only ///
    // Processes the remote sdp offer
    fn process_remote_offer(&mut self, remote_sdp: &str) -> Result<String, PeerConnectionError> {
        if remote_sdp.trim().is_empty() {
            
            eprintln!("Remote SDP is empty");
            return Err(PeerConnectionError::Sdp("Client not initialized".into()));
        }
        if let Some(client) = self.client().as_mut() {
            let answer = client.process_offer(remote_sdp)?;
            println!("JOIN MEET SCREEN: Offer processed successfully.");

            Ok(answer)
        } else {
            eprintln!("JOIN MEET SCREEN: Client not initialized");
            Err(PeerConnectionError::Sdp("Client not initialized".into()))
        }
    }
}
