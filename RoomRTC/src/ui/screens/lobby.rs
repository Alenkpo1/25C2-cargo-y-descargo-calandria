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

        // Top/Side Panel for User Info
        egui::SidePanel::left("lobby_sidebar")
            .resizable(false)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    // Avatar/Icon placeholder
                    ui.label(egui::RichText::new("👤").size(60.0));
                    ui.add_space(10.0);
                    
                    #[allow(clippy::manual_unwrap_or)]
                    let user_display_name = match current_user {
                        Some(name) => name,
                        None => "Unknown",
                    };
                    
                    ui.heading(egui::RichText::new(user_display_name).size(20.0).color(egui::Color32::WHITE));
                    ui.label(egui::RichText::new("Online").color(crate::ui::theme::colors::SUCCESS));
                });
                
                ui.add_space(40.0);
                ui.separator();
                ui.add_space(20.0);
                
                // Actions in Sidebar
                ui.vertical_centered(|ui| {
                    if let Some(signaling) = signaling {
                        let refresh_btn = egui::Button::new(egui::RichText::new("🔄 Refresh List").size(14.0))
                            .fill(crate::ui::theme::colors::BACKGROUND_SECONDARY)
                            .min_size(egui::vec2(180.0, 40.0));
                            
                        if ui.add(refresh_btn).clicked() {
                             let _ = signaling.request_users();
                        }
                        
                        ui.add_space(10.0);
                        
                        // Debug/Error box in sidebar
                        if let Some(err) = &self.err_message {
                            ui.colored_label(crate::ui::theme::colors::DANGER, format!("Error: {}", err));
                        }
                    }
                });
                
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                   ui.add_space(20.0);
                   if let Some(signaling) = signaling {
                        let logout_btn = egui::Button::new(egui::RichText::new("� Log Out").size(14.0).color(egui::Color32::WHITE))
                            .fill(crate::ui::theme::colors::DANGER)
                            .rounding(4.0)
                            .min_size(egui::vec2(180.0, 40.0));

                        if ui.add(logout_btn).clicked() {
                            let _ = signaling.logout();
                            self.status_message = Some("Session closed".to_string());
                            next_action = Some(LobbyAction::Logout);
                        }
                   }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(20.0);
            ui.heading(egui::RichText::new("Active Users").size(28.0).strong().color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("Connect with peers in the room").color(crate::ui::theme::colors::TEXT_MUTED));
            ui.add_space(30.0);

            if let Some(status) = &self.status_message {
                 ui.colored_label(crate::ui::theme::colors::SUCCESS, status);
                 ui.add_space(10.0);
            }

            // User list grid
            if self.users.is_empty() {
                ui.centered_and_justified(|ui| {
                   ui.label(egui::RichText::new("No other users found.\nTry clicking Refresh.").size(18.0).color(crate::ui::theme::colors::TEXT_MUTED)); 
                });
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                    
                    for (user, status) in &self.users {
                        // Custom Card for each user
                        egui::Frame::none()
                            .fill(crate::ui::theme::colors::BACKGROUND_SECONDARY)
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Status Dot
                                    let dot_color = if status == "AVAILABLE" { crate::ui::theme::colors::SUCCESS } else { crate::ui::theme::colors::DANGER };
                                    ui.painter().circle_filled(ui.cursor().min + egui::vec2(5.0, 10.0), 5.0, dot_color);
                                    ui.add_space(15.0);
                                    
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(user).size(16.0).strong().color(egui::Color32::WHITE));
                                        ui.label(egui::RichText::new(status).size(12.0).color(crate::ui::theme::colors::TEXT_MUTED));
                                    });
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                         if ui_status::Status::Connected.is_callable(user, current_user) && status == "AVAILABLE" {
                                             let call_btn = egui::Button::new(egui::RichText::new("📞 Call").color(egui::Color32::WHITE))
                                                .fill(crate::ui::theme::colors::SUCCESS)
                                                .rounding(20.0)
                                                .min_size(egui::vec2(80.0, 30.0));
                                                
                                             if ui.add(call_btn).clicked() {
                                                 next_action = Some(LobbyAction::GoToWaitingCall(user.to_string()));
                                             }
                                         }
                                    });
                                });
                            });
                    }
                });
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
