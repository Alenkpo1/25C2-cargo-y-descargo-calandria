//! Tipos compartidos del servidor de señalización.

use std::sync::mpsc::Sender;

use rustls::{ServerConnection, StreamOwned};
use std::net::TcpStream;

/// Estado de conexión de un usuario.
#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Disconnected,
    Available,
    Busy,
}

impl UserStatus {
    pub fn to_string(&self) -> &str {
        match self {
            UserStatus::Disconnected => "DISCONNECTED",
            UserStatus::Available => "AVAILABLE",
            UserStatus::Busy => "BUSY",
        }
    }
}

/// Datos de usuario persistidos.
#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
    pub metadata: String,
}

/// Alias para el stream TLS del servidor.
pub type TlsStream = StreamOwned<ServerConnection, TcpStream>;

/// Cliente conectado con su canal de envío.
pub struct ConnectedClient {
    pub sender: Sender<String>,
}
