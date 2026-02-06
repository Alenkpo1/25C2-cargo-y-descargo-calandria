use opencv::core::Mat;
use room_rtc::protocols::rtcp::rtcp_packet::RtcpPacket;
use room_rtc::protocols::rtcp::rtcp_payload::RtcpPayload;
use room_rtc::protocols::rtp::rtp_header::RtpHeader;
use room_rtc::rtc::rtc_peer_connection::{
    PeerConnectionError, PeerConnectionRole, RtcPeerConnection,
};
use room_rtc::worker_thread::error::worker_error::WorkerError;
use room_rtc::worker_thread::media_metrics::{CallMetricsSnapshot, MediaMetrics};
use room_rtc::worker_thread::worker_media::{VideoParams, WorkerMedia};
use room_rtc::crypto::srtp::SrtpContext;
use room_rtc::rtc::socket::peer_socket::PeerSocket;
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
    media_incoming: Arc<Mutex<Option<SyncSender<Vec<u8>>>>>,
    audio_incoming: Arc<Mutex<Option<SyncSender<Vec<u8>>>>>,
    media_metrics: Option<Arc<Mutex<MediaMetrics>>>,
    pub sctp_incoming: Arc<Mutex<Option<SyncSender<(u16, Vec<u8>)>>>>,
}

impl Clone for P2PClient {
    fn clone(&self) -> Self {
        Self {
            peer_connection: Arc::clone(&self.peer_connection),
            listener_handle: None,
            media_worker: None,
            media_incoming: Arc::clone(&self.media_incoming),
            audio_incoming: Arc::clone(&self.audio_incoming),
            media_metrics: self.media_metrics.clone(),
            sctp_incoming: Arc::clone(&self.sctp_incoming),
        }
    }
}

