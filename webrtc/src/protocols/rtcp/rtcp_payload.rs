use crate::protocols::rtcp::receiver_report::ReceiverReport;
use crate::protocols::rtcp::rtcp_bye::ByeRtcp;
use crate::protocols::rtcp::rtcp_const::rtp_controller_const::{
    RECEIVER_REPORT_TYPE, RTCP_BYE_TYPE, SENDER_REPORT_TYPE, SOURCE_DESCRIPTION_TYPE,
};
use crate::protocols::rtcp::rtcp_err::rtcp_error::RtcpError;
use crate::protocols::rtcp::sender_report::SenderReport;
use crate::protocols::rtcp::source_description_enum::SdesEnum;

pub enum RtcpPayload {
    SenderReport(SenderReport),
    ReceiverReport(ReceiverReport),
    Sdes(SdesEnum),
    Bye(ByeRtcp),
}

impl RtcpPayload {
    pub fn write_bytes(&self) -> Vec<u8> {
        match self {
            RtcpPayload::SenderReport(sr) => sr.write_bytes(),
            RtcpPayload::ReceiverReport(rr) => rr.write_bytes(),
            RtcpPayload::Sdes(sdes) => sdes.write_bytes(),
            RtcpPayload::Bye(bye) => bye.write_bytes(),
        }
    }
    pub fn read_bytes(payload_type: u8, report_count: u8, bytes: &[u8]) -> Result<Self, RtcpError> {
        match payload_type {
            SENDER_REPORT_TYPE => Ok(RtcpPayload::SenderReport(SenderReport::read_bytes(
                bytes,
                report_count,
            ))),
            RECEIVER_REPORT_TYPE => Ok(RtcpPayload::ReceiverReport(ReceiverReport::read_bytes(
                bytes,
                report_count,
            ))),
            SOURCE_DESCRIPTION_TYPE => Ok(RtcpPayload::Sdes(SdesEnum::read_bytes(bytes)?)),
            RTCP_BYE_TYPE => Ok(RtcpPayload::Bye(ByeRtcp::read_bytes(bytes))),
            invalid => Err(RtcpError::InvalidRtcpPayloadType(invalid)),
        }
    }
}
