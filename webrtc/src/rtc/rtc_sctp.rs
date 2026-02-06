use sctp_proto::{
    Association, AssociationHandle, ClientConfig, DatagramEvent, Endpoint, EndpointConfig,
    Payload, PayloadProtocolIdentifier, ServerConfig, Transmit,
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

        let server_config = is_server
            .then(|| Arc::new(ServerConfig::default()));

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
            let client_config = ClientConfig::default();

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

            stream.write(&payload).map_err(|e| e.to_string())?;
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

            if let Some(assoc) = self.association.as_mut() {
                while let Some(ep_event) = assoc.poll_endpoint_event() {
                    if let Some(handle) = self.association_handle {
                        if let Some(back) = self.endpoint.handle_event(handle, ep_event) {
                            assoc.handle_event(back);
                            progressed = true;
                        }
                    }
                }

                let mut pending: Vec<Transmit> = Vec::new();
                while let Some(tx) = assoc.poll_transmit(now) {
                    pending.push(tx);
                }

                for tx in pending {
                    if let Some(first) = self.take_transmit(tx) {
                        self.outgoing_queue.push_front(first);
                    }
                    progressed = true;
                }
            }

            if !progressed {
                break;
            }
        }
    }
}