impl P2PClient {
    fn write_dtls_with_retry(pc: &mut RtcPeerConnection, data: &[u8]) {
        let mut backoff_ms = 2u64;
        loop {
            match pc.dtls_write(data) {
                Ok(_) => return,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                    backoff_ms = (backoff_ms * 2).min(200); // cap backoff
                    continue;
                }
                Err(_) => return, // drop on other errors (e.g., connection closed)
            }
        }
    }
    pub fn new(role: PeerConnectionRole) -> Result<Self, PeerConnectionError> {
        let peer_connection = Arc::new(Mutex::new(RtcPeerConnection::new(None, role)?));

        Ok(Self {
            peer_connection,
            listener_handle: None,
            media_worker: None,
            media_incoming: Arc::new(Mutex::new(None)),
            audio_incoming: Arc::new(Mutex::new(None)),
            media_metrics: None,
            sctp_incoming: Arc::new(Mutex::new(None)),
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
        let sctp_extension = Arc::clone(&self.sctp_incoming);

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
                    return;
                }
            }

            // 4. Iniciar SCTP Association
            {
               let mut pc = pc_clone.lock().unwrap();
               // Determine if client (Controlling -> Client).
               let callback_role = pc.role(); 
               if let Some(sctp) = &mut pc.sctp_association {
                   // Controlling node initiates Connect
                   // Both sides call establish; initiator will send INIT.
                   if matches!(callback_role, PeerConnectionRole::Controlling) {
                        sctp.establish();
                   } else {
                        sctp.establish();
                   }
               }
            }

            // 5. Start SCTP Pump Loop
            println!("Connection Thread: Entering SCTP Pump Loop...");
            loop {
                thread::sleep(Duration::from_millis(1));
                
                let mut pc = pc_clone.lock().unwrap();
                if pc.sctp_association.is_none() || !pc.has_dtls_session() {
                    break;
                }

                // A. Read from DTLS -> Feed SCTP
                let mut buf = [0u8; 2048];
                match pc.dtls_read(&mut buf) {
                     Ok(n) => {
                         let data = &buf[..n];
                         pc.sctp_association.as_mut().unwrap().handle_input(data);
                     }
                     Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                         // No data, continue
                     }
                     Err(_) => {
                         // DTLS error
                     }
                }

                // B. Poll SCTP Output -> Write DTLS
                if pc.sctp_association.is_some() {
                    let mut outbound: Vec<Vec<u8>> = Vec::new();
                    let mut incoming: Vec<(u16, Vec<u8>)> = Vec::new();
                    {
                        let sctp = pc.sctp_association.as_mut().unwrap();
                        while let Some(out_packet) = sctp.poll_output() {
                            outbound.push(out_packet);
                        }
                        while let Some(pkt) = sctp.recv_data() {
                            incoming.push(pkt);
                        }
                    }

                    for out_packet in outbound {
                        Self::write_dtls_with_retry(&mut pc, &out_packet);
                    }

                    for (stream, payload) in incoming {
                        if let Ok(guard) = sctp_extension.lock() {
                            if let Some(tx) = guard.as_ref() {
                                let _ = tx.send((stream, payload));
                            }
                        }
                    }
                }
            }
            println!("Connection Thread: SCTP Pump Loop exited.");
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

    /// Returns the socket and SRTP context for audio (to be started in UI thread).
    pub fn audio_params(&self) -> (Arc<Mutex<PeerSocket>>, Option<SrtpContext>) {
        let socket = self.peer_connection.lock().unwrap().media_socket();
        let context = self.peer_connection.lock().unwrap().srtp_context();
        (socket, context)
    }

    /// Sets the audio incoming sender (called from VideoCall after WorkerAudio is created).
    pub fn set_audio_incoming(&self, sender: SyncSender<Vec<u8>>) {
        if let Ok(mut guard) = self.audio_incoming.lock() {
            *guard = Some(sender);
        }
    }

    pub fn stop_media(&mut self) {
        self.media_worker.take();
        if let Ok(mut guard) = self.media_incoming.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.audio_incoming.lock() {
            *guard = None;
        }
        self.media_metrics = None;
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
        let audio_input = Arc::clone(&self.audio_incoming);

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
                        
                        // Route RTP packets by SSRC: 1000 = video, 2000 = audio
                        if bytes.len() >= 12 {
                            let (header, _) = RtpHeader::read_bytes(&bytes);
                            let ssrc = header.get_ssrc();
                            
                            if ssrc == 2000 {
                                // Audio packet
                                if let Ok(lock) = audio_input.lock() {
                                    if let Some(tx) = lock.as_ref() {
                                        let _ = tx.send(bytes);
                                    }
                                }
                            } else {
                                // Video packet (or default)
                                if let Ok(lock) = media_input.lock()
                                    && let Some(tx) = lock.as_ref()
                                {
                                    let _ = tx.send(bytes);
                                }
                            }
                        } else {
                            // Fallback: short packets go to video
                            if let Ok(lock) = media_input.lock()
                                && let Some(tx) = lock.as_ref()
                            {
                                let _ = tx.send(bytes);
                            }
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
    
    pub fn send_sctp_data(&self, stream: u16, payload: Vec<u8>) -> Result<(), String> {
        let mut pc = self.peer_connection.lock().unwrap();
        if let Some(sctp) = &mut pc.sctp_association {
            sctp.send_data(stream, payload)?;
            
            // Trigger write immediately if possible
            let mut outbound: Vec<Vec<u8>> = Vec::new();
            while let Some(out) = sctp.poll_output() {
                outbound.push(out);
            }
            for out in outbound {
                Self::write_dtls_with_retry(&mut pc, &out);
            }
            Ok(())
        } else {
            Err("SCTP not initialized".to_string())
        }
    }
    
    pub fn set_sctp_incoming(&self, sender: SyncSender<(u16, Vec<u8>)>) {
          if let Ok(mut guard) = self.sctp_incoming.lock() {
               *guard = Some(sender);
          }
    }
}
