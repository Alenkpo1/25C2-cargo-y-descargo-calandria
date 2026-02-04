use crate::client::signaling_client::{SignalingClient, SignalingEvent};
use crate::logger::Logger;
use crate::ui::screens::login::egui::Color32;
use crate::ui::screens::login::egui::Rounding;
use crate::ui::screens::login::egui::Stroke;
use crate::ui::screens::login::egui::Vec2;
use eframe::egui::{self, Button};
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
            ui.heading("Login");
            ui.separator();

            ui.vertical_centered(|ui| {
                ui.group(|ui| {
                    ui.set_width(250.0);
                    let mut visuals = ui.style().visuals.clone();
                    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::BLACK);
                    visuals.widgets.inactive.rounding = Rounding::from(2.0);
                    ui.style_mut().visuals = visuals;

                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.label("Server:");
                    });

                    ui.add(
                        egui::TextEdit::singleline(&mut self.server_addr)
                            .desired_width(f32::INFINITY)
                            .hint_text("Enter text here..."),
                    );
                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.label("User:");
                    });
                    ui.add(
                        egui::TextEdit::singleline(&mut self.username)
                            .desired_width(f32::INFINITY)
                            .hint_text("Enter username here..."),
                    );

                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.label("Password:");
                    });
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                });
            });
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                let login_btn = Button::new(
                    RichText::new("Login")
                        .text_style(TextStyle::Button)
                        .color(egui::Color32::WHITE)
                        .size(20.0),
                )
                .fill(egui::Color32::BLUE)
                .rounding(egui::Rounding::same(10.0))
                .min_size(Vec2::new(180.0, 40.0));
                let res_login_btn = ui.add(login_btn);

                if res_login_btn.clicked() {
                    // REQUEST: LOGIN
                    if let Ok(client) = SignalingClient::connect(&self.server_addr) {
                        let _ = client.login(&self.username, &self.password);
                        self.pending_client = Some(client);
                        self.pending_action = Some(PendingAction::Login);
                        self.status_message = Some("Logging in...".to_string());
                    } else {
                        self.status_message =
                            Some("Could not connect to signaling server".to_string());
                    }
                }
                ui.add_space(10.0);
                ui.separator();
                ui.label("Need an account? Type your credentials and");
                let sign_up = Button::new(
                    RichText::new("SIGN UP")
                        .text_style(TextStyle::Button)
                        .size(15.0)
                        .underline(),
                );
                let res_sign_up = ui.add(sign_up);
                if res_sign_up.clicked() {
                    if let Ok(client) = SignalingClient::connect(&self.server_addr) {
                        let _ = client.register(&self.username, &self.password);
                        self.pending_client = Some(client);
                        self.pending_action = Some(PendingAction::RegisterThenLogin);
                        self.status_message = Some("Signing up user on the server...".to_string());
                        if let Some(log) = &self.logger {
                            log.info("User registration sent");
                        }
                    } else {
                        self.status_message =
                            Some("Could not connect to signaling server".to_string());
                    }
                }
            });

            if let Some(status) = &self.status_message {
                ui.separator();
                ui.label(status);
            }
        });

        login_result
    }
}
