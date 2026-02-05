use crate::client::p2p_client::P2PClient;
use eframe::egui::load::SizedTexture;
use eframe::egui::{
    self, Align2, Button, Color32, ColorImage, FontId, TextureHandle, TextureOptions, Vec2, RichText
};
use opencv::core::Mat;
use opencv::prelude::*;
use room_rtc::worker_thread::media_metrics::CallMetricsSnapshot;
use room_rtc::worker_thread::worker_audio::WorkerAudio;
use room_rtc::worker_thread::worker_media::VideoParams;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;

pub enum VideoMeetAction {
    GoToLobby,
}
pub struct VideoCall {
    client: Option<P2PClient>,
    local_texture: Option<TextureHandle>,
    remote_texture: Option<TextureHandle>,
    media_started: bool,
    status_message: Option<String>,
    message_inbox: Option<Arc<Mutex<Vec<String>>>>,
    processed_messages: usize,
    quality_metrics: Option<CallMetricsSnapshot>,
    peer_username: Option<String>,
    video: VideoParams,
    media_loader: Option<Receiver<Result<P2PClient, (P2PClient, String)>>>,
    unstable: bool,
    last_remote_seen: Option<std::time::Instant>,
    audio_started: bool,
    audio_worker: Option<WorkerAudio>,
    show_stats: bool,
}

impl VideoCall {
    pub fn new(video: VideoParams) -> Self {
        Self {
            client: None,
            local_texture: None,
            remote_texture: None,
            media_started: false,
            status_message: None,
            message_inbox: None,
            processed_messages: 0,
            quality_metrics: None,
            peer_username: None,
            video,
            media_loader: None,
            unstable: false,
            last_remote_seen: None,
            audio_started: false,
            audio_worker: None,
            show_stats: false,
        }
    }

    pub fn set_client(
        &mut self,
        client: P2PClient,
        inbox: Arc<Mutex<Vec<String>>>,
        peer_username: Option<String>,
    ) {
        self.client = Some(client);
        self.local_texture = None;
        self.remote_texture = None;
        self.media_started = false;
        self.status_message = None;
        self.processed_messages = {
            if let Ok(guard) = inbox.lock() {
                guard.len()
            } else {
                0
            }
        };
        self.message_inbox = Some(Arc::clone(&inbox));
        self.peer_username = peer_username.clone();
        self.media_loader = None;
        self.unstable = false;
        self.last_remote_seen = Some(std::time::Instant::now());
    }

