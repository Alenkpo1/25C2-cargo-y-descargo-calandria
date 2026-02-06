use sctp_proto::{
    Association, AssociationHandle, ClientConfig, DatagramEvent, Endpoint, EndpointConfig,
    Payload, PayloadProtocolIdentifier, ServerConfig, Transmit, TransportConfig,
};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use bytes::Bytes;

pub struct SctpAssociation {
    endpoint: Endpoint,
    association: Option<Association>,
    association_handle: Option<AssociationHandle>,
    incoming_data: VecDeque<(u16, Vec<u8>)>,
    outgoing_queue: VecDeque<Vec<u8>>,
    is_server: bool,
}

impl SctpAssociation {
    pub fn new(is_server: bool) -> Self {
        // Minimal endpoint configuration for experimentation.
        let endpoint_config = Arc::new(EndpointConfig::default());

        let server_config = is_server.then(|| {
            let mut sc = ServerConfig::default();
            let mut tc = TransportConfig::default();
            tc.max_inbound_streams = 16;
            tc.max_initial_outgoing_streams = 16;
            sc.transport = Arc::new(tc);
            Arc::new(sc)
        });

        let endpoint = Endpoint::new(endpoint_config, server_config);

        Self {
            endpoint,
            association: None,
            association_handle: None,
            incoming_data: VecDeque::new(),
            outgoing_queue: VecDeque::new(),
            is_server,
        }
    }

    pub fn establish(&mut self) {
        if !self.is_server {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
            let mut client_config = ClientConfig::default();
            let mut tc = TransportConfig::default();
            tc.max_inbound_streams = 16;
            tc.max_initial_outgoing_streams = 16;
            client_config.transport = Arc::new(tc);

            if let Ok((handle, association)) = self.endpoint.connect(client_config, addr) {
                self.association_handle = Some(handle);
                self.association = Some(association);
                self.pump_association(Instant::now()); // queue INIT
            }
        }
    }

    pub fn handle_input(&mut self, packet: &[u8]) {
        let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
        let local_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let payload = Bytes::copy_from_slice(packet);
        if let Some((handle, event)) =
            self.endpoint
                .handle(Instant::now(), remote_addr, Some(local_ip), None, payload)
        {
            self.association_handle.get_or_insert(handle);
            match event {
                DatagramEvent::NewAssociation(association) => {
                    self.association = Some(association);
                }
                DatagramEvent::AssociationEvent(assoc_event) => {
                    if let Some(assoc) = self.association.as_mut() {
                        assoc.handle_event(assoc_event);
                    }
                }
            }
            self.pump_association(Instant::now());
        }
    }

    pub fn poll_output(&mut self) -> Option<Vec<u8>> {
        if let Some(buf) = self.outgoing_queue.pop_front() {
            return Some(buf);
        }

        if let Some(transmit) = self.endpoint.poll_transmit() {
            return self.take_transmit(transmit);
        }

        None
    }
    
    fn handle_event(&mut self, _event: DatagramEvent) {
        // Placeholder kept for backward compatibility.
    }

    pub fn send_data(&mut self, stream_id: u16, payload: Vec<u8>) -> Result<(), String> {
        {
            let assoc = self
                .association
                .as_mut()
                .ok_or_else(|| "Association not established".to_string())?;

            let mut stream = match assoc.stream(stream_id) {
                Ok(s) => s,
                Err(_) => assoc
                    .open_stream(stream_id, PayloadProtocolIdentifier::Binary)
                    .map_err(|e| e.to_string())?,
            };

            let mut offset = 0;
            while offset < payload.len() {
                match stream.write(&payload[offset..]) {
                    Ok(n) => {
                        offset += n;
                        if n == 0 {
                            return Err("BufferFull".to_string());
                        }
                    }
                    Err(e) => {
                        println!("DEBUG: SCTP send error on stream {}: {:?}", stream_id, e);
                        return Err(e.to_string());
                    }
                }
            }
        }

        self.pump_association(Instant::now());
        Ok(())
    }

    pub fn recv_data(&mut self) -> Option<(u16, Vec<u8>)> {
        // Events are handled in handle_input
        self.incoming_data.pop_front()
    }

