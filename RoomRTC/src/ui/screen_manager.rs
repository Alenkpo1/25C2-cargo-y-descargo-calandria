use crate::client::signaling_client::{SignalingClient, SignalingEvent};
use crate::config::AppConfig;
use crate::logger::Logger;
use crate::ui::screens::join_meet::JoinMeetAction;
use crate::ui::screens::join_meet::JoinMeetScreen;
use crate::ui::screens::lobby::LobbyAction;
use crate::ui::screens::lobby::LobbyScreen;
use crate::ui::screens::login::{LoginAction, LoginScreen};
use crate::ui::screens::video::VideoCall;
use crate::ui::screens::video::VideoMeetAction;
use crate::ui::screens::waiting_call::WaitingCall;
use crate::ui::screens::waiting_call::WaitingCallAction;
use std::time::Duration;
use eframe::egui;
use room_rtc::rtc::rtc_peer_connection::PeerConnectionRole;
use room_rtc::worker_thread::worker_media::VideoParams;
pub enum Screen {
    Login,
    Lobby,
    JoinMeet,
    WaitingCall,
    VideoCall,
}

pub struct MainApp {
    current_screen: Screen,
    lobby: LobbyScreen,
    join_meet: JoinMeetScreen,
    waiting_call: WaitingCall,
    video_meet: VideoCall,
    login: LoginScreen,
    signaling: Option<SignalingClient>,
    username: Option<String>,
    active_peer: Option<String>,
    logger: Logger,
}

impl MainApp {
    pub fn new(config: AppConfig) -> Self {
        let logger = Logger::start(&config.log_file).unwrap_or_else(|err| {
            eprintln!(
                "No se pudo abrir log {} ({}), usando /tmp/roomrtc-client.log",
                config.log_file, err
            );
            Logger::start("/tmp/roomrtc-client.log").unwrap_or_else(|_| Logger::noop())
        });
        Self {
            current_screen: Screen::Login,
            lobby: LobbyScreen::new(),
            join_meet: JoinMeetScreen::new(PeerConnectionRole::Controlled),
            waiting_call: WaitingCall::new(PeerConnectionRole::Controlling),
            video_meet: VideoCall::new(VideoParams {
                width: config.video_width,
                height: config.video_height,
                fps: config.video_fps,
            }),
            login: LoginScreen::new(config.server_addr.clone(), Some(logger.clone())),
            signaling: None,
            username: None,
            active_peer: None,
            logger,
        }
    }

