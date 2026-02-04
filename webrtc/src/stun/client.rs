//! STUN client for discovering reflexive addresses using Binding Requests.

use super::message::{MessageType, StunMessage};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

/// STUN client to send Binding Requests.
pub struct StunClient {
    pub default_server: String,
    pub timeout: Duration,
}

impl StunClient {
    /// Build a client pointing to the default public server.
    pub fn new() -> Self {
        Self {
            default_server: "stun.l.google.com:19302".to_string(),
            timeout: Duration::from_secs(5),
        }
    }

    /// Allows specify a STUN server other than the default one.
    pub fn with_server(server: String) -> Self {
        Self {
            default_server: server,
            timeout: Duration::from_secs(5),
        }
    }

    /// Perform a STUN query using the default server.
    pub fn query(
        &self,
        socket: &UdpSocket,
    ) -> Result<Option<SocketAddr>, Box<dyn std::error::Error>> {
        self.query_server(socket, &self.default_server)
    }

    /// Perform a STUN query against a specific server.
    pub fn query_server(
        &self,
        socket: &UdpSocket,
        server: &str,
    ) -> Result<Option<SocketAddr>, Box<dyn std::error::Error>> {
        // Create a Binding Request

        let request = StunMessage::create_binding_request();

        // send to server

        let resolved_addr = server
            .to_socket_addrs()?
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| std::io::Error::other("No IPv4 address found for STUN server"))?;

        socket.send_to(&request, resolved_addr)?;

        socket.set_read_timeout(Some(self.timeout))?;

        // wait for response

        let mut buf = [0u8; 1024];

        match socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let response = StunMessage::parse(&buf[..len])?;

                if response.message_type == MessageType::BindingResponse {
                    Ok(response.xor_mapped_address)
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Attempt to query multiple servers until a valid response is obtained.
    pub fn query_multiple(
        &self,
        socket: &UdpSocket,
        servers: &[String],
    ) -> Result<Option<SocketAddr>, Box<dyn std::error::Error>> {
        for server in servers {
            if let Ok(Some(addr)) = self.query_server(socket, server) {
                return Ok(Some(addr));
            }
        }
        Ok(None)
    }
}

impl Default for StunClient {
    /// Equivalente a llamar a [`StunClient::new`].
    fn default() -> Self {
        Self::new()
    }
}
