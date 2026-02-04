//! Punto de entrada del servidor de señalización.

mod config;
mod logger;
mod server;

use config::AppConfig;
use logger::Logger;
use server::state::ServerState;
use server::tls::build_tls_config;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

fn main() -> std::io::Result<()> {
    let config_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => "server.conf".to_string(),
    };
    let config = match AppConfig::load(&config_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!(
                "No se pudo cargar {} ({}), usando valores por defecto",
                config_path, err
            );
            AppConfig::default()
        }
    };
    let logger = Logger::start(&config.log_file)?;

    let listener = TcpListener::bind(&config.server_addr)?;
    let state = Arc::new(ServerState::new(&config, logger.clone()));
    let tls_config = build_tls_config();

    state.load_users()?;

    println!("Signaling server listening in {}", config.server_addr);
    println!("Users file: {}", config.users_file);
    println!("Max clients: {}", config.max_clients);
    println!("Encryption: TLS (self-signed)\n");
    logger.info(&format!(
        "Servidor iniciado en {} con archivo de usuarios {}",
        config.server_addr, config.users_file
    ));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let addr = match stream.peer_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        logger.error(&format!("No se pudo obtener addr del cliente: {}", e));
                        continue;
                    }
                };

                // Limitar conexiones concurrentes
                let over_capacity = match state.connected_clients.read() {
                    Ok(clients) => clients.len() >= config.max_clients,
                    Err(_) => {
                        logger.error("Lock de clientes envenenado");
                        true
                    }
                };
                if over_capacity {
                    println!(
                        "Max clients capacity reached, refuse connection from {}",
                        addr
                    );
                    logger.warn("Capacidad máxima alcanzada, rechazando conexión");
                    continue;
                }

                let state = Arc::clone(&state);
                let tls_config = Arc::clone(&tls_config);
                thread::spawn(move || {
                    server::handle_client(stream, addr, state, tls_config);
                });
            }
            Err(e) => {
                logger.error(&format!("Error aceptando conexión: {}", e));
            }
        }
    }

    Ok(())
}
