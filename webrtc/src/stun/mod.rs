mod attributes;
mod binding;
mod client;
mod message;

pub use client::StunClient;
pub use message::{MessageType, StunMessage};
pub const MAGIC_COOKIE: u32 = 0x2112A442;
pub const STUN_HEADER_SIZE: usize = 20;
