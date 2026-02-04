use crate::client::p2p_client::P2PClient;
use crate::client::signaling_client::SignalingClient;
use crate::client::webrtc_service::WebRTCHandler;
use eframe::egui::{self, Button};
use egui::RichText;
use egui::TextStyle;
use egui::Vec2;
use room_rtc::rtc::rtc_peer_connection::PeerConnectionRole;
use std::sync::{Arc, Mutex};
pub enum JoinMeetAction {
    GoToLobby,
    GoToVideo,
}
pub struct JoinMeetScreen {
    pub local_sdp: String,

    role: PeerConnectionRole,
    outgoing_msg: String,
    received_msgs: Arc<Mutex<Vec<String>>>,
    client: Option<P2PClient>,
    pub remote_sdp: String,
    ice_started: bool,
    status_message: Option<String>,
    incoming_from: Option<String>,
    active_peer: Option<String>,
}

impl WebRTCHandler for JoinMeetScreen {
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

impl JoinMeetScreen {
    pub fn new(role: PeerConnectionRole) -> Self {
        Self {
            local_sdp: String::new(),
            role,
            //Refactor these fields later
            outgoing_msg: String::new(),
            received_msgs: Arc::new(Mutex::new(Vec::new())),
            client: None,
            remote_sdp: String::new(),
            ice_started: false,
            status_message: None,
            incoming_from: None,
            active_peer: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        signaling: Option<&SignalingClient>,
    ) -> Option<JoinMeetAction> {
        let mut next_action = None;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("Join Meeting");

            let res_go_lobby = ui.add(Button::new("Go to Lobby"));
            if res_go_lobby.clicked() {
                println!("Returning to Lobby");
                next_action = Some(JoinMeetAction::GoToLobby);
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
            // Shows the incoming call screen
            if let Some(status) = &self.status_message {
                ui.separator();
                ui.label(status);
            }
            if self.incoming_from.is_some() {
                if let Some(from) = &self.incoming_from {
                    ui.label(format!("{} is calling you", from));
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            let accept_btn = Button::new(
                                RichText::new("📞").text_style(TextStyle::Button).size(20.0),
                            )
                            .fill(egui::Color32::LIGHT_GREEN)
                            .rounding(egui::Rounding::same(10.0))
                            .min_size(Vec2::new(100.0, 50.0));
                            let res_accept_btn = ui.add(accept_btn);
                            if res_accept_btn.clicked() {
                                if let Some(signaling) = signaling {
                                    match self.accept_current_call(signaling) {
                                        Ok(_) => {
                                            self.status_message =
                                                Some("Answer sent... Starting ICE...".into());
                                            next_action = Some(JoinMeetAction::GoToVideo);
                                        }
                                        Err(err) => self.status_message = Some(err),
                                    }
                                } else {
                                    self.status_message =
                                        Some("First connect to the signaling server.".to_string());
                                }
                            }
                            ui.add_space(20.0);
                            let decline_btn = Button::new(
                                RichText::new("☎").text_style(TextStyle::Button).size(20.0),
                            )
                            .fill(egui::Color32::LIGHT_RED)
                            .rounding(egui::Rounding::same(10.0))
                            .min_size(Vec2::new(100.0, 50.0));
                            let res_decline_btn = ui.add(decline_btn);
                            if res_decline_btn.clicked() {
                                if let Some(signaling) = signaling
                                    && let Some(peer) = &self.incoming_from
                                {
                                    let _ = signaling.reject_call(peer);
                                }
                                self.incoming_from = None;
                                self.active_peer = None;
                                self.status_message = Some("Call was declined".to_string());
                            }
                        });
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
                                    "Wait for a call and join after accepting it.".to_string(),
                                );
                            } else {
                                if !self.ice_started {
                                    if let Some(result) = self.ensure_peer_and_start_ice() {
                                        if let Err(err) = result {
                                            self.status_message = Some(format!("Error: {}", err));
                                        } else {
                                            self.status_message = Some("Iniciando conexión...".to_string());
                                        }
                                    }
                                } else if let Some(client) = &self.client {
                                    if client.has_connection() {
                                        self.status_message = Some("Joining meeting".to_string());
                                        next_action = Some(JoinMeetAction::GoToVideo);
                                    } else {
                                        self.status_message = Some("Esperando conexión...".to_string());
                                    }
                                }
                            }
                        }
                    });
                } else {
                    ui.label("Waiting for incoming calls...");
                }
            } else {
                // Shows the default screen
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("Respuesta SDP local");
                    ui.add(egui::TextEdit::multiline(&mut self.local_sdp).desired_rows(6));

                    let sdp_copy_btn = Button::new("Click to copy");
                    let res_sdp_copy_btn = ui.add(sdp_copy_btn);

                    if res_sdp_copy_btn.clicked() {
                        ctx.output_mut(|o| o.copied_text = self.local_sdp.clone());
                        println!("SDP copied");
                    }
                });
                ui.separator();
                let ice_starter = ui.add(Button::new("Start ice"));
                if ice_starter.clicked() {
                    if self.ice_started {
                        self.status_message = Some("ICE ya está iniciado".to_string());
                    } else if let Some(result) = self.ensure_peer_and_start_ice()
                        && let Err(err) = result
                    {
                        eprintln!("ICE ERROR {}", err);
                        self.status_message = Some(format!("Error iniciando ICE: {}", err));
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Message");
                    ui.text_edit_singleline(&mut self.outgoing_msg);
                });

                if ui.button("Send").clicked()
                    && let Err(err) = self.send_message(&self.outgoing_msg.clone())
                {
                    eprintln!("Error: {:?}", err);
                    return;
                }

                ui.separator();
                ui.label("received messages:");
                if let Ok(messages) = self.received_msgs.lock() {
                    for msg in messages.iter() {
                        ui.label(msg);
                    }
                }

                if let Some(status) = &self.status_message {
                    ui.separator();
                    ui.label(status);
                }

                ui.separator();
                let go_meet = ui.add(Button::new("Go to meet"));
                if go_meet.clicked() {
                    println!("Joining meet");
                    if self.client.is_none() {
                        self.status_message = Some(
                            "Espera una llamada y acéptala antes de entrar al video.".to_string(),
                        );
                    } else {
                        if !self.ice_started {
                            if let Some(result) = self.ensure_peer_and_start_ice() {
                                if let Err(err) = result {
                                    self.status_message = Some(format!("Error: {}", err));
                                } else {
                                    self.status_message = Some("Iniciando conexión...".to_string());
                                }
                            }
                        } else if let Some(client) = &self.client {
                            if client.has_connection() {
                                self.status_message = Some("Entrando a la sala de video...".to_string());
                                next_action = Some(JoinMeetAction::GoToVideo);
                            } else {
                                self.status_message = Some("Esperando conexión...".to_string());
                            }
                        }
                    }
                }
            }
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

    fn ensure_peer_and_start_ice(
        &mut self,
    ) -> Option<Result<(), room_rtc::rtc::rtc_peer_connection::PeerConnectionError>> {
        if self.client.is_none()
            && let Err(err) = self.initialize_peer()
        {
            self.status_message = Some(format!("Error iniciando peer: {}", err));
            return None;
        }
        self.client.as_mut()?;
        match self.start_ice() {
            Ok(_) => {
                self.ice_started = true;
                self.status_message = Some("ICE iniciado, esperando conexión...".to_string());
                Some(Ok(()))
            }
            Err(err) => Some(Err(err)),
        }
    }

    pub fn on_incoming_call(&mut self, from: String, sdp: String) {
        self.remote_sdp = sdp;
        self.incoming_from = Some(from.clone());
        self.active_peer = Some(from.clone());
        self.status_message = Some(format!("Llamada entrante de {}", from));
    }

    pub fn on_call_ended(&mut self, from: &str) {
        if self.active_peer.as_deref() == Some(from) {
            self.status_message = Some(format!("{} colgó la llamada", from));
            self.incoming_from = None;
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

    fn accept_current_call(&mut self, signaling: &SignalingClient) -> Result<(), String> {
        let Some(caller) = self.incoming_from.clone() else {
            return Err("No hay ninguna llamada entrante".to_string());
        };
        self.initialize_peer()
            .map_err(|e| format!("No se pudo iniciar el peer: {}", e))?;
        let remote_sdp = self.remote_sdp.clone();
        let answer = self
            .process_remote_offer(&remote_sdp)
            .map_err(|e| format!("No se pudo procesar la oferta: {}", e))?;
        signaling
            .answer_call(&caller, &answer)
            .map_err(|e| e.to_string())?;
        self.local_sdp = answer;
        if let Err(err) = self.start_ice() {
            self.status_message = Some(format!("Error iniciando ICE: {}", err));
        } else {
            self.ice_started = true;
        }
        Ok(())
    }
}
