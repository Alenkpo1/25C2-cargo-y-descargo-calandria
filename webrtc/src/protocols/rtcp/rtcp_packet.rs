use crate::protocols::rtcp::rtcp_bye::ByeRtcp;
use crate::protocols::rtcp::rtcp_const::rtp_controller_const::RTCP_BYE_TYPE;
use crate::protocols::rtcp::rtcp_err::rtcp_error::RtcpError;
use crate::protocols::rtcp::rtcp_header::RtcpHeader;
use crate::protocols::rtcp::rtcp_payload::RtcpPayload;
pub struct RtcpPacket {
    pub header: RtcpHeader,
    pub payload: RtcpPayload,
}
impl RtcpPacket {
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.write_bytes();
        bytes.extend_from_slice(&self.payload.write_bytes());
        bytes
    }
    pub fn read_bytes(bytes: &[u8]) -> Result<Self, RtcpError> {
        let header = RtcpHeader::read_bytes(bytes);
        let payload_type = header.get_packet_type();
        let report_count = header.get_report_count();
        let payload = RtcpPayload::read_bytes(payload_type, report_count, &bytes[4..])?;
        Ok(Self { header, payload })
    }

    pub fn from_payload(packet_type: u8, rc: u8, payload: RtcpPayload) -> Self {
        let payload_len = payload.write_bytes().len();
        let total_len = payload_len + 4;
        let total_words = total_len.div_ceil(4) as u16;
        let header = RtcpHeader::new(false, rc, packet_type, total_words - 1);
        Self { header, payload }
    }

    /// Helper to generate an RTCP BYE packet for the provided SSRC.
    pub fn bye(ssrc: u32) -> Self {
        let payload = RtcpPayload::Bye(ByeRtcp::new(ssrc));
        RtcpPacket::from_payload(RTCP_BYE_TYPE, 1, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::rtcp::receiver_report::ReceiverReport;
    use crate::protocols::rtcp::report_block::ReportBlock;
    use crate::protocols::rtcp::rtcp_const::rtp_controller_const::{
        RECEIVER_REPORT_TYPE, SENDER_REPORT_TYPE,
    };
    use crate::protocols::rtcp::rtcp_payload::RtcpPayload;
    use crate::protocols::rtcp::sender_report::SenderReport;

    #[test]
    fn sender_report_roundtrip() {
        let sr = SenderReport {
            sender_ssrc: 1,
            ntp_msw: 2,
            ntp_lsw: 3,
            rtp_timestamp: 4,
            packet_count: 5,
            octet_count: 6,
            report_blocks: vec![],
        };
        let packet = RtcpPacket::from_payload(SENDER_REPORT_TYPE, 0, RtcpPayload::SenderReport(sr));
        let bytes = packet.write_bytes();
        let parsed = RtcpPacket::read_bytes(&bytes).expect("rtcp");
        assert!(matches!(parsed.payload, RtcpPayload::SenderReport(_)));
    }

    #[test]
    fn receiver_report_roundtrip() {
        let rr = ReceiverReport {
            reporter_ssrc: 9,
            report_blocks: vec![ReportBlock {
                ssrc: 10,
                fraction_lost: 1,
                cumulative_lost: 2,
                highest_seq: 3,
                jitter: 4,
                last_sr: 5,
                delay_since_last_sr: 6,
            }],
        };
        let packet =
            RtcpPacket::from_payload(RECEIVER_REPORT_TYPE, 1, RtcpPayload::ReceiverReport(rr));
        let bytes = packet.write_bytes();
        let parsed = RtcpPacket::read_bytes(&bytes).expect("rtcp");
        assert!(matches!(parsed.payload, RtcpPayload::ReceiverReport(_)));
    }

    #[test]
    fn bye_roundtrip() {
        let bye = RtcpPacket::bye(1234);
        let bytes = bye.write_bytes();
        let parsed = RtcpPacket::read_bytes(&bytes).expect("rtcp");
        assert!(matches!(parsed.payload, RtcpPayload::Bye(_)));
    }
}
