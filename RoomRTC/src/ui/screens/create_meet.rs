use crate::client::p2p_client::P2PClient;
use crate::client::signaling_client::SignalingClient;
use crate::client::webrtc_service::WebRTCHandler;
use eframe::egui::{self, Button};
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
            ui.horizontal(|ui| {
                ui.label("Enter user to call:");
                ui.text_edit_singleline(&mut self.target_username);
                if ui.button("Call").clicked() {
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

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Your SDP offer:");
                    ui.add_sized(
                        [300.0, 100.0],
                        egui::TextEdit::multiline(&mut self.local_sdp),
                    );
                    let sdp_copy_btn = Button::new("Click to copy");
                    let res_sdp_copy_btn = ui.add(sdp_copy_btn);

                    if res_sdp_copy_btn.clicked() {
                        ctx.output_mut(|o| o.copied_text = self.local_sdp.clone());
                        println!("SDP copied");
                    }
                });

                ui.vertical(|ui| {
                    ui.label("Input peer's SDP offer:");
                    ui.add_sized(
                        [300.0, 100.0],
                        egui::TextEdit::multiline(&mut self.remote_sdp),
                    );
                    if ui.button("Apply remote response").clicked()
                        && let Err(err) = self.apply_remote_description(&self.remote_sdp.clone())
                    {
                        eprintln!("Error applying remote description: {:?}", err);
                    }
                });
            });
            ui.separator();
            let ice_starter = ui.add(Button::new("Start ice"));
            if ice_starter.clicked() {
                if self.ice_started {
                    self.status_message = Some("ICE ya está iniciado".to_string());
                } else {
                    match self.start_ice() {
                        Ok(_) => {
                            self.ice_started = true;
                            self.status_message =
                                Some("ICE iniciado, esperando conexión...".to_string());
                        }
                        Err(e) => {
                            eprintln!("ICE ERROR {}", e);
                            self.status_message = Some(format!("Error iniciando ICE: {}", e));
                        }
                    }
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

            ui.separator();
            let go_meet = ui.add(Button::new("Go to meet"));
            if go_meet.clicked() {
                if self.client.is_none() {
                    self.status_message = Some(
                        "Inicializa el peer y comparte la oferta antes de entrar.".to_string(),
                    );
                } else {
                    next_action = Some(CreateMeetAction::GoToVideo);
                }
            }
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