    fn handle_signaling_events(&mut self) {
        while let Some(event) = self
            .signaling
            .as_ref()
            .and_then(|signaling| signaling.try_next_event())
        {
            match event {
                SignalingEvent::UserList(users) => self.lobby.set_users(users),
                SignalingEvent::UserStatusChanged { username, status } => {
                    self.lobby.update_user_status(username, status)
                }
                SignalingEvent::IncomingCall { from, sdp } => {
                    self.active_peer = Some(from.clone());
                    self.join_meet.on_incoming_call(from, sdp);
                    self.current_screen = Screen::JoinMeet;
                    self.logger.info("Llamada entrante recibida");
                }
                SignalingEvent::CallAccepted { from, sdp } => {
                    self.active_peer = Some(from.clone());
                    self.waiting_call.on_call_accepted(from, sdp);
                    if let Some((client, inbox)) = self.waiting_call.take_client_with_inbox() {
                        self.video_meet.set_client(client, inbox, self.waiting_call.active_peer());
                        self.current_screen = Screen::VideoCall;
                    }
                    self.logger.info("Oferta aceptada por el peer remoto");
                }
                SignalingEvent::CallRejected { from } => self.waiting_call.on_call_rejected(from),
                SignalingEvent::CallEnded { from } => {
                    self.waiting_call.on_call_ended(&from);
                    self.join_meet.on_call_ended(&from);
                    self.video_meet.handle_call_ended(from.clone());
                    self.video_meet.reset();
                    self.active_peer = None;
                    self.current_screen = Screen::Lobby;
                    self.logger.info("Llamada finalizada");
                }
                SignalingEvent::Error(err) => {
                    eprintln!("Signaling error: {}", err);
                    self.logger
                        .error(&format!("Error de señalización: {}", err));
                }
                SignalingEvent::Registered(msg) => {
                    self.login.status_message = Some(msg);
                }
                SignalingEvent::RegisterError(err) => {
                    self.login.status_message = Some(err);
                }
                SignalingEvent::LoginError(err) => {
                    self.login.status_message = Some(format!("Login rechazado: {}", err));
                    self.signaling = None;
                    self.current_screen = Screen::Login;
                    break;
                }
                SignalingEvent::Disconnected | SignalingEvent::LoggedOut => {
                    self.login.status_message = Some("Conexión con el servidor cerrada".into());
                    self.signaling = None;
                    self.current_screen = Screen::Login;
                    self.logger
                        .warn("Sesión cerrada o desconectada del servidor de señalización");
                    break;
                }
                SignalingEvent::IceCandidate { from, candidate } => {
                    eprintln!("ICE desde {}: {}", from, candidate);
                }
                SignalingEvent::LoginSuccess(_) => {}
            }
        }
    }
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Repaint frecuente para procesar eventos de señalización aunque no haya input
        ctx.request_repaint_after(Duration::from_millis(30));
        self.handle_signaling_events();
        match self.current_screen {
            Screen::Login => {
                if let Some(LoginAction::LoggedIn {
                    username,
                    signaling,
                }) = self.login.update(ctx)
                {
                    self.username = Some(username);
                    self.signaling = Some(signaling);
                    if let Some(sig) = self.signaling.as_ref() {
                        let _ = sig.request_users();
                    }
                    self.current_screen = Screen::Lobby;
                }
            }
            Screen::Lobby => {
                let signaling = self.signaling.as_ref();
                let username = self.username.as_deref();
                if let Some(action) = self.lobby.update(ctx, signaling, username) {
                    match action {
                        LobbyAction::GoToWaitingCall(username) => {
                            self.current_screen = Screen::WaitingCall;
                            if let Some(signaling) = self.signaling.as_ref()
                                && let Err(e) = self.waiting_call.call_user(&username, signaling)
                            {
                                self.logger.error(&format!("Failed to call: {}", e));
                                self.waiting_call.status_message =
                                    Some(format!("Failed to place call: {}", e));
                            }
                        }
                        LobbyAction::Logout => {
                            self.signaling = None;
                            self.current_screen = Screen::Login;
                            self.logger.info("Usuario cerró sesión desde lobby");
                        }
                    }
                }
            }
            Screen::JoinMeet => {
                let signaling = self.signaling.as_ref();
                if let Some(action) = self.join_meet.update(ctx, frame, signaling) {
                    match action {
                        JoinMeetAction::GoToLobby => {
                            if let (Some(signaling), Some(peer)) =
                                (self.signaling.as_ref(), self.join_meet.active_peer())
                            {
                                let _ = signaling.end_call(&peer);
                            }
                            self.current_screen = Screen::Lobby
                        }
                        JoinMeetAction::GoToVideo => {
                            if let Some((client, inbox)) = self.join_meet.take_client_with_inbox() {
                                self.video_meet.set_client(
                                    client,
                                    inbox,
                                    self.join_meet.active_peer(),
                                );
                            }
                            self.current_screen = Screen::VideoCall;
                        }
                    }
                }
            }
            Screen::WaitingCall => {
                if let Some(action) = self.waiting_call.update(ctx, frame) {
                    match action {
                        WaitingCallAction::GoToLobby => {
                            if let (Some(signaling), Some(peer)) =
                                (self.signaling.as_ref(), self.waiting_call.active_peer())
                            {
                                let _ = signaling.end_call(&peer);
                            }
                            self.current_screen = Screen::Lobby
                        }
                        WaitingCallAction::GoToVideo => {
                            if let Some((client, inbox)) =
                                self.waiting_call.take_client_with_inbox()
                            {
                                self.video_meet.set_client(
                                    client,
                                    inbox,
                                    self.waiting_call.active_peer(),
                                );
                            }
                            self.current_screen = Screen::VideoCall;
                        }
                    }
                }
            }
            Screen::VideoCall => {
                if let Some(action) = self.video_meet.update(ctx, frame) {
                    match action {
                        VideoMeetAction::GoToLobby => {
                            if let (Some(signaling), Some(peer)) =
                                (self.signaling.as_ref(), self.video_meet.peer())
                            {
                                let _ = signaling.end_call(&peer);
                            }
                            self.video_meet.reset();
                            self.current_screen = Screen::Lobby;
                            self.active_peer = None;
                        }
                    }
                }
            }
        }
    }
}
