//! Handlers de autenticación: REGISTER, LOGIN, LOGOUT.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::context::HandlerResult;
use crate::server::state::ServerState;
use crate::server::types::{ConnectedClient, UserStatus};
use crate::server::validation::{validate_password, validate_username};

/// Procesa el mensaje REGISTER.
pub fn handle_register(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
) -> HandlerResult {
    let Some(username) = msg.get("username").cloned() else {
        ServerState::send_message(tx, "REGISTER_ERROR|error:missing username");
        return HandlerResult::Continue;
    };
    let Some(password) = msg.get("password").cloned() else {
        ServerState::send_message(tx, "REGISTER_ERROR|error:missing password");
        return HandlerResult::Continue;
    };
    if let Err(err) = validate_username(&username).and_then(|_| validate_password(&password)) {
        ServerState::send_message(tx, &format!("REGISTER_ERROR|error:{}", err));
        return HandlerResult::Continue;
    }

    match state.register_user(username, password) {
        Ok(_) => {
            ServerState::send_message(tx, "REGISTER_SUCCESS|message:User register successfully");
            state.logger.info("Registro de usuario exitoso");
        }
        Err(e) => {
            ServerState::send_message(tx, &format!("REGISTER_ERROR|error:{}", e));
            state
                .logger
                .error(&format!("Error registrando usuario: {}", e));
        }
    }
    HandlerResult::Continue
}

/// Procesa el mensaje LOGIN.
pub fn handle_login(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &mut Option<String>,
) -> HandlerResult {
    let Some(username) = msg.get("username").cloned() else {
        ServerState::send_message(tx, "LOGIN_ERROR|error:missing username");
        return HandlerResult::Continue;
    };
    let Some(password) = msg.get("password").cloned() else {
        ServerState::send_message(tx, "LOGIN_ERROR|error:missing password");
        return HandlerResult::Continue;
    };
    if let Err(err) = validate_username(&username).and_then(|_| validate_password(&password)) {
        ServerState::send_message(tx, &format!("LOGIN_ERROR|error:{}", err));
        return HandlerResult::Continue;
    }

    match state.authenticate(&username, &password) {
        Ok(_) => {
            let already_connected = match state.connected_clients.read() {
                Ok(clients) => clients.contains_key(&username),
                Err(_) => {
                    ServerState::send_message(tx, "LOGIN_ERROR|error:internal server error");
                    state
                        .logger
                        .error("No se pudo leer clientes conectados (lock envenenado)");
                    return HandlerResult::Continue;
                }
            };
            if already_connected {
                ServerState::send_message(tx, "LOGIN_ERROR|error:User already connected");
                return HandlerResult::Continue;
            }

            *authenticated_user = Some(username.clone());

            let client = ConnectedClient { sender: tx.clone() };

            if let Ok(mut guard) = state.connected_clients.write() {
                guard.insert(username.clone(), client);
            } else {
                ServerState::send_message(tx, "LOGIN_ERROR|error:internal server error");
                state
                    .logger
                    .error("No se pudo guardar cliente (lock envenenado)");
                return HandlerResult::Continue;
            }
            state.set_user_status(&username, UserStatus::Available);

            ServerState::send_message(tx, "LOGIN_SUCCESS|message:Login success");
            state.logger.info(&format!("{} inició sesión", username));
        }
        Err(e) => {
            ServerState::send_message(tx, &format!("LOGIN_ERROR|error:{}", e));
            state.logger.error(&format!("Error de login: {}", e));
        }
    }
    HandlerResult::Continue
}

/// Procesa el mensaje LOGOUT.
pub fn handle_logout(
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &Option<String>,
) -> HandlerResult {
    if let Some(username) = authenticated_user {
        if let Ok(mut guard) = state.connected_clients.write() {
            guard.remove(username);
        }
        state.set_user_status(username, UserStatus::Disconnected);
        ServerState::send_message(tx, "LOGOUT_SUCCESS");
        state.logger.info(&format!("{} cerró sesión", username));
    }
    HandlerResult::Disconnect
}
