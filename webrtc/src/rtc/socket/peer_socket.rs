//! UDP socket with specific utilities for WebRTC traffic.

use crate::rtc::socket::peer_socket_err::PeerSocketErr;
use crate::stun::{MessageType, StunMessage};
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::thread::JoinHandle;

/// Encapsulates a UDP socket and the associated listening loop for an RTC peer.
pub struct PeerSocket {
    socket: UdpSocket,
    local_addr: SocketAddr,
    remote_addr: Option<SocketAddr>,
    handler: Vec<JoinHandle<()>>,
    receiver: Option<Receiver<(Vec<u8>, SocketAddr)>>,
}
impl PeerSocket {
    /// Creates and binds a UDP socket at the specified address.
    pub fn new(local_addr: Option<&str>) -> Result<PeerSocket, PeerSocketErr> {
        let bind_addr = local_addr.unwrap_or("0.0.0.0:0");
        let socket = UdpSocket::bind(bind_addr).map_err(PeerSocketErr::BindSocketError)?;
        let local_addr = socket
            .local_addr()
            .map_err(PeerSocketErr::SetLocalAddrError)?;
        Ok(PeerSocket {
            socket,
            local_addr,
            remote_addr: None,
            handler: vec![],
            receiver: None,
        })
    }

    /// Start a thread that receives packets and responds to incoming STUN requests.
    /// 
    /// Checks handle_stun_message to automatically respond to STUN Binding Requests.
    /// If it's not a STUN message now we look for the first byte to send the packet to DTLS or SRTP.
    pub fn listener(&mut self, dtls_sender: Option<SyncSender<Vec<u8>>>) -> Result<(), PeerSocketErr> {
        println!("DEBUG: Starting PeerSocket listener");
        let (tx, rx) = mpsc::channel();

        let socket = self
            .socket
            .try_clone()
            .map_err(PeerSocketErr::CloneSocketError)?;

        self.receiver = Some(rx);
        let handle = thread::spawn(move || {
            // Cambio: aumente el buffer a 1500 por tema MTU
            let mut buffer = [0u8; 1500];
            loop {
                match socket.recv_from(&mut buffer) {
                    Ok((size, src_addr)) => {
                        let data = buffer[..size].to_vec();
                        // First: check if it's a STUN message and handle iT
                        if Self::handle_stun_message(&socket, &data, src_addr) {
                            continue;
                        }

                        // Second: check if it's DTLS or SRTP
                        if let Some(first_byte) = data.first() {
                            // DTLS records start with bytes between 20 and 63
                            if *first_byte >= 20 && *first_byte <= 63 {
                                if let Some(ref d_tx) = dtls_sender {
                                    if let Err(e) = d_tx.send(data) {
                                        println!(
                                            "DEBUG: DTLS channel send failed ({}), keeping listener alive",
                                            e
                                        );
                                    }
                                }
                                continue;
                            }
                        }
                        // If it was not STUN nor DTLS, we send it back.
                        if let Err(e) = tx.send((data, src_addr)) {
                            println!(
                                "DEBUG: RTP/RTCP channel closed ({}), dropping packet but listener stays alive",
                                e
                            );
                            continue;
                        }


                    }
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::Interrupted => continue,
                        std::io::ErrorKind::WouldBlock => {
                            // Socket no tenía datos listos, seguimos escuchando
                            continue;
                        }
                        _ => {
                            println!("DEBUG: PeerSocket listener recv_from error: {}", err);
                            break;
                        }
                    },
                }
            }
            println!("DEBUG: PeerSocket listener exiting");
        });
        self.handler.push(handle);
        Ok(())
    }
    
    /// Declares the remote address with which traffic exchange will be attempted.
    pub fn add_remote_address(&mut self, remote_addr_str: &str) -> std::io::Result<()> {
        let addr: SocketAddr = remote_addr_str
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        self.remote_addr = Some(addr);
        Ok(())
    }

    /// Send data to the registered remote address.
    pub fn send(&self, data: &[u8]) -> Result<(), PeerSocketErr> {
        if let Some(addr) = self.remote_addr {
            self.socket
                .send_to(data, addr)
                .map_err(PeerSocketErr::SendError)?;
            Ok(())
        } else {
            Err(PeerSocketErr::NotConnectedSocket)
        }
    }

    /// Returns the receiver channel associated with the listener thread.
    pub fn get_receiver(&mut self) -> Result<Receiver<(Vec<u8>, SocketAddr)>, PeerSocketErr> {
        if let Some(receiver) = self.receiver.take() {
            Ok(receiver)
        } else {
            Err(PeerSocketErr::NotConnectedSocket)
        }
    }

    /// Returns the local address of the socket.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Retrieves the remote address if it has already been established
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Indicates whether there is an associated remote address.
    pub fn is_connected(&self) -> bool {
        self.remote_addr.is_some()
    }

    /// Direct access to the underlying socket.
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    /// Automatically responds to STUN Binding Request messages.
    fn handle_stun_message(socket: &UdpSocket, data: &[u8], src_addr: SocketAddr) -> bool {
        if data.len() < 20 {
            return false;
        }

        match StunMessage::parse(data) {
            Ok(message) => match message.message_type {
                MessageType::BindingRequest => {
                    let response =
                        StunMessage::create_binding_success(message.transaction_id, src_addr);
                    let _ = socket.send_to(&response, src_addr);
                    true
                }
                MessageType::BindingResponse => true,
                _ => false,
            },
            Err(_) => false,
        }
    }
}
