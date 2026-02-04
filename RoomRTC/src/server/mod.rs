//! Servidor de señalización RoomRTC.
//!
//! Este módulo contiene el loop principal del cliente y reexports de todos los submódulos.

pub mod handlers;
pub mod protocol;
pub mod state;
pub mod tls;
pub mod types;
pub mod validation;

use std::io::{BufRead, BufReader, ErrorKind};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use rustls::{ServerConfig, ServerConnection, StreamOwned};

use handlers::{dispatch, HandlerResult};
use protocol::{flush_outgoing, parse_message};
use state::ServerState;
use types::{TlsStream, UserStatus};

/// Maneja una conexión de cliente individual.
pub fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    state: Arc<ServerState>,
    tls_config: Arc<ServerConfig>,
) {
    println!("New connection from: {}", addr);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));

    let server_conn = match ServerConnection::new(tls_config) {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Error creating TLS connection: {}", err);
            return;
        }
    };

    let tls_stream: TlsStream = StreamOwned::new(server_conn, stream);
    let mut reader = BufReader::new(tls_stream);
    let (tx, rx) = mpsc::channel::<String>();
    let mut authenticated_user: Option<String> = None;

    loop {
        if let Err(e) = flush_outgoing(&mut reader, &rx) {
            eprintln!("Error sending message: {}", e);
            break;
        }

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => {
                println!("Error reading line: {}", e);
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg = parse_message(trimmed);
        let result = dispatch(&msg, &tx, &state, &mut authenticated_user);

        if result == HandlerResult::Disconnect {
            break;
        }
    }

    // Cleanup al desconectar
    if let Some(username) = authenticated_user {
        println!("Client {} disconnected", username);
        if let Ok(mut guard) = state.connected_clients.write() {
            guard.remove(&username);
        }
        state.set_user_status(&username, UserStatus::Disconnected);
        state.logger.warn(&format!("{} se desconectó", username));

        // Si estaba en llamada, notificar al otro
        if let Ok(mut calls) = state.active_calls.write()
            && let Some(other) = calls.remove(&username)
        {
            calls.remove(&other);
            state.set_user_status(&other, UserStatus::Available);

            if let Ok(clients) = state.connected_clients.read()
                && let Some(other_client) = clients.get(&other)
            {
                let msg = format!("CALL_ENDED|from:{}", username);
                ServerState::send_message(&other_client.sender, &msg);
            }
        }
    }
}
