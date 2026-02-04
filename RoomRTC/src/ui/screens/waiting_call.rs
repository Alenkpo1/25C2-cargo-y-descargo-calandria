use crate::client::p2p_client::P2PClient;
use crate::client::signaling_client::SignalingClient;
use crate::client::webrtc_service::WebRTCHandler;
use eframe::egui::{self, Button};
use egui::RichText;
use egui::TextStyle;
use egui::Vec2;
use room_rtc::rtc::rtc_peer_connection::PeerConnectionRole;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum WaitingCallAction {
    GoToLobby,
    GoToVideo,
}
pub struct WaitingCall {
    pub local_sdp: String,
    pub role: PeerConnectionRole,
    pub target_username: String,
    received_msgs: Arc<Mutex<Vec<String>>>,
    pub client: Option<P2PClient>,
    pub remote_sdp: String,
    ice_started: bool,
    pub status_message: Option<String>,
    active_peer: Option<String>,
}

impl WebRTCHandler for WaitingCall {
    fn client(&mut self) -> &mut Option<P2PClient> {
        &mut self.client
    }

    fn role(&self) -> PeerConnectionRole {
        self.role
    }
    fn received_msgs(&self) -> &Arc<Mutex<Vec<String>>> {
        &self.received_msgs
    }
}

impl WaitingCall {
    pub fn new(role: PeerConnectionRole) -> Self {
        Self {
            local_sdp: String::new(),
            role,
            target_username: String::new(),
            received_msgs: Arc::new(Mutex::new(Vec::new())),
            client: None,
            remote_sdp: String::new(),
            ice_started: false,
            status_message: None,
            active_peer: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) -> Option<WaitingCallAction> {
        let mut next_action = None;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading(format!("Calling {}", self.target_username));
            let res_go_lobby = ui.add(Button::new("Go to Lobby"));
            if res_go_lobby.clicked() {
                println!("Returning to Lobby");
                next_action = Some(WaitingCallAction::GoToLobby);
            }

            /* DEBUG */
            ui.horizontal(|ui| {
                ui.label("Client status:");
                if self.client.is_some() {
                    ui.colored_label(egui::Color32::GREEN, "INITIALIZED");
                    if let Some(client) = self.client.as_ref() {
                        ui.label(format!("Role: {:?}", client.role()));
                        match client.local_addr() {
                            Ok(addr) => ui.label(format!("Addr: {}", addr)),
                            Err(err) => ui.label(format!("Addr error: {}", err)),
                        };
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "NOT INITIALIZED");
                }
            });
            /* END DEBUG */
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if let Some(status) = &self.status_message {
                    ui.separator();
                    ui.label(status);
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "Waiting for {} to accept the call...",
                            self.target_username
                        ))
                        .size(20.0)
                        .color(egui::Color32::DARK_BLUE),
                    );
                }
                ui.separator();

                let go_meet = Button::new(
                    RichText::new("🙌 Join meeting")
                        .text_style(TextStyle::Button)
                        .size(20.0),
                )
                .fill(egui::Color32::LIGHT_BLUE)
                .rounding(egui::Rounding::same(10.0))
                .min_size(Vec2::new(200.0, 50.0));
                let go_meet_btn = ui.add(go_meet);

                if go_meet_btn.clicked() {
                    if self.client.is_none() {
                        self.status_message = Some(
                            "Inicializa el peer y comparte la oferta antes de entrar.".to_string(),
                        );
                    } else {
                        if !self.ice_started {
                            match self.start_ice() {
                                Ok(_) => {
                                    self.ice_started = true;
                                    self.status_message =
                                        Some("ICE iniciado, esperando conexión...".to_string());
                                }
                                Err(e) => {
                                    eprintln!("ICE ERROR {}", e);
                                    self.status_message =
                                        Some(format!("Error iniciando ICE: {}", e));
                                    return;
                                }
                            }
                            self.status_message = Some("Conectando... Por favor espere.".to_string());
                        } else if let Some(client) = &self.client {
                            // Solo entramos si la conexión (ICE + DTLS) está completa
                            if client.has_connection() {
                                self.status_message = Some("Entrando a la sala de video...".to_string());
                                next_action = Some(WaitingCallAction::GoToVideo);
                            } else {
                                self.status_message = Some("Esperando a que finalice la conexión...".to_string());
                            }
                        }
                    }
                }
            });
        });
        next_action
    }

    pub fn take_client_with_inbox(&mut self) -> Option<(P2PClient, Arc<Mutex<Vec<String>>>)> {
        if let Some(client) = self.client.take() {
            let inbox = Arc::clone(&self.received_msgs);
            self.received_msgs = Arc::new(Mutex::new(Vec::new()));
            return Some((client, inbox));
        }
        None
    }

    pub fn on_call_accepted(&mut self, from: String, sdp: String) {
        self.active_peer = Some(from.clone());
        self.remote_sdp = sdp.clone();
        if let Err(err) = self.apply_remote_description(&sdp) {
            self.status_message = Some(format!("Error aplicando SDP remoto: {}", err));
            return;
        }
        if let Err(err) = self.start_ice() {
            self.status_message = Some(format!("Error iniciando ICE: {}", err));
            return;
        }
        self.ice_started = true;
        self.status_message = Some(format!("{} aceptó la llamada", from));
        // Pasar directamente a la sala de video
        self.status_message = Some("Entrando a la sala de video...".to_string());
    }

    pub fn on_call_rejected(&mut self, from: String) {
        self.status_message = Some(format!("{} rechazó tu llamada", from));
        self.active_peer = None;
    }

    pub fn on_call_ended(&mut self, from: &str) {
        if self.active_peer.as_deref() == Some(from) {
            self.status_message = Some(format!("{} colgó la llamada", from));
            self.active_peer = None;
            self.client = None;
            self.remote_sdp.clear();
            self.local_sdp.clear();
            self.ice_started = false;
        }
    }

    pub fn active_peer(&self) -> Option<String> {
        self.active_peer.clone()
    }

    fn place_call(&mut self, signaling: &SignalingClient) -> Result<(), String> {
        if self.target_username.trim().is_empty() {
            return Err("Input user to call".to_string());
        }

        self.initialize_peer()
            .map_err(|e| format!("Error initializing peer: {}", e))?;

        self.ice_started = false;
        let offer = self
            .generate_offer()
            .map_err(|e| format!("Couldn't generate offer: {}", e))?;

        signaling
            .call(&self.target_username, &offer)
            .map_err(|e| e.to_string())?;
        self.local_sdp = offer;
        self.active_peer = Some(self.target_username.clone());
        Ok(())
    }

    pub fn call_user(&mut self, username: &str, signaling: &SignalingClient) -> Result<(), String> {
        self.target_username = username.to_string();
        self.place_call(signaling)
    }
}
