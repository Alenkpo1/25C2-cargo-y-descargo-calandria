use crate::client::signaling_client::SignalingClient;
use crate::ui::screens::status_utils::ui_status;
use eframe::egui::{self};

pub enum LobbyAction {
    GoToWaitingCall(String),
    Logout,
}

pub struct LobbyScreen {
    err_message: Option<String>,
    users: Vec<(String, String)>,
    status_message: Option<String>,
}

impl eframe::App for LobbyScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update(ctx, None, None);
    }
}

impl LobbyScreen {
    pub fn new() -> Self {
        Self {
            err_message: None,
            users: Vec::new(),
            status_message: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        signaling: Option<&SignalingClient>,
        current_user: Option<&str>,
    ) -> Option<LobbyAction> {
        let mut next_action = None;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            #[allow(clippy::manual_unwrap_or)]
            let user_display_name = match current_user {
                Some(name) => name,
                None => "User",
            };
            ui.heading(format!("{}'s Lobby", user_display_name));
            if let Some(status) = &self.status_message {
                ui.colored_label(egui::Color32::LIGHT_GREEN, status);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(egui::RichText::new("🙋 Peers").size(20.0));

                // REQUEST: REQUEST USERS
                if let Some(signaling) = signaling
                    && ui.button("Refresh").clicked()
                {
                    let _ = signaling.request_users();
                }
            });

            // User list with video call button
            if self.users.is_empty() {
                ui.label("No users signed up yet.");
            } else {
                for (user, status) in &self.users {
                    if let Some(action) = ui_status::user_row(ui, user, status, current_user) {
                        next_action = Some(action);
                    }
                }
            }
            ui.separator();

            // Sends to login screen
            if let Some(signaling) = signaling
                && ui.button("Log Off").clicked()
            {
                // REQUEST: LOGOUT
                let _ = signaling.logout();
                self.status_message = Some("Session closed".to_string());
                next_action = Some(LobbyAction::Logout);
            }

            if let Some(err) = &self.err_message {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
            }
        });
        next_action
    }

    pub fn set_users(&mut self, users: Vec<(String, String)>) {
        self.users = users;
        self.status_message = Some("Updated user list".to_string());
    }

    pub fn update_user_status(&mut self, username: String, status: String) {
        if let Some(entry) = self.users.iter_mut().find(|(u, _)| u == &username) {
            entry.1 = status.clone();
        } else {
            self.users.push((username.clone(), status.clone()));
        }
        self.status_message = Some(format!("{} -> {}", username, status));
    }
}