    fn take_transmit(&mut self, transmit: Transmit) -> Option<Vec<u8>> {
        match transmit.payload {
            Payload::RawEncode(chunks) => {
                let mut iter = chunks.into_iter();
                if let Some(first) = iter.next() {
                    for chunk in iter {
                        self.outgoing_queue.push_back(chunk.to_vec());
                    }
                    Some(first.to_vec())
                } else {
                    None
                }
            }
            Payload::PartialDecode(_) => None,
        }
    }

    /// Drive association -> endpoint -> association event flow and queue outgoing datagrams.
    fn pump_association(&mut self, now: Instant) {
        loop {
            let mut progressed = false;
            let mut pending_transmits: Vec<Transmit> = Vec::new();
            let mut pending_events: Vec<sctp_proto::Event> = Vec::new();
            let mut endpoint_events: Vec<(AssociationHandle, sctp_proto::EndpointEvent)> = Vec::new();

            // Scope for mutable borrow of self.association
            if let Some(assoc) = self.association.as_mut() {
                // Poll for anything that needs to go to the Endpoint
                while let Some(ep_event) = assoc.poll_endpoint_event() {
                    // We can't access self.endpoint here if assoc is borrowed?
                    // assoc borrows self.association. self.endpoint is distinct.
                    // However, we need self.association_handle too.
                    // To be safe, let's collect these too, or handle them if disjoint.
                    // Compiler is smart enough for disjoint fields:
                    // self.association (mut), self.endpoint (mut), self.association_handle (immut)
                    // This block MIGHT be fine if fields are disjoint.
                    // But let's verify. Polling assoc.poll_endpoint_event() is fine.
                    // self.endpoint.handle_event() takes &mut Endpoint.
                    // So fine.
                    // But to be absolutely safe and consistent with other parts, let's try to handle inline 
                    // if compiler allows disjoint borrows, OR collect.
                    // The Error report didn't complain about endpoint interaction, only take_transmit and self.association=None.
                    // So endpoint interaction might be fine.
                    // Let's keep endpoint interaction inline but collect transmits/events.
                    if let Some(handle) = self.association_handle {
                         if let Some(back) = self.endpoint.handle_event(handle, ep_event) {
                             assoc.handle_event(back);
                             progressed = true;
                         }
                    }
                }

                while let Some(tx) = assoc.poll_transmit(now) {
                    pending_transmits.push(tx);
                }

                while let Some(event) = assoc.poll() {
                    pending_events.push(event);
                }
            } // assoc borrow ends

            // Process collected Transmits
            for tx in pending_transmits {
                if let Some(first) = self.take_transmit(tx) {
                     self.outgoing_queue.push_front(first);
                }
                progressed = true;
            }

            // Process collected Events
            for event in pending_events {
                 use sctp_proto::Event;
                 use sctp_proto::StreamEvent;
                 
                 // Debug Log
                 println!("DEBUG: SCTP Event: {:?}", event);
                 
                 match event {
                    Event::Stream(StreamEvent::Readable { id }) => {
                        // We need to borrow assoc again to read.
                        // This is fine as we are in the main loop scope, not inside the if-let.
                        if let Some(assoc) = self.association.as_mut() {
                             match assoc.stream(id) {
                                Ok(mut stream) => {
                                  // Read all available chunks
                                  loop {
                                      match stream.read() {
                                          Ok(Some(chunks)) => {
                                              let mut buf = vec![0u8; chunks.len()];
                                              if let Ok(_) = chunks.read(&mut buf) {
                                                  println!("DEBUG: Read {} bytes from Stream {}", buf.len(), id);
                                                  self.incoming_data.push_back((id, buf));
                                              }
                                          }
                                          Ok(None) => break, 
                                          Err(e) => {
                                              println!("DEBUG: Stream read error: {:?}", e);
                                              break;
                                          }
                                      }
                                      if !stream.is_readable() {
                                          break;
                                      }
                                  }
                                }
                                Err(e) => {
                                    println!("DEBUG: Failed to get stream {}: {:?}", id, e);
                                }
                             }
                        }
                        progressed = true;
                    }
                    Event::Stream(StreamEvent::Writable { id }) => {
                         println!("DEBUG: Stream {} is writable", id);
                    }
                    Event::AssociationLost { reason } => {
                        println!("DEBUG: SCTP Association Lost: {:?}", reason);
                        self.association = None;
                        progressed = true;
                    }
                    Event::Connected => {
                        println!("DEBUG: SCTP Connected");
                        progressed = true;
                    }
                    _ => {}
                 }
            }

            if !progressed {
                break;
            }
        }
    }
}
