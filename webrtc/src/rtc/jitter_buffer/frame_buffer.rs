use crate::protocols::rtp::rtp_packet::RtpPacket;

pub struct FrameBuffer {
    packets: Vec<RtpPacket>,
    marker_received: bool,
}
impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}
impl FrameBuffer {
    pub fn new() -> Self {
        FrameBuffer {
            packets: Vec::new(),
            marker_received: false,
        }
    }
    pub fn push(&mut self, packet: RtpPacket) {
        if packet.get_marker() {
            self.marker_received = true;
        }
        self.packets.push(packet);
    }
    pub fn is_complete(&self) -> bool {
        self.marker_received && !self.packets.is_empty()
    }
    pub fn sort_by_sequence(&mut self) {
        self.packets
            .sort_by_key(|rtp_packet| rtp_packet.get_sequence_number());
    }
    pub fn to_bytes(&mut self) -> Vec<u8> {
        self.sort_by_sequence();

        let mut full_data = Vec::new();
        let mut fu_construction = false;
        let mut current_fu: Vec<u8> = Vec::new();

        for rtp_packet in &self.packets {
            let payload = rtp_packet.get_payload_bytes();
            if payload.is_empty() {
                continue;
            }

            let nal_type = payload[0] & 0x1F;

            if nal_type != 28 {
                full_data.extend_from_slice(&[0, 0, 0, 1]);
                full_data.extend_from_slice(&payload);
                continue;
            }

            if payload.len() < 2 {
                continue;
            }

            let fu_indicator = payload[0];
            let fu_header = payload[1];

            let start = fu_header & 0x80 != 0;
            let end = fu_header & 0x40 != 0;
            let nal_type = fu_header & 0x1F;
            let reconstructed_header = (fu_indicator & 0xE0) | nal_type;

            if start {
                fu_construction = true;
                current_fu.clear();

                full_data.extend_from_slice(&[0, 0, 0, 1]);
                current_fu.push(reconstructed_header);

                current_fu.extend_from_slice(&payload[2..]);
            } else if fu_construction {
                current_fu.extend_from_slice(&payload[2..]);
            }

            if end && fu_construction {
                full_data.extend_from_slice(&current_fu);
                fu_construction = false;
                current_fu.clear();
            }
        }
        full_data
    }
    pub fn get_packets(&self) -> &Vec<RtpPacket> {
        &self.packets
    }
}
