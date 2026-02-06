use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FileTransferMessage {
    #[serde(rename = "offer")]
    Offer {
        filename: String,
        size: usize,
        mime_type: String,
    },
    #[serde(rename = "answer")]
    Answer {
        accepted: bool,
    },
    #[serde(rename = "chunk")]
    Chunk {
        data: String, // Base64 if needed, but we prefer binary stream
    },
    #[serde(rename = "ack")]
    Ack {
        bytes_received: usize,
    },
    #[serde(rename = "eof")]
    Eof,
}
