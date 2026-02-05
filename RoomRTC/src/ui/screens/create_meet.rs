use crate::client::p2p_client::P2PClient;
use crate::client::signaling_client::SignalingClient;
use crate::client::webrtc_service::WebRTCHandler;
use eframe::egui::{self, Button, RichText};
use room_rtc::rtc::rtc_peer_connection::PeerConnectionRole;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum CreateMeetAction {
    GoToLobby,
    GoToVideo,
}
pub struct CreateMeetScreen {
    pub local_sdp: String,
    pub role: PeerConnectionRole,
    pub target_username: String,

    outgoing_msg: String,
    received_msgs: Arc<Mutex<Vec<String>>>,
    pub client: Option<P2PClient>,
    pub remote_sdp: String,
    ice_started: bool,
    active_peer: Option<String>,
}

impl WebRTCHandler for CreateMeetScreen {
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

impl CreateMeetScreen {
    pub fn new(role: PeerConnectionRole) -> Self {
        Self {
            local_sdp: String::new(),
            role,
            target_username: String::new(),
            //Refactor these fields later
            outgoing_msg: String::new(),
            received_msgs: Arc::new(Mutex::new(Vec::new())),
            client: None,
            remote_sdp: String::new(),
            ice_started: false,
            active_peer: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        signaling: Option<&SignalingClient>,
    ) -> Option<CreateMeetAction> {
        let mut next_action = None;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("Create Meeting");

            let res_go_lobby = ui.add(Button::new("Go to Lobby"));
            if res_go_lobby.clicked() {
                println!("Returning to Lobby");
                next_action = Some(CreateMeetAction::GoToLobby);
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
                ui.add_space(20.0);
                
                // Call Card
                egui::Frame::none()
                    .fill(crate::ui::theme::colors::BACKGROUND_SECONDARY)
                    .rounding(8.0)
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        ui.set_max_width(400.0);
                        ui.heading(RichText::new("Start a Call").size(20.0).color(egui::Color32::WHITE));
                        ui.add_space(10.0);

                        // Call Input
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("USERNAME").size(12.0).strong().color(crate::ui::theme::colors::TEXT_MUTED));
                        });
                        let user_input = egui::TextEdit::singleline(&mut self.target_username)
                            .hint_text("Enter username to call")
                            .desired_width(f32::INFINITY)
                            .margin(egui::vec2(10.0, 10.0));
                        ui.add(user_input);
                        
                        ui.add_space(20.0);
                        
                        // Call Button
                        let call_btn = Button::new(RichText::new("Call").size(16.0).color(egui::Color32::WHITE))
                            .fill(crate::ui::theme::colors::SUCCESS) // Green for call
                            .rounding(4.0)
                            .min_size(egui::vec2(f32::INFINITY, 44.0));
                            
                        if ui.add(call_btn).clicked() {
                            if let Some(signaling) = signaling {
                                match self.place_call(signaling) {
                                    Ok(_) => {
                                        self.status_message =
                                            Some(format!("Offer sent to {}", self.target_username));
                                    }
                                    Err(err) => {
                                        self.status_message = Some(format!("Error sending call: {}", err))
                                    }
                                }
                            } else {
                                self.status_message = Some("Signaling Server unavailable".to_string());
                            }
                        }
                    });
                    
                ui.add_space(20.0);

                // Advanced / Debug section (Collapsible)
                ui.collapsing("Advanced Debug Info", |ui| {
                    ui.horizontal(|ui| {
                         ui.label("Client status:");
                         if self.client.is_some() {
                             ui.colored_label(crate::ui::theme::colors::SUCCESS, "INITIALIZED");
                         } else {
                             ui.colored_label(crate::ui::theme::colors::DANGER, "NOT INITIALIZED");
                         }
                    });
                    
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Your SDP offer:");
                            ui.add_sized(
                                [300.0, 100.0],
                                egui::TextEdit::multiline(&mut self.local_sdp),
                            );
                            if ui.button("Copy SDP").clicked() {
                                ctx.output_mut(|o| o.copied_text = self.local_sdp.clone());
                            }
                        });

                        ui.vertical(|ui| {
                            ui.label("Remote SDP:");
                            ui.add_sized(
                                [300.0, 100.0],
                                egui::TextEdit::multiline(&mut self.remote_sdp),
                            );
                        });
                    });
                });
                
                ui.add_space(20.0);
                
                if let Some(status) = &self.status_message {
                    ui.label(RichText::new(status).color(crate::ui::theme::colors::TEXT_PRIMARY));
                }

                // Chat / Messages area
                if self.active_peer.is_some() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.outgoing_msg);
                        if ui.button("Send Msg").clicked() {
                             let _ = self.send_message(&self.outgoing_msg.clone());
                             self.outgoing_msg.clear();
                        }
                    });
                    
                    if let Ok(messages) = self.received_msgs.lock() {
                        for msg in messages.iter() {
                            ui.label(RichText::new(msg).color(crate::ui::theme::colors::TEXT_MUTED));
                        }
                    }
                }
                
                // Transition Button (if ready)
                if self.client.is_some() {
                     ui.add_space(20.0);
                     let join_btn = Button::new(RichText::new("Join Video Room").size(16.0))
                        .fill(crate::ui::theme::colors::PRIMARY)
                        .rounding(4.0)
                        .min_size(egui::vec2(200.0, 44.0));
                        
                     if ui.add(join_btn).clicked() {
                         next_action = Some(CreateMeetAction::GoToVideo);
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
            self.ice_started = false;
            self.remote_sdp.clear();
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
        // En lugar de start_ice, llamamos a establish_connection
        if let Some(client) = &mut self.client {
            let _ = client.establish_connection();
            self.ice_started = true;
        }
        self.status_message = Some(format!("{} aceptó la llamada", from));
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
