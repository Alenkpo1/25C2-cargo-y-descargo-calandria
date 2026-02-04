use crate::codec::h264::encoder::H264Encoder;
use crate::codec::h264::fu_a::FragmentationUnitTypeA;
use crate::codec::h264::fu_header::FuHeader;
use crate::codec::h264::nalu_header::NaluHeader;
use crate::codec::h264::single_nal_unit_packet::SingleNalUnitPacket;
use crate::crypto::srtp::SrtpContext;
use crate::protocols::rtp::constants::rtp_const::RTP_H264_TYPE;
use crate::protocols::rtp::h264_video_type::H264VideoType;
use crate::protocols::rtp::payload_type::PayloadType;
use crate::protocols::rtp::rtp_header::RtpHeader;
use crate::protocols::rtp::rtp_packet::RtpPacket;
use crate::rtc::rtc_err::RtcError;
use crate::rtc::socket::peer_socket::PeerSocket;
use crate::worker_thread::media_metrics::MediaMetrics;
use std::sync::{Arc, Mutex};

pub struct RtcRtpSender {
    ssrc: u32,
    sequence_number: u16,
    timestamp: u32,
    metrics: Arc<Mutex<MediaMetrics>>,
    srtp: Option<SrtpContext>,
}
impl RtcRtpSender {
    pub fn new(ssrc: u32, metrics: Arc<Mutex<MediaMetrics>>, key: Option<Vec<u8>>) -> Self {
        RtcRtpSender {
            ssrc,
            sequence_number: 0,
            timestamp: 0,
            metrics,
            srtp: key.and_then(|k| SrtpContext::new(&k)),
        }
    }
    pub fn send_video_payload(
        &mut self,
        frame_bytes: Vec<u8>,
        rtp_socket: &mut PeerSocket,
    ) -> Result<(), RtcError> {
        let nalus = H264Encoder::split_by_startcode(&frame_bytes);
        let total_nalus = nalus.len();

        for (n, nalu) in nalus.into_iter().enumerate() {
            let nalu_header = NaluHeader::read_byte(nalu[0]);
            let is_last_nalu = n == total_nalus - 1;
            if nalu.len() <= 900 {
                self.send_single_nalu(nalu_header, nalu, is_last_nalu, rtp_socket)?;
            } else {
                self.send_fragmented_nalu(nalu_header, nalu, is_last_nalu, rtp_socket)?;
            }
        }

        // clock rate 90kHz, target 30 fps -> 3000 ticks por frame
        self.timestamp = self.timestamp.wrapping_add(3000);
        Ok(())
    }
    fn send_single_nalu(
        &mut self,
        header: NaluHeader,
        nalu: Vec<u8>,
        last_nalu: bool,
        rtp_socket: &mut PeerSocket,
    ) -> Result<(), RtcError> {
        let single = SingleNalUnitPacket::new(header, nalu[1..].to_vec());
        let payload = PayloadType::H264Video(H264VideoType::Single(single));
        let rtp_header = RtpHeader::new(
            2,
            false,
            false,
            0,
            last_nalu,
            RTP_H264_TYPE,
            self.sequence_number,
            self.timestamp,
            self.ssrc,
            vec![],
        );
        let packet = RtpPacket::new(rtp_header, payload);
        let mut bytes = packet.write_bytes();
        if let Some(ref srtp) = self.srtp {
            let header = &bytes[..12];
            if let Some(cipher) = srtp.protect(self.sequence_number, self.timestamp, &bytes[12..]) {
                let mut out = Vec::with_capacity(12 + cipher.len());
                out.extend_from_slice(header);
                out.extend_from_slice(&cipher);
                bytes = out;
            }
        }
        rtp_socket.send(&bytes).map_err(RtcError::RtcPeerError)?;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        self.register_send(bytes.len(), self.timestamp);
        Ok(())
    }

    fn send_fragmented_nalu(
        &mut self,
        header: NaluHeader,
        nalu: Vec<u8>,
        last_nalu: bool,
        rtp_socket: &mut PeerSocket,
    ) -> Result<(), RtcError> {
        let nalu_type = header.get_nalu_type();
        let nri = header.get_nri();
        let forbidden = header.get_forbidden_zero_bit();
        let vec_fu_a: Vec<Vec<u8>> = H264Encoder::split_nal(nalu[1..].to_vec());
        let total_fu_a = vec_fu_a.len();
        for (i, byte_slice) in vec_fu_a.into_iter().enumerate() {
            let start = i == 0;
            let end = i == total_fu_a - 1;
            let fu_indicator = NaluHeader::new(forbidden, nri, 28);
            let fu_header = FuHeader::new(start, end, false, nalu_type);
            let fu_a = FragmentationUnitTypeA::new(fu_indicator, fu_header, byte_slice);
            let payload = PayloadType::H264Video(H264VideoType::Fragmented(fu_a));
            let marker = end && last_nalu;
            let rtp_header = RtpHeader::new(
                2,
                false,
                false,
                0,
                marker,
                RTP_H264_TYPE,
                self.sequence_number,
                self.timestamp,
                self.ssrc,
                vec![],
            );
            let packet = RtpPacket::new(rtp_header, payload);
            let mut bytes = packet.write_bytes();
            if let Some(ref srtp) = self.srtp {
                let header = &bytes[..12];
                if let Some(cipher) =
                    srtp.protect(self.sequence_number, self.timestamp, &bytes[12..])
                {
                    let mut out = Vec::with_capacity(12 + cipher.len());
                    out.extend_from_slice(header);
                    out.extend_from_slice(&cipher);
                    bytes = out;
                }
            }
            rtp_socket.send(&bytes).map_err(RtcError::RtcPeerError)?;
            self.sequence_number = self.sequence_number.wrapping_add(1);
            self.register_send(bytes.len(), self.timestamp);
        }
        Ok(())
    }

    fn register_send(&self, packet_len: usize, timestamp: u32) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.update_sender(packet_len, timestamp);
        }
    }
}
