use crate::client::signaling_client::{SignalingClient, SignalingEvent};
use crate::logger::Logger;
use eframe::egui::{self, Button, Vec2};
use egui::RichText;
use egui::TextStyle;
pub enum LoginAction {
    LoggedIn {
        username: String,
        signaling: SignalingClient,
    },
}

enum PendingAction {
    Login,
    RegisterThenLogin,
}

pub struct LoginScreen {
    pub username: String,
    pub password: String,
    pub server_addr: String,
    pub status_message: Option<String>,
    pending_client: Option<SignalingClient>,
    pending_action: Option<PendingAction>,
    logger: Option<Logger>,
}

impl LoginScreen {
    pub fn new(default_server: String, logger: Option<Logger>) -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            server_addr: default_server,
            status_message: None,
            pending_client: None,
            pending_action: None,
            logger,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) -> Option<LoginAction> {
        let mut login_result = None;

        while let Some(event) = self
            .pending_client
            .as_ref()
            .and_then(|client| client.try_next_event())
        {
            match event {
                SignalingEvent::Registered(_) => {
                    if matches!(self.pending_action, Some(PendingAction::RegisterThenLogin)) {
                        // REQUEST: LOGIN
                        if let Some(client) = self.pending_client.as_ref() {
                            let _ = client.login(&self.username, &self.password);
                        }
                        self.status_message = Some("User created, logging in...".into());
                        self.pending_action = Some(PendingAction::Login);
                    }
                }
                SignalingEvent::LoginSuccess(_) => {
                    if let Some(client) = self.pending_client.take() {
                        if let Some(log) = &self.logger {
                            log.info("Successful login to signaling server");
                        }
                        login_result = Some(LoginAction::LoggedIn {
                            username: self.username.clone(),
                            signaling: client,
                        });
                    }
                }
                SignalingEvent::LoginError(err)
                | SignalingEvent::RegisterError(err)
                | SignalingEvent::Error(err) => {
                    self.status_message = Some(err);
                    self.pending_client = None;
                    self.pending_action = None;
                }
                SignalingEvent::Disconnected => {
                    self.status_message = Some("Connection lost with the server".into());
                    self.pending_client = None;
                    self.pending_action = None;
                }
                _ => {}
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Center everything vertically and horizontally
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.15);
                
                // Login Card
                egui::Frame::none()
                    .fill(crate::ui::theme::colors::BACKGROUND_SECONDARY)
                    .rounding(crate::ui::screens::login::egui::Rounding::same(8.0))
                    .shadow(eframe::egui::Shadow::default())
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        ui.set_max_width(320.0);
                        
                        // Header
                        ui.heading(RichText::new("Welcome Back!").size(24.0).color(egui::Color32::WHITE));
                        ui.label(RichText::new("We're so excited to see you again!").color(crate::ui::theme::colors::TEXT_MUTED));
                        ui.add_space(20.0);
                        
                        // Inputs styling
                        let input_frame = |ui: &mut egui::Ui, label: &str, value: &mut String, password: bool, hint: &str| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(label.to_uppercase()).size(12.0).strong().color(crate::ui::theme::colors::TEXT_MUTED));
                            });
                            ui.add_space(4.0);
                            
                            let edit = egui::TextEdit::singleline(value)
                                .password(password)
                                .hint_text(hint)
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 10.0)); // Padding inside input
                                
                            ui.add(edit);
                            ui.add_space(16.0);
                        };

                        input_frame(ui, "Server Address", &mut self.server_addr, false, "127.0.0.1:8080");
                        input_frame(ui, "Username", &mut self.username, false, "Enter your username");
                        input_frame(ui, "Password", &mut self.password, true, "Enter your password");
                        
                        ui.add_space(10.0);

                        // Login Button (Primary)
                        let login_btn = Button::new(RichText::new("Log In").size(16.0).color(egui::Color32::WHITE))
                            .fill(crate::ui::theme::colors::PRIMARY)
                            .rounding(4.0)
                            .min_size(Vec2::new(f32::INFINITY, 44.0));
                        
                        if ui.add(login_btn).clicked() {
                            if let Ok(client) = SignalingClient::connect(&self.server_addr) {
                                let _ = client.login(&self.username, &self.password);
                                self.pending_client = Some(client);
                                self.pending_action = Some(PendingAction::Login);
                                self.status_message = Some("Logging in...".to_string());
                            } else {
                                self.status_message = Some("Could not connect to signaling server".to_string());
                            }
                        }
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Need an account?").color(crate::ui::theme::colors::TEXT_MUTED));
                            if ui.link("Register").clicked() {
                                if let Ok(client) = SignalingClient::connect(&self.server_addr) {
                                    let _ = client.register(&self.username, &self.password);
                                    self.pending_client = Some(client);
                                    self.pending_action = Some(PendingAction::RegisterThenLogin);
                                    self.status_message = Some("Registering...".to_string());
                                } else {
                                    self.status_message = Some("Could not connect to server".to_string());
                                }
                            }
                        });
                    });

                if let Some(status) = &self.status_message {
                    ui.add_space(20.0);
                    ui.label(RichText::new(status).color(crate::ui::theme::colors::DANGER));
                }
            });
        });

        login_result
    }
}
