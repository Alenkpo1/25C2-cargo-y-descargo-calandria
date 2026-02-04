use crate::protocols::rtcp::receiver_report::ReceiverReport;
use crate::protocols::rtcp::report_block::ReportBlock;
use crate::protocols::rtcp::sender_report::SenderReport;
use crate::protocols::rtp::rtp_packet::RtpPacket;
use std::time::{Duration, Instant, SystemTime};

const VIDEO_CLOCK_RATE: f64 = 90_000.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct CallMetricsSnapshot {
    pub bitrate_kbps: f32,
    pub packet_loss_pct: f32,
    pub jitter_ms: f32,
    pub highest_seq: u32,
    pub fraction_lost: u8,
    pub cumulative_lost: u32,
    pub since_last_ms: Option<u32>,
}

pub struct MediaMetrics {
    ssrc: u32,
    sender: SenderMetrics,
    receiver: ReceiverMetrics,
}

impl MediaMetrics {
    pub fn new(ssrc: u32) -> Self {
        Self {
            ssrc,
            sender: SenderMetrics::default(),
            receiver: ReceiverMetrics::default(),
        }
    }

    pub fn ssrc(&self) -> u32 {
        self.ssrc
    }

    pub fn update_sender(&mut self, payload_len: usize, rtp_timestamp: u32) {
        self.sender.packet_count = self.sender.packet_count.wrapping_add(1);
        self.sender.octet_count = self.sender.octet_count.wrapping_add(payload_len as u32);
        self.sender.last_rtp_timestamp = rtp_timestamp;
        self.sender.bytes_since_refresh += payload_len as u64;
        let now = Instant::now();
        if now.duration_since(self.sender.last_bitrate_check) >= Duration::from_millis(500) {
            let elapsed = now
                .duration_since(self.sender.last_bitrate_check)
                .as_secs_f32();
            if elapsed > 0.0 {
                let bits = (self.sender.bytes_since_refresh * 8) as f32;
                self.sender.bitrate_kbps = (bits / elapsed) / 1000.0;
            }
            self.sender.bytes_since_refresh = 0;
            self.sender.last_bitrate_check = now;
        }
    }

    pub fn update_receiver_on_rtp(&mut self, packet: &RtpPacket, arrival: Instant) {
        let seq = packet.get_sequence_number();
        let timestamp = packet.get_timestamp();
        let ssrc = packet.get_ssrc();
        if self.receiver.remote_ssrc.is_none() {
            self.receiver.remote_ssrc = Some(ssrc);
            self.receiver.base_time = Some(arrival);
        }

        self.receiver.received_packets = self.receiver.received_packets.wrapping_add(1);

        if let Some(last_seq) = self.receiver.last_sequence {
            let expected = last_seq.wrapping_add(1);
            let gap = seq.wrapping_sub(expected);
            if gap > 0 {
                self.receiver.lost_packets = self.receiver.lost_packets.saturating_add(gap as u32);
            }

            if seq < last_seq && last_seq.wrapping_sub(seq) > 30_000 {
                self.receiver.sequence_cycles = self.receiver.sequence_cycles.wrapping_add(1);
            }
        }

        let ext_seq = (self.receiver.sequence_cycles << 16) | (seq as u32);
        self.receiver.highest_ext_seq = ext_seq;
        self.receiver.last_sequence = Some(seq);

        let arrival_secs = if let Some(base) = self.receiver.base_time {
            arrival.duration_since(base).as_secs_f64()
        } else {
            0.0
        };

        let arrival_units = arrival_secs * VIDEO_CLOCK_RATE;
        let transit = arrival_units - (timestamp as f64);
        if let Some(prev_transit) = self.receiver.transit {
            let d = transit - prev_transit;
            self.receiver.jitter += (d.abs() - self.receiver.jitter) / 16.0;
        }
        self.receiver.transit = Some(transit);
        self.receiver.last_arrival = Some(arrival);
        self.receiver.last_rtp_timestamp = Some(timestamp);
    }

    pub fn record_remote_sr(&mut self, sr: &SenderReport, arrival: Instant) {
        self.receiver.last_sr = Some((sr.ntp_msw, sr.ntp_lsw, arrival));
    }

    pub fn build_sender_report(&mut self, ntp: (u32, u32)) -> Option<SenderReport> {
        if self.sender.packet_count == 0 {
            return None;
        }
        Some(SenderReport {
            sender_ssrc: self.ssrc,
            ntp_msw: ntp.0,
            ntp_lsw: ntp.1,
            rtp_timestamp: self.sender.last_rtp_timestamp,
            packet_count: self.sender.packet_count,
            octet_count: self.sender.octet_count,
            report_blocks: vec![],
        })
    }

