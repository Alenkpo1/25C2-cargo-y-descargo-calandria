//! Parsing y serialización del protocolo de mensajes.

use std::collections::HashMap;
use std::io::{self, BufReader, Write};
use std::sync::mpsc::Receiver;

use super::types::TlsStream;

/// Parsea un mensaje del protocolo en formato "TYPE|key:value|key:value".
pub fn parse_message(msg: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let parts: Vec<&str> = msg.split('|').collect();

    if !parts.is_empty() {
        map.insert("type".to_string(), parts[0].to_string());

        for part in &parts[1..] {
            if let Some(pos) = part.find(':') {
                let key = &part[..pos];
                let value = &part[pos + 1..];
                map.insert(key.to_string(), value.to_string());
            }
        }
    }

    map
}

/// Envía todos los mensajes pendientes en el canal al stream TLS.
pub fn flush_outgoing(reader: &mut BufReader<TlsStream>, rx: &Receiver<String>) -> io::Result<()> {
    while let Ok(msg) = rx.try_recv() {
        let stream = reader.get_mut();
        stream.write_all(msg.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;
    }
    Ok(())
}
