use crate::protocols::rtcp::report_block::ReportBlock;

pub struct ReceiverReport {
    pub reporter_ssrc: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl ReceiverReport {
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.reporter_ssrc.to_be_bytes());
        for block in &self.report_blocks {
            bytes.extend_from_slice(&block.write_bytes());
        }
        bytes
    }

    pub fn read_bytes(bytes: &[u8], report_count: u8) -> ReceiverReport {
        let reporter_ssrc = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let mut blocks = Vec::new();
        let mut offset = 4;
        for _ in 0..report_count {
            let block = ReportBlock::read_bytes(&bytes[offset..offset + 24]);
            blocks.push(block);
            offset += 24;
        }
        ReceiverReport {
            reporter_ssrc,
            report_blocks: blocks,
        }
    }
}