    pub fn build_receiver_report(&self) -> Option<ReceiverReport> {
        let remote_ssrc = self.receiver.remote_ssrc?;
        let expected = self.receiver.received_packets + self.receiver.lost_packets;
        let fraction_lost = if expected > 0 {
            ((self.receiver.lost_packets * 256) / expected).min(255) as u8
        } else {
            0
        };
        let cumulative = self.receiver.lost_packets.min(0x00FF_FFFF);
        let jitter = self.receiver.jitter.round() as u32;
        let (lsr, dlsr) = self.receiver.compact_last_sr();

        let block = ReportBlock {
            ssrc: remote_ssrc,
            fraction_lost,
            cumulative_lost: cumulative,
            highest_seq: self.receiver.highest_ext_seq,
            jitter,
            last_sr: lsr,
            delay_since_last_sr: dlsr,
        };

        Some(ReceiverReport {
            reporter_ssrc: self.ssrc,
            report_blocks: vec![block],
        })
    }

    pub fn snapshot(&self) -> CallMetricsSnapshot {
        let expected = self.receiver.received_packets + self.receiver.lost_packets;
        let loss_pct = if expected > 0 {
            (self.receiver.lost_packets as f32 / expected as f32) * 100.0
        } else {
            0.0
        };
        let fraction_lost = if expected > 0 {
            ((self.receiver.lost_packets * 256) / expected).min(255) as u8
        } else {
            0
        };
        let cumulative = self.receiver.lost_packets.min(0x00FF_FFFF);
        let since_last_ms = self
            .receiver
            .last_arrival
            .map(|t| t.elapsed().as_millis() as u32);

        CallMetricsSnapshot {
            bitrate_kbps: self.sender.bitrate_kbps,
            packet_loss_pct: loss_pct,
            jitter_ms: ((self.receiver.jitter / VIDEO_CLOCK_RATE) * 1000.0) as f32,
            highest_seq: self.receiver.highest_ext_seq,
            fraction_lost,
            cumulative_lost: cumulative,
            since_last_ms,
        }
    }
}

impl ReceiverMetrics {
    fn compact_last_sr(&self) -> (u32, u32) {
        if let Some((msw, lsw, instant)) = self.last_sr {
            let lsr = ((msw & 0xFFFF) << 16) | ((lsw >> 16) & 0xFFFF);
            let elapsed = instant.elapsed();
            let dlsr = (elapsed.as_secs_f64() * 65_536.0).round() as u32;
            (lsr, dlsr)
        } else {
            (0, 0)
        }
    }
}

struct SenderMetrics {
    packet_count: u32,
    octet_count: u32,
    last_rtp_timestamp: u32,
    last_bitrate_check: Instant,
    bytes_since_refresh: u64,
    bitrate_kbps: f32,
}

impl Default for SenderMetrics {
    fn default() -> Self {
        Self {
            packet_count: 0,
            octet_count: 0,
            last_rtp_timestamp: 0,
            last_bitrate_check: Instant::now(),
            bytes_since_refresh: 0,
            bitrate_kbps: 0.0,
        }
    }
}

struct ReceiverMetrics {
    remote_ssrc: Option<u32>,
    received_packets: u32,
    lost_packets: u32,
    last_sequence: Option<u16>,
    sequence_cycles: u32,
    highest_ext_seq: u32,
    jitter: f64,
    transit: Option<f64>,
    last_arrival: Option<Instant>,
    last_rtp_timestamp: Option<u32>,
    base_time: Option<Instant>,
    last_sr: Option<(u32, u32, Instant)>,
}

impl Default for ReceiverMetrics {
    fn default() -> Self {
        Self {
            remote_ssrc: None,
            received_packets: 0,
            lost_packets: 0,
            last_sequence: None,
            sequence_cycles: 0,
            highest_ext_seq: 0,
            jitter: 0.0,
            transit: None,
            last_arrival: None,
            last_rtp_timestamp: None,
            base_time: None,
            last_sr: None,
        }
    }
}

pub fn system_time_to_ntp(now: SystemTime) -> (u32, u32) {
    const NTP_UNIX_OFFSET: u64 = 2_208_988_800;
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = duration.as_secs() + NTP_UNIX_OFFSET;
    let fraction = ((duration.subsec_nanos() as u64) << 32) / 1_000_000_000u64;
    (seconds as u32, fraction as u32)
}
