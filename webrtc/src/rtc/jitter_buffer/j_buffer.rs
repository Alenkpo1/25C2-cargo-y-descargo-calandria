use crate::protocols::rtp::rtp_packet::RtpPacket;
use crate::rtc::jitter_buffer::frame_buffer::FrameBuffer;
use std::collections::HashMap;

pub struct JitterBuffer {
    frames: HashMap<u32, FrameBuffer>,
    last_timestamp: Option<u32>,
}
impl Default for JitterBuffer {
    fn default() -> Self {
        Self::new()
    }
}
impl JitterBuffer {
    pub fn new() -> Self {
        JitterBuffer {
            frames: HashMap::new(),
            last_timestamp: None,
        }
    }
    pub fn push(&mut self, packet: RtpPacket) {
        let timestap = packet.get_timestamp();
        let frame = self.frames.entry(timestap).or_default();
        frame.push(packet);
    }

    pub fn sort_by_sequence(packets: &mut [RtpPacket]) {
        packets.sort_by_key(|p| p.get_sequence_number());
    }
    pub fn is_timestamp_newer(ts1: u32, ts2: u32) -> bool {
        ts1.wrapping_sub(ts2) < 0x8000_0000
    }
    pub fn pop(&mut self) -> Option<FrameBuffer> {
        if self.frames.is_empty() {
            return None;
        }
        if let Some(last_ts) = self.last_timestamp {
            let stale: Vec<u32> = self
                .frames
                .keys()
                .copied()
                .filter(|ts| !Self::is_timestamp_newer(*ts, last_ts))
                .collect();
            for ts in stale {
                self.frames.remove(&ts);
            }
        }
        let mut min_timestamp: Option<u32> = None;
        for &ts in self.frames.keys() {
            match min_timestamp {
                None => min_timestamp = Some(ts),
                Some(current_min) => {
                    if !Self::is_timestamp_newer(ts, current_min) {
                        min_timestamp = Some(ts);
                    }
                }
            }
        }
        let ts = min_timestamp?;
        let has_incomplete_older = self.frames.iter().any(|(&older_ts, frame)| {
            !frame.is_complete() && !Self::is_timestamp_newer(older_ts, ts)
        });
        if has_incomplete_older {
            return None;
        }
        if let Some(frame) = self.frames.get(&ts) {
            if frame.is_complete() {
                self.last_timestamp = Some(ts);
                return self.frames.remove(&ts);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::h264::nalu_header::NaluHeader;
    use crate::codec::h264::single_nal_unit_packet::SingleNalUnitPacket;
    use crate::protocols::rtp::constants::rtp_const::RTP_H264_TYPE;
    use crate::protocols::rtp::h264_video_type::H264VideoType;
    use crate::protocols::rtp::payload_type::PayloadType;
    use crate::protocols::rtp::rtp_header::RtpHeader;
    use crate::protocols::rtp::rtp_packet::RtpPacket;

    fn make_rtp(sequence: u16, timestamp: u32, marker: bool) -> RtpPacket {
        let nalu_header = NaluHeader::new(false, 0, 1);
        let single = SingleNalUnitPacket::new(nalu_header, vec![0xAA, 0xBB]);
        let payload = PayloadType::H264Video(H264VideoType::Single(single));
        let header = RtpHeader::new(
            2,
            false,
            false,
            0,
            marker,
            RTP_H264_TYPE,
            sequence,
            timestamp,
            1234,
            vec![],
        );
        RtpPacket::new(header, payload)
    }

    #[test]
    fn completes_frame_when_marker_seen() {
        let mut jitter = JitterBuffer::new();
        let ts = 10;
        let pkt1 = make_rtp(1, ts, false);
        let pkt2 = make_rtp(2, ts, true);

        jitter.push(pkt1);
        assert!(jitter.pop().is_none());

        jitter.push(pkt2);
        let frame = jitter.pop().expect("frame");
        assert!(frame.is_complete());
        assert_eq!(frame.get_packets().len(), 2);
    }
}
