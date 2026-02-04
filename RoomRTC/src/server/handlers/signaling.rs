//! Handlers de señalización: CALL_OFFER, CALL_ANSWER, CALL_REJECT, CALL_END, ICE_CANDIDATE.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::context::HandlerResult;
use crate::server::state::ServerState;
use crate::server::types::UserStatus;

/// Procesa el mensaje CALL_OFFER.
pub fn handle_call_offer(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &Option<String>,
) -> HandlerResult {
    let Some(caller) = authenticated_user else {
        return HandlerResult::Continue;
    };

    let Some(to) = msg.get("to").cloned() else {
        ServerState::send_message(tx, "CALL_ERROR|error:missing destination");
        return HandlerResult::Continue;
    };
    let Some(sdp) = msg.get("sdp").cloned() else {
        ServerState::send_message(tx, "CALL_ERROR|error:missing sdp");
        return HandlerResult::Continue;
    };
    let srtp_key = msg.get("srtp_key").cloned().unwrap_or_default();

    let callee_status = match state.user_statuses.read() {
        Ok(statuses) => statuses.get(&to).cloned(),
        Err(_) => {
            ServerState::send_message(tx, "CALL_ERROR|error:internal server error");
            state
                .logger
                .error("No se pudo leer estados (lock envenenado)");
            return HandlerResult::Continue;
        }
    };

    if let Some(status) = callee_status {
        if status != UserStatus::Available {
            ServerState::send_message(tx, "CALL_ERROR|error:User not available");
            return HandlerResult::Continue;
        }

        let callee_sender = match state.connected_clients.read() {
            Ok(clients) => clients.get(&to).map(|c| c.sender.clone()),
            Err(_) => {
                state
                    .logger
                    .error("No se pudo leer clientes (lock envenenado)");
                None
            }
        };

        if let Some(callee_sender) = callee_sender {
            state.set_user_status(caller, UserStatus::Busy);
            state.set_user_status(&to, UserStatus::Busy);
            if let Ok(mut calls) = state.active_calls.write() {
                calls.insert(caller.clone(), to.clone());
                calls.insert(to.clone(), caller.clone());
            } else {
                state
                    .logger
                    .error("No se pudo registrar llamada (lock envenenado)");
            }

            let msg = format!("INCOMING_CALL|from:{}|sdp:{}|srtp_key:{}", caller, sdp, srtp_key);
            ServerState::send_message(&callee_sender, &msg);
            state.logger.info(&format!("{} llamó a {}", caller, to));
        } else {
            ServerState::send_message(tx, "CALL_ERROR|error:user not connected");
        }
    } else {
        ServerState::send_message(tx, "CALL_ERROR|error:User does not exist");
    }
    HandlerResult::Continue
}

/// Procesa el mensaje CALL_ANSWER.
pub fn handle_call_answer(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &Option<String>,
) -> HandlerResult {
    let Some(callee) = authenticated_user else {
        return HandlerResult::Continue;
    };

    let Some(to) = msg.get("to").cloned() else {
        ServerState::send_message(tx, "CALL_ERROR|error:missing destination");
        return HandlerResult::Continue;
    };
    let accept = msg.get("accept").map(|v| v == "true").unwrap_or(false);
    let sdp = msg.get("sdp").cloned();
    let srtp_key = msg.get("srtp_key").cloned().unwrap_or_default();

    let caller_sender = match state.connected_clients.read() {
        Ok(clients) => clients.get(&to).map(|c| c.sender.clone()),
        Err(_) => {
            state
                .logger
                .error("No se pudo leer clientes (lock envenenado)");
            None
        }
    };

    if let Some(caller_sender) = caller_sender {
        if accept {
            let Some(sdp_val) = sdp else {
                ServerState::send_message(&caller_sender, "CALL_REJECTED|from:server");
                return HandlerResult::Continue;
            };
            state.set_user_status(callee, UserStatus::Busy);
            let msg = format!(
                "CALL_ACCEPTED|from:{}|sdp:{}|srtp_key:{}",
                callee, sdp_val, srtp_key
            );
            ServerState::send_message(&caller_sender, &msg);
            state.logger.info(&format!("{} aceptó la llamada", callee));
        } else {
            let msg = format!("CALL_REJECTED|from:{}", callee);
            ServerState::send_message(&caller_sender, &msg);

            state.set_user_status(&to, UserStatus::Available);
            state.set_user_status(callee, UserStatus::Available);
            if let Ok(mut calls) = state.active_calls.write() {
                calls.remove(&to);
                calls.remove(callee);
            }
            state.logger.info(&format!("{} rechazó la llamada", callee));
        }
    }
    HandlerResult::Continue
}

