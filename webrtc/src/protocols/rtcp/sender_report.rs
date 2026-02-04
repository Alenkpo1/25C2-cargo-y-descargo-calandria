use crate::protocols::rtcp::report_block::ReportBlock;

pub struct SenderReport {
    pub sender_ssrc: u32,
    pub ntp_msw: u32,
    pub ntp_lsw: u32,
    pub rtp_timestamp: u32,
    pub packet_count: u32,
    pub octet_count: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl SenderReport {
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.sender_ssrc.to_be_bytes());
        bytes.extend_from_slice(&self.ntp_msw.to_be_bytes());
        bytes.extend_from_slice(&self.ntp_lsw.to_be_bytes());
        bytes.extend_from_slice(&self.rtp_timestamp.to_be_bytes());
        bytes.extend_from_slice(&self.packet_count.to_be_bytes());
        bytes.extend_from_slice(&self.octet_count.to_be_bytes());
        for block in &self.report_blocks {
            bytes.extend_from_slice(&block.write_bytes());
        }
        bytes
    }

    pub fn read_bytes(bytes: &[u8], report_count: u8) -> Self {
        let sender_ssrc = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let ntp_msw = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let ntp_lsw = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let rtp_timestamp = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let packet_count = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let octet_count = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);

        let mut report_blocks = Vec::new();
        let mut offset = 24;
        for _ in 0..report_count {
            let block = ReportBlock::read_bytes(&bytes[offset..offset + 24]);
            report_blocks.push(block);
            offset += 24;
        }

        SenderReport {
            sender_ssrc,
            ntp_msw,
            ntp_lsw,
            rtp_timestamp,
            packet_count,
            octet_count,
            report_blocks,
        }
    }
}
