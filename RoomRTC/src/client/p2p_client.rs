use opencv::core::Mat;
use room_rtc::protocols::rtcp::rtcp_packet::RtcpPacket;
use room_rtc::protocols::rtcp::rtcp_payload::RtcpPayload;
use room_rtc::protocols::rtp::rtp_header::RtpHeader;
use room_rtc::rtc::rtc_peer_connection::{
    PeerConnectionError, PeerConnectionRole, RtcPeerConnection,
};
use room_rtc::worker_thread::error::worker_error::WorkerError;
use room_rtc::worker_thread::media_metrics::{CallMetricsSnapshot, MediaMetrics};
use room_rtc::worker_thread::worker_audio::{WorkerAudio, WorkerAudioError};
use room_rtc::worker_thread::worker_media::{VideoParams, WorkerMedia};
use std::net::SocketAddr;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread::{self, JoinHandle};

pub struct P2PClient {
    // Usamos Arc<Mutex<>> para poder compartirlo de forma segura entre hilos
    peer_connection: Arc<Mutex<RtcPeerConnection>>,
    listener_handle: Option<JoinHandle<()>>,
    media_worker: Option<WorkerMedia>,
    audio_worker: Option<WorkerAudio>,
    media_incoming: Arc<Mutex<Option<SyncSender<Vec<u8>>>>>,
    audio_incoming: Arc<Mutex<Option<SyncSender<Vec<u8>>>>>,
    media_metrics: Option<Arc<Mutex<MediaMetrics>>>,
}

impl P2PClient {
    pub fn new(role: PeerConnectionRole) -> Result<Self, PeerConnectionError> {
        let peer_connection = Arc::new(Mutex::new(RtcPeerConnection::new(None, role)?));

        Ok(Self {
            peer_connection,
            listener_handle: None,
            media_worker: None,
            audio_worker: None,
            media_incoming: Arc::new(Mutex::new(None)),
            audio_incoming: Arc::new(Mutex::new(None)),
            media_metrics: None,
        })
    }

    pub fn role(&self) -> PeerConnectionRole {
        self.peer_connection.lock().unwrap().role()
    }

    pub fn local_addr(&self) -> Result<SocketAddr, PeerConnectionError> {
        self.peer_connection.lock().unwrap().local_addr()
    }

    pub fn create_offer(&mut self) -> Result<String, PeerConnectionError> {
        self.peer_connection.lock().unwrap().create_offer()
    }

    pub fn process_offer(&mut self, offer_sdp: &str) -> Result<String, PeerConnectionError> {
        let answer = self.peer_connection.lock().unwrap().process_offer(offer_sdp)?;
        Ok(answer)
    }

    pub fn set_remote_description(&mut self, remote_sdp: &str) -> Result<(), PeerConnectionError> {
        self.peer_connection
            .lock()
            .unwrap()
            .set_remote_description(remote_sdp)
    }

