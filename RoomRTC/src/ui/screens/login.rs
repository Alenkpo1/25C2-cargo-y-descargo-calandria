use crate::client::signaling_client::{SignalingClient, SignalingEvent};
use crate::logger::Logger;
use crate::ui::theme::colors;
use eframe::epaint::Margin;
use eframe::egui::{Color32, Rounding, Stroke, Vec2};
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

        egui::CentralPanel::default().frame(
            egui::Frame::none()
                .fill(colors::BACKGROUND)
        ).show(ctx, |ui| {

            // Fondo plano oscuro
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, colors::BACKGROUND);

            ui.vertical_centered(|ui| {
                ui.set_max_width(620.0);
                ui.add_space(28.0);

                // Encabezado compacto
                egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 12))
                    .rounding(Rounding::same(999.0))
                    .inner_margin(Margin::symmetric(14.0, 8.0))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("RoomRTC").strong().color(colors::TEXT_PRIMARY));
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Reuniones nitidas y rapidas")
                                    .size(13.0)
                                    .color(colors::TEXT_MUTED),
                            );
                        });
                    });

                ui.add_space(12.0);
                ui.label(
                    RichText::new("Bienvenido de nuevo")
                        .size(30.0)
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new("Organiza tus llamadas y comparte tu sala en segundos.")
                        .size(16.0)
                        .color(colors::TEXT_MUTED),
                );

                ui.add_space(22.0);

                // Contenedor principal "glass"
                egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(16, 17, 24, 220))
                    .rounding(Rounding::same(18.0))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 26)))
                    .inner_margin(Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.set_width(520.0);

                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 14.0;

                            ui.label(
                                RichText::new("Datos de acceso")
                                    .size(18.0)
                                    .color(colors::TEXT_PRIMARY)
                                    .strong(),
                            );
                            ui.separator();

                            // Campo de servidor
                            ui.label(
                                RichText::new("Servidor")
                                    .size(13.0)
                                    .color(colors::TEXT_MUTED)
                                    .strong(),
                            );
                            egui::Frame::none()
                                .fill(colors::BACKGROUND_TERTIARY)
                                .rounding(Rounding::same(10.0))
                                .stroke(Stroke::new(1.0, colors::BORDER))
                                .inner_margin(Margin::symmetric(12.0, 10.0))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.server_addr)
                                            .desired_width(f32::INFINITY)
                                            .hint_text("wss://servidor:puerto")
                                            .frame(false)
                                            .font(TextStyle::Body),
                                    );
                                });

                            // Campo de usuario
                            ui.label(
                                RichText::new("Usuario")
                                    .size(13.0)
                                    .color(colors::TEXT_MUTED)
                                    .strong(),
                            );
                            egui::Frame::none()
                                .fill(colors::BACKGROUND_TERTIARY)
                                .rounding(Rounding::same(10.0))
                                .stroke(Stroke::new(1.0, colors::BORDER))
                                .inner_margin(Margin::symmetric(12.0, 10.0))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.username)
                                            .desired_width(f32::INFINITY)
                                            .hint_text("tu usuario")
                                            .frame(false),
                                    );
                                });

                            // Campo de contrasena
                            ui.label(
                                RichText::new("Contrasena")
                                    .size(13.0)
                                    .color(colors::TEXT_MUTED)
                                    .strong(),
                            );
                            egui::Frame::none()
                                .fill(colors::BACKGROUND_TERTIARY)
                                .rounding(Rounding::same(10.0))
                                .stroke(Stroke::new(1.0, colors::BORDER))
                                .inner_margin(Margin::symmetric(12.0, 10.0))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .desired_width(f32::INFINITY)
                                            .password(true)
                                            .frame(false)
                                            .hint_text("********"),
                                    );
                                });

                            ui.add_space(4.0);

                            // Boton de accion
                            let login_btn = Button::new(
                                RichText::new("Ingresar")
                                    .size(17.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(colors::PRIMARY)
                            .min_size(Vec2::new(ui.available_width(), 46.0))
                            .rounding(12.0);

                            if ui.add(login_btn).clicked() {
                                if let Ok(client) = SignalingClient::connect(&self.server_addr) {
                                    let _ = client.login(&self.username, &self.password);
                                    self.pending_client = Some(client);
                                    self.pending_action = Some(PendingAction::Login);
                                    self.status_message = Some("Logging in...".into());
                                } else {
                                    self.status_message = Some("Cannot connect to server".into());
                                }
                            }

                            // Enlace de registro y estado
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Eres nuevo?")
                                        .color(colors::TEXT_MUTED)
                                        .size(13.0),
                                );
                                if ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new("Crear cuenta")
                                                .underline()
                                                .color(colors::PRIMARY)
                                                .size(13.5),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                {
                                    if let Ok(client) = SignalingClient::connect(&self.server_addr) {
                                        let _ = client.register(&self.username, &self.password);
                                        self.pending_client = Some(client);
                                        self.pending_action = Some(PendingAction::RegisterThenLogin);
                                        self.status_message = Some("Registering...".into());
                                    } else {
                                        self.status_message = Some("Cannot connect to server".into());
                                    }
                                }
                            });

                            if let Some(status) = &self.status_message {
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(status)
                                        .color(Color32::LIGHT_RED)
                                        .size(14.0),
                                );
                            }
                        });
                    });

                ui.add_space(32.0);
            });
        });

        login_result
    }
}
