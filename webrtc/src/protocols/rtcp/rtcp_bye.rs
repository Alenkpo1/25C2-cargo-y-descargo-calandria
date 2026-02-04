pub struct ByeRtcp {
    ssrc: u32,
}
impl ByeRtcp {
    pub fn new(ssrc: u32) -> Self {
        Self { ssrc }
    }
    pub fn write_bytes(&self) -> Vec<u8> {
        self.ssrc.to_be_bytes().to_vec()
    }
    pub fn read_bytes(bytes: &[u8]) -> ByeRtcp {
        let ssrc = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        ByeRtcp { ssrc }
    }
}