/// Procesa el mensaje CALL_REJECT.
pub fn handle_call_reject(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &Option<String>,
) -> HandlerResult {
    let Some(callee) = authenticated_user else {
        return HandlerResult::Continue;
    };

    let Some(to) = msg.get("to").cloned() else {
        ServerState::send_message(tx, "CALL_ERROR|error:missing destination");
        return HandlerResult::Continue;
    };

    let caller_sender = match state.connected_clients.read() {
        Ok(clients) => clients.get(&to).map(|c| c.sender.clone()),
        Err(_) => {
            state
                .logger
                .error("No se pudo leer clientes (lock envenenado)");
            None
        }
    };
    if let Some(caller_sender) = caller_sender {
        let msg = format!("CALL_REJECTED|from:{}", callee);
        ServerState::send_message(&caller_sender, &msg);
    }

    state.set_user_status(&to, UserStatus::Available);
    state.set_user_status(callee, UserStatus::Available);
    if let Ok(mut calls) = state.active_calls.write() {
        calls.remove(&to);
        calls.remove(callee);
    }
    state.logger.info(&format!("{} rechazó la llamada", callee));
    HandlerResult::Continue
}

/// Procesa el mensaje CALL_END.
pub fn handle_call_end(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &Option<String>,
) -> HandlerResult {
    let Some(username) = authenticated_user else {
        return HandlerResult::Continue;
    };

    let Some(to) = msg.get("to").cloned() else {
        ServerState::send_message(tx, "CALL_ERROR|error:missing destination");
        return HandlerResult::Continue;
    };

    if let Ok(clients) = state.connected_clients.read()
        && let Some(other_client) = clients.get(&to)
    {
        let msg = format!("CALL_ENDED|from:{}", username);
        ServerState::send_message(&other_client.sender, &msg);
    }

    state.set_user_status(username, UserStatus::Available);
    state.set_user_status(&to, UserStatus::Available);

    if let Ok(mut calls) = state.active_calls.write() {
        calls.remove(username);
        calls.remove(&to);
    }
    state
        .logger
        .info(&format!("{} terminó la llamada con {}", username, to));
    HandlerResult::Continue
}

/// Procesa el mensaje ICE_CANDIDATE.
pub fn handle_ice_candidate(
    msg: &HashMap<String, String>,
    tx: &Sender<String>,
    state: &Arc<ServerState>,
    authenticated_user: &Option<String>,
) -> HandlerResult {
    let Some(from) = authenticated_user else {
        return HandlerResult::Continue;
    };

    let Some(to) = msg.get("to").cloned() else {
        ServerState::send_message(tx, "ERROR|error:missing destination");
        return HandlerResult::Continue;
    };
    let Some(candidate) = msg.get("candidate").cloned() else {
        ServerState::send_message(tx, "ERROR|error:missing candidate");
        return HandlerResult::Continue;
    };

    if let Ok(clients) = state.connected_clients.read()
        && let Some(to_client) = clients.get(&to)
    {
        let msg = format!("ICE_CANDIDATE|from:{}|candidate:{}", from, candidate);
        ServerState::send_message(&to_client.sender, &msg);
    }
    HandlerResult::Continue
}
