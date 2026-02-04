//! Contexto y dispatcher de handlers.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::server::state::ServerState;

use super::auth::{handle_login, handle_logout, handle_register};
use super::presence::handle_get_users;
use super::signaling::{
    handle_call_answer, handle_call_end, handle_call_offer, handle_call_reject, handle_ice_candidate,
};

/// Resultado de un handler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandlerResult {
    /// Continuar procesando mensajes.
    Continue,
    /// Desconectar el cliente.
    Disconnect,
}

/// Despacha el mensaje al handler correspondiente según el tipo.
pub fn dispatch(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &mut Option<String>,
) -> HandlerResult {
    let Some(msg_type) = msg.get("type").map(|s| s.as_str()) else {
        ServerState::send_message(tx, "ERROR|error:missing type");
        return HandlerResult::Continue;
    };

    match msg_type {
        "REGISTER" => handle_register(msg, tx, state),
        "LOGIN" => handle_login(msg, tx, state, authenticated_user),
        "LOGOUT" => handle_logout(tx, state, authenticated_user),
        "GET_USERS" => handle_get_users(tx, state),
        "CALL_OFFER" => handle_call_offer(msg, tx, state, authenticated_user),
        "CALL_ANSWER" => handle_call_answer(msg, tx, state, authenticated_user),
        "CALL_REJECT" => handle_call_reject(msg, tx, state, authenticated_user),
        "CALL_END" => handle_call_end(msg, tx, state, authenticated_user),
        "ICE_CANDIDATE" => handle_ice_candidate(msg, tx, state, authenticated_user),
        _ => {
            ServerState::send_message(
                tx,
                &format!("ERROR|error:unknow message type: {}", msg_type),
            );
            HandlerResult::Continue
        }
    }
}
