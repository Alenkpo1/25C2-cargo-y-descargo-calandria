#[derive(Clone)]
pub struct ReportBlock {
    pub ssrc: u32,
    pub fraction_lost: u8,
    pub cumulative_lost: u32,
    pub highest_seq: u32,
    pub jitter: u32,
    pub last_sr: u32,
    pub delay_since_last_sr: u32,
}

impl ReportBlock {
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24);
        bytes.extend_from_slice(&self.ssrc.to_be_bytes());
        bytes.push(self.fraction_lost);
        let cumulative = self.cumulative_lost & 0x00FF_FFFF;
        bytes.extend_from_slice(&[
            ((cumulative >> 16) & 0xFF) as u8,
            ((cumulative >> 8) & 0xFF) as u8,
            (cumulative & 0xFF) as u8,
        ]);
        bytes.extend_from_slice(&self.highest_seq.to_be_bytes());
        bytes.extend_from_slice(&self.jitter.to_be_bytes());
        bytes.extend_from_slice(&self.last_sr.to_be_bytes());
        bytes.extend_from_slice(&self.delay_since_last_sr.to_be_bytes());
        bytes
    }

    pub fn read_bytes(bytes: &[u8]) -> Self {
        let ssrc = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let fraction_lost = bytes[4];
        let cumulative_lost =
            ((bytes[5] as u32) << 16) | ((bytes[6] as u32) << 8) | bytes[7] as u32;
        let highest_seq = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let jitter = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let last_sr = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let delay_since_last_sr = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        Self {
            ssrc,
            fraction_lost,
            cumulative_lost,
            highest_seq,
            jitter,
            last_sr,
            delay_since_last_sr,
        }
    }
}