    /// Inicia el proceso de conexión ICE y DTLS en un hilo de fondo.
    pub fn establish_connection(&mut self) -> Result<(), PeerConnectionError> {
        let pc_clone = Arc::clone(&self.peer_connection);

        // Asegurarse de que el listener esté iniciado antes de empezar
        pc_clone.lock().unwrap().ensure_listener_started()?;

        thread::spawn(move || {
            println!("Connection Thread: Starting...");

            // 1. Iniciar comprobaciones de conectividad ICE
            if let Err(e) = pc_clone.lock().unwrap().start_connectivity_checks() {
                eprintln!("Connection Thread: ICE connectivity checks failed to start: {}", e);
                return;
            }
            println!("Connection Thread: ICE checks started.");

            // 2. Esperar a que ICE se conecte
            for _ in 0..50 { // Timeout de 5 segundos
                if pc_clone.lock().unwrap().is_connected() {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }

            if !pc_clone.lock().unwrap().is_connected() {
                eprintln!("Connection Thread: ICE connection timed out.");
                return;
            }
            println!("Connection Thread: ICE connection established!");

            // 3. Iniciar el handshake DTLS
            match pc_clone.lock().unwrap().start_dtls_handshake(5000) {
                Ok(_) => {
                    println!("Connection Thread: DTLS handshake successful!");
                }
                Err(e) => {
                    eprintln!("Connection Thread: DTLS handshake failed: {}", e);
                }
            }
        });

        Ok(())
    }

    pub fn has_connection(&self) -> bool {
        // Ahora comprobamos tanto ICE como DTLS
        let pc = self.peer_connection.lock().unwrap();
        pc.is_connected() && pc.is_dtls_connected()
    }

    pub fn is_dtls_connected(&self) -> bool {
        self.peer_connection.lock().unwrap().is_dtls_connected()
    }

    pub fn start_media(
        &mut self,
        camera_index: i32,
        video: VideoParams,
    ) -> Result<(), WorkerError> {
        if self.media_worker.is_some() {
            return Ok(());
        }

        println!("DEBUG: start_media acquiring locks...");
        let socket = self.peer_connection.lock().unwrap().media_socket();
        let context = self.peer_connection.lock().unwrap().srtp_context();
        println!("DEBUG: Locks acquired. Starting WorkerMedia...");
        let worker = WorkerMedia::start(camera_index, socket, video, context)?;
        let metrics_handle = worker.metrics();
        let incoming = worker.incoming_sender();
        {
            if let Ok(mut guard) = self.media_incoming.lock() {
                *guard = Some(incoming);
            } else {
                return Err(WorkerError::SendError);
            }
        }
        self.media_worker = Some(worker);
        self.media_metrics = Some(metrics_handle);
        Ok(())
    }

    /// Starts audio capture and playback.
    pub fn start_audio(&mut self) -> Result<(), WorkerAudioError> {
        if self.audio_worker.is_some() {
            return Ok(());
        }

        let socket = self.peer_connection.lock().unwrap().media_socket();
        let context = self.peer_connection.lock().unwrap().srtp_context();
        let audio = WorkerAudio::start(socket, context)?;
        let incoming = audio.incoming_sender();
        {
            if let Ok(mut guard) = self.audio_incoming.lock() {
                *guard = Some(incoming);
            }
        }
        self.audio_worker = Some(audio);
        Ok(())
    }

    pub fn stop_media(&mut self) {
        self.media_worker.take();
        self.audio_worker.take();
        if let Ok(mut guard) = self.media_incoming.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.audio_incoming.lock() {
            *guard = None;
        }
        self.media_metrics = None;
    }

    /// Toggles mute state and returns the new state (true = muted).
    pub fn toggle_mute(&self) -> bool {
        self.audio_worker
            .as_ref()
            .map(|w| w.toggle_mute())
            .unwrap_or(false)
    }

    /// Returns whether the microphone is currently muted.
    pub fn is_muted(&self) -> bool {
        self.audio_worker
            .as_ref()
            .map(|w| w.is_muted())
            .unwrap_or(false)
    }

    pub fn try_recv_local_frame(&self) -> Option<Mat> {
        self.media_worker
            .as_ref()
            .and_then(|worker| worker.get_preview_receiver().try_recv().ok())
    }

    pub fn try_recv_remote_frame(&self) -> Option<Mat> {
        self.media_worker
            .as_ref()
            .and_then(|worker| worker.get_decoded_receiver().try_recv().ok())
    }
    // For messages
    pub fn start_listener(
        &mut self,
        on_msg: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<(), PeerConnectionError> {
        if self.listener_handle.is_some() {
            return Ok(());
        }

        let receiver = self.peer_connection.lock().unwrap().take_receiver()?;
        let callback = Arc::new(on_msg);
        let thread_callback = Arc::clone(&callback);
        let media_input = Arc::clone(&self.media_incoming);

        let srtp_context = self.peer_connection.lock().unwrap().srtp_context();

        let pc_for_addr_update = Arc::clone(&self.peer_connection);
        let mut last_packet_time = std::time::Instant::now();
        let mut packet_count: u64 = 0;

        let handle = thread::spawn(move || {
            while let Ok((data, src_addr)) = receiver.recv() {
                packet_count += 1;
                let now = std::time::Instant::now();
                let gap = now.duration_since(last_packet_time).as_millis();
                
                // Log if there was a gap > 1 second (possible reconnection)
                if gap > 1000 {
                    println!("DEBUG: Packet received after {}ms gap from {} (total: {})", gap, src_addr, packet_count);
                }
                last_packet_time = now;

                // Update remote address if it changed (NAT rebind after reconnection)
                if let Ok(mut pc) = pc_for_addr_update.lock() {
                    pc.update_remote_addr(src_addr);
                }

                // Intentamos descifrar el paquete. Si falla, lo tratamos como texto.
                let mut decrypted_data = data.clone();
                if let Some(ctx) = &srtp_context {
                    // Verificamos longitud mínima segura para leer el header (12 bytes + CSRC list)
                    let min_len = if data.len() >= 1 { 12 + ((data[0] & 0x0F) as usize * 4) } else { 12 };
                    
                    if data.len() >= min_len {
                        let (header, header_size) = RtpHeader::read_bytes(&data);
                        let encrypted_payload = &data[header_size..];
                        if let Some(unprotected) = ctx.unprotect(header.get_sequence_number(), header.get_timestamp(), encrypted_payload) {
                            let mut new_bytes = Vec::with_capacity(header_size + unprotected.len());
                            new_bytes.extend_from_slice(&data[..header_size]);
                            new_bytes.extend_from_slice(&unprotected);
                            decrypted_data = new_bytes;
                        }
                    }
                }

                // Ahora procesamos el paquete (ya sea descifrado o el original)
                match String::from_utf8(decrypted_data.clone()) {
                    Ok(message) => thread_callback(message),
                    Err(_err) => {
                        //If it's not UTF8, it's a media packet (RTP/RTCP)
                        
                        let bytes = decrypted_data;

                        let is_rtcp_bye = bytes.len() >= 4
                            && RtcpPacket::read_bytes(&bytes)
                                .is_ok_and(|packet| matches!(packet.payload, RtcpPayload::Bye(_)));

                        if is_rtcp_bye {
                            thread_callback("CALL_END".to_string());
                        }
                        if let Ok(lock) = media_input.lock()
                            && let Some(tx) = lock.as_ref()
                        {
                            let _ = tx.send(bytes);
                        }
                    }
                }
            }
        });

        self.listener_handle = Some(handle);
        Ok(())
    }

    pub fn send_msg(&self, msg: &str) -> Result<(), PeerConnectionError> {
        self.peer_connection.lock().unwrap().send(msg.as_bytes())
    }

    pub fn send_rtcp_bye(&self) -> Result<(), WorkerError> {
        self.media_worker
            .as_ref()
            .ok_or(WorkerError::SendError)?
            .send_rtcp_bye()
    }

    pub fn metrics_snapshot(&self) -> Option<CallMetricsSnapshot> {
        self.media_metrics
            .as_ref()
            .and_then(|metrics| metrics.lock().ok().map(|m| m.snapshot()))
    }
}