    pub fn reset(&mut self) {
        self.stop_current_call();
        self.client = None;
        self.local_texture = None;
        self.remote_texture = None;
        self.media_started = false;
        self.audio_started = false;
        self.audio_worker = None;
        self.status_message = None;
        self.message_inbox = None;
        self.processed_messages = 0;
        self.quality_metrics = None;
        self.peer_username = None;
        self.media_loader = None;
        self.unstable = false;
        self.last_remote_seen = None;
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) -> Option<VideoMeetAction> {
        let mut next_action = None;

        let remote_hangup = self.consume_remote_messages();
        if !self.media_started {
            self.quality_metrics = None;
            self.unstable = false;
            self.last_remote_seen = None;
        }

        if remote_hangup {
            self.stop_current_call();
            next_action = Some(VideoMeetAction::GoToLobby);
        } else {
            //Checks if there is a media loader in progress
            if let Some(loader) = &self.media_loader {
                if let Ok(result) = loader.try_recv() {
                    self.media_loader = None;
                    match result {
                        Ok(client_ready) => {
                            self.client = Some(client_ready);
                            self.media_started = true;
                            self.status_message = None;
                        }
                        Err((client_failed, err)) => {
                            self.client = Some(client_failed);
                            self.status_message = Some(format!("Error starting camera: {}", err));
                        }
                    }
                }
            }
            // Start media if we have a client and haven't started yet
            else if let Some(mut client) = self.client.take() {
                if client.has_connection() && !self.media_started {
                    self.status_message = Some("Starting Camera".to_string());
                    let (tx, rx) = std::sync::mpsc::channel();
                    let video_params = self.video;
                    thread::spawn(move || {
                        let res = match client.start_media(0, video_params) {
                            Ok(_) => Ok(client),
                            Err(e) => Err((client, e.to_string())),
                        };
                        let _ = tx.send(res);
                    });
                    self.media_loader = Some(rx);
                } else {
                    self.client = Some(client);
                }
            }

            //Update textures if media has started
            if self.media_started {
                // Start audio once media is ready (must be in main thread due to cpal)
                if !self.audio_started {
                    if let Some(client) = self.client.as_ref() {
                        let (socket, context) = client.audio_params();
                        match WorkerAudio::start(socket, context) {
                            Ok(worker) => {
                                // Connect audio incoming sender to client listener
                                let sender = worker.incoming_sender();
                                client.set_audio_incoming(sender);
                                
                                self.audio_worker = Some(worker);
                                self.audio_started = true;
                            }
                            Err(e) => {
                                eprintln!("Failed to start audio: {}", e);
                                self.audio_started = true; // Don't retry
                            }
                        }
                    }
                }
                
                if let Some(client) = self.client.as_ref() {
                    self.quality_metrics = client.metrics_snapshot();
                    if let Some(frame) = client.try_recv_local_frame()
                        && let Some(image) = Self::mat_to_color_image(&frame)
                    {
                        Self::update_texture(
                            ctx,
                            &mut self.local_texture,
                            "roomrtc-local-preview",
                            image,
                        );
                    }

                    if let Some(frame) = client.try_recv_remote_frame()
                        && let Some(image) = Self::mat_to_color_image(&frame)
                    {
                        self.last_remote_seen = Some(std::time::Instant::now());
                        Self::update_texture(
                            ctx,
                            &mut self.remote_texture,
                            "roomrtc-remote-preview",
                            image,
                        );
                    }

                    ctx.request_repaint();

                    // Heartbeat remoto: si hay actividad reciente, refrescamos el último visto
                    if let Some(metrics) = &self.quality_metrics {
                        if let Some(ms) = metrics.since_last_ms {
                            if ms < 2_000 {
                                self.last_remote_seen = Some(std::time::Instant::now());
                            }
                        }
                    }
                    // Evaluar inactividad remota con umbral más amplio
                    if let Some(last_seen) = self.last_remote_seen {
                        let gap = last_seen.elapsed().as_millis() as u64;
                        self.unstable = gap > 2_000 && gap <= 30_000;
                        if gap > 30_000 {
                            self.status_message =
                                Some("Conexión perdida, finalizando llamada".to_string());
                            Self::send_hangup_signal(client);
                            self.stop_current_call();
                            next_action = Some(VideoMeetAction::GoToLobby);
                        }
                    } else {
                        self.unstable = false;
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Stats Overlay
            if self.show_stats {
                egui::Window::new("stats_overlay")
                    .anchor(Align2::LEFT_TOP, egui::vec2(20.0, 80.0))
                    .title_bar(false)
                    .resizable(false)
                    .frame(egui::Frame::none().fill(Color32::from_black_alpha(180)).rounding(8.0).inner_margin(12.0))
                    .show(ctx, |ui| {
                         ui.label(RichText::new("🔌 Network Statistics").strong().color(Color32::WHITE));
                         ui.add_space(4.0);
                         
                         if let Some(metrics) = &self.quality_metrics {
                             let text_color = crate::ui::theme::colors::TEXT_PRIMARY;
                             ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                             
                             egui::Grid::new("stats_grid").num_columns(2).spacing(egui::vec2(20.0, 4.0)).show(ui, |ui| {
                                 ui.label(RichText::new("Bitrate:").color(crate::ui::theme::colors::TEXT_MUTED));
                                 ui.label(RichText::new(format!("{:.0} kbps", metrics.bitrate_kbps)).color(text_color));
                                 ui.end_row();
                                 
                                 ui.label(RichText::new("Packet Loss:").color(crate::ui::theme::colors::TEXT_MUTED));
                                 let loss_color = if metrics.packet_loss_pct > 5.0 { crate::ui::theme::colors::DANGER } else { crate::ui::theme::colors::SUCCESS };
                                 ui.label(RichText::new(format!("{:.2}%", metrics.packet_loss_pct)).color(loss_color));
                                 ui.end_row();
                                 
                                 ui.label(RichText::new("Jitter:").color(crate::ui::theme::colors::TEXT_MUTED));
                                 ui.label(RichText::new(format!("{:.1} ms", metrics.jitter_ms)).color(text_color));
                                 ui.end_row();
                                 
                                 ui.label(RichText::new("RTT (est):").color(crate::ui::theme::colors::TEXT_MUTED));
                                 ui.label(RichText::new(format!("{} ms", metrics.since_last_ms.unwrap_or(0))).color(text_color));
                                 ui.end_row();
                             });
                         } else {
                             ui.label(RichText::new("Gathering metrics...").italics().color(crate::ui::theme::colors::TEXT_MUTED));
                         }
                    });
            }

            // Header (Status overlay)
            if let Some(status) = &self.status_message {
                ui.colored_label(crate::ui::theme::colors::DANGER, status);
            }
            if self.unstable {
                ui.colored_label(crate::ui::theme::colors::DANGER, "⚠ Network Unstable");
            }

            // Main Video Area (Remote)
            let available_rect = ui.available_rect_before_wrap();
            let control_bar_height = 80.0;
            let video_area_height = available_rect.height() - control_bar_height;
            
            // Allocate space for videos
            ui.allocate_ui_at_rect(egui::Rect::from_min_size(available_rect.min, egui::vec2(available_rect.width(), video_area_height)), |ui| {
                ui.centered_and_justified(|ui| {
                    if self.client.is_some() && self.media_started {
                        // Remote Video (Primary)
                        Self::draw_video_slot(ui, self.remote_texture.as_ref(), "Waiting for participant...", ui.available_size());
                    } else {
                        ui.label(RichText::new("Connecting...").size(24.0).color(crate::ui::theme::colors::TEXT_MUTED));
                    }
                });
            });

            // Local Video (PiP - Bottom Right)
            // We use a fixed relative rect for PiP
            let pip_width = 200.0;
            let pip_height = pip_width * 9.0 / 16.0;
            let pip_rect = egui::Rect::from_min_size(
                egui::pos2(
                    available_rect.max.x - pip_width - 20.0,
                    available_rect.min.y + video_area_height - pip_height - 20.0
                ),
                egui::vec2(pip_width, pip_height)
            );
            
            // Draw PiP frame
            ui.put(pip_rect, |ui: &mut egui::Ui| {
                egui::Frame::none()
                    .stroke(egui::Stroke::new(2.0, crate::ui::theme::colors::BACKGROUND_TERTIARY))
                    .shadow(egui::Shadow::default())
                    .show(ui, |ui| {
                         Self::draw_video_slot(ui, self.local_texture.as_ref(), "No Cam", pip_rect.size());
                    }).response
            });


            // Floating Control Bar (Bottom)
            egui::Area::new("control_bar".into())
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(crate::ui::theme::colors::BACKGROUND_TERTIARY)
                        .rounding(32.0)
                        .shadow(egui::Shadow::default())
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                
                                // Mute Button
                                let is_muted = self.audio_worker.as_ref().map(|w| w.is_muted()).unwrap_or(false);
                                let (mute_icon, _mute_color) = if is_muted { 
                                    ("🔇", crate::ui::theme::colors::DANGER) 
                                } else { 
                                    ("🎤", crate::ui::theme::colors::TEXT_PRIMARY) 
                                };
                                
                                let mute_btn = Button::new(RichText::new(mute_icon).size(24.0))
                                    .fill(if is_muted { crate::ui::theme::colors::BACKGROUND_SECONDARY } else { crate::ui::theme::colors::BACKGROUND })
                                    .frame(true)
                                    .rounding(30.0)
                                    .min_size(Vec2::new(50.0, 50.0));
                                    
                                if ui.add(mute_btn).on_hover_text("Toggle Mute").clicked() {
                                    if let Some(audio) = &self.audio_worker {
                                        audio.toggle_mute();
                                    }
                                }
                                
                                ui.add_space(20.0);
                                
                                // Video Toggle (Placeholder)
                                let video_btn = Button::new(RichText::new("📷").size(24.0))
                                    .fill(crate::ui::theme::colors::BACKGROUND)
                                    .rounding(30.0)
                                    .min_size(Vec2::new(50.0, 50.0));
                                ui.add(video_btn).on_hover_text("Toggle Video");
                                
                                ui.add_space(20.0);

                                // Stats Toggle Button
                                let stats_icon = "📊";
                                let stats_btn = Button::new(RichText::new(stats_icon).size(24.0))
                                    .fill(if self.show_stats { crate::ui::theme::colors::PRIMARY } else { crate::ui::theme::colors::BACKGROUND })
                                    .rounding(30.0)
                                    .min_size(Vec2::new(50.0, 50.0));
                                if ui.add(stats_btn).on_hover_text("Toggle Statistics").clicked() {
                                    self.show_stats = !self.show_stats;
                                }

                                ui.add_space(20.0);

                                // Hangup Button
                                let hangup_btn = Button::new(RichText::new("📞").size(24.0).color(egui::Color32::WHITE))
                                    .fill(crate::ui::theme::colors::DANGER)
                                    .rounding(30.0)
                                    .min_size(Vec2::new(60.0, 50.0));
                                    
                                if ui.add(hangup_btn).on_hover_text("End Call").clicked() {
                                    if let Some(client) = self.client.as_mut() {
                                        Self::send_hangup_signal(client);
                                    }
                                    self.stop_current_call();
                                    self.status_message = Some("Call Ended".to_string());
                                    next_action = Some(VideoMeetAction::GoToLobby);
                                }
                                
                                ui.add_space(10.0);
                            });
                        });
                });
        });

        next_action
    }

    fn update_texture(
        ctx: &egui::Context,
        handle: &mut Option<TextureHandle>,
        name: &str,
        image: ColorImage,
    ) {
        match handle {
            Some(texture) => texture.set(image, TextureOptions::LINEAR),
            None => {
                *handle = Some(ctx.load_texture(name.to_string(), image, TextureOptions::LINEAR));
            }
        }
    }

    fn draw_video_slot(
        ui: &mut egui::Ui,
        texture: Option<&TextureHandle>,
        placeholder: &str,
        target_size: Vec2,
    ) {
        let video_size = target_size;

        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                if let Some(texture) = texture {
                    let tex_size = texture.size_vec2();
                    let aspect = if tex_size.y > 0.0 {
                        tex_size.x / tex_size.y
                    } else {
                        video_size.x / video_size.y
                    };

                    let mut size = video_size;
                    if aspect > 0.0 {
                        size.y = size.x / aspect;
                    }

                    let sized = SizedTexture::new(texture.id(), size);
                    let image = egui::Image::from_texture(sized).fit_to_exact_size(size);
                    ui.add(image);
                } else {
                    let (rect, _) = ui.allocate_exact_size(video_size, egui::Sense::hover());
                    ui.painter().rect_filled(rect, 8.0, Color32::from_gray(40));
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        placeholder,
                        FontId::proportional(16.0),
                        Color32::from_gray(210),
                    );
                }
            });
        });
    }

    fn mat_to_color_image(mat: &Mat) -> Option<ColorImage> {
        let width = mat.cols();
        let height = mat.rows();

        if width <= 0 || height <= 0 {
            return None;
        }

        let width = width as usize;
        let height = height as usize;
        let channels = mat.channels() as usize;
        if channels < 3 {
            return None;
        }

        let step = mat.step1(0).ok()?;
        let data = mat.data_bytes().ok()?;

        let mut rgba = vec![0u8; width * height * 4];
        for y in 0..height {
            let row_start = y * step;
            for x in 0..width {
                let src_index = row_start + x * channels;
                let dst_index = (y * width + x) * 4;

                let b = *data.get(src_index)?;
                let g = *data.get(src_index + 1)?;
                let r = *data.get(src_index + 2)?;

                rgba[dst_index] = r;
                rgba[dst_index + 1] = g;
                rgba[dst_index + 2] = b;
                rgba[dst_index + 3] = 255;
            }
        }

        Some(ColorImage::from_rgba_unmultiplied([width, height], &rgba))
    }

    fn consume_remote_messages(&mut self) -> bool {
        if let Some(inbox) = &self.message_inbox
            && let Ok(messages) = inbox.lock()
        {
            let total = messages.len();
            if self.processed_messages < total {
                for msg in messages.iter().skip(self.processed_messages) {
                    if msg.trim() == "CALL_END" {
                        self.status_message =
                            Some("El otro participante colgó la llamada.".to_string());
                        self.processed_messages = total;
                        return true;
                    }
                }
                self.processed_messages = total;
            }
        }

        false
    }

    fn stop_current_call(&mut self) {
        if let Some(client) = self.client.as_mut() {
            client.stop_media();
        }
        self.media_started = false;
        self.local_texture = None;
        self.remote_texture = None;
    }

    fn send_hangup_signal(client: &P2PClient) {
        if let Err(err) = client.send_rtcp_bye() {
            eprintln!("Error enviando RTCP BYE: {:?}", err);
            if let Err(msg_err) = client.send_msg("CALL_END") {
                eprintln!("Error enviando fin de llamada: {:?}", msg_err);
            }
        }
    }

    pub fn peer(&self) -> Option<String> {
        self.peer_username.clone()
    }

    pub fn handle_call_ended(&mut self, from: String) {
        if self.peer_username.as_deref() == Some(&from) {
            self.status_message = Some(format!("{} finalizó la llamada.", from));
            self.stop_current_call();
            self.peer_username = None;
        }
    }
}
