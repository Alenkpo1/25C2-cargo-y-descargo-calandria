//! Handler de presencia: GET_USERS.

use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::context::HandlerResult;
use crate::server::state::ServerState;

/// Procesa el mensaje GET_USERS.
pub fn handle_get_users(tx: &Sender<String>, state: &Arc<ServerState>) -> HandlerResult {
    let users = state.get_user_list();
    let mut response = String::from("USER_LIST");
    for (username, status) in users {
        response.push_str(&format!("|{}:{}", username, status.to_string()));
    }
    ServerState::send_message(tx, &response);
    HandlerResult::Continue
}
