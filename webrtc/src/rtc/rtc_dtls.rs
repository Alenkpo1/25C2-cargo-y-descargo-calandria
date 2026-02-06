use openssl::asn1::Asn1Time;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::ssl::{Ssl, SslContext, SslMethod, SslStream, SslVerifyMode, HandshakeError};
use openssl::x509::{X509NameBuilder, X509};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver};
use std::cmp;

/// Stream que conecta OpenSSL con el mundo UDP a través de un Channel.
/// - Escritura: Directa al UdpSocket.

/// - Lectura: Desde un mpsc::Receiver (alimentado por el demultiplexor).

#[derive(Debug)]
pub struct UdpStream {
    socket: Arc<Mutex<UdpSocket>>,
    remote_addr: SocketAddr,
    receiver: Receiver<Vec<u8>>,

    // (Ej: llega paquete de 50 bytes, OpenSSL pide leer 10, sobran 40)
    read_buffer: Vec<u8>,
    cursor: usize,
}

impl UdpStream {
    pub fn new(
        socket: Arc<Mutex<UdpSocket>>,
        remote_addr: SocketAddr,
        receiver: Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            socket,
            remote_addr,
            receiver,
            read_buffer: Vec::new(),
            cursor: 0,
        }
    }
}

impl Read for UdpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // 1. Si hay datos sobrantes del paquete anterior, úsalos primero.
        let available = self.read_buffer.len() - self.cursor;
        if available > 0 {
            let n = cmp::min(available, buf.len());
            buf[..n].copy_from_slice(&self.read_buffer[self.cursor..self.cursor + n]);
            self.cursor += n;

            // Si consumimos todo el buffer, limpiamos para ahorrar memoria
            if self.cursor == self.read_buffer.len() {
                self.read_buffer.clear();
                self.cursor = 0;
            }
            return Ok(n);
        }

        // 2. Si no hay datos, intentamos recibir del canal sin bloquear.
        match self.receiver.try_recv() {
            Ok(packet) => {
                println!("DEBUG: UdpStream READ packet of {} bytes", packet.len());
                let n = cmp::min(packet.len(), buf.len());
                buf[..n].copy_from_slice(&packet[..n]);

                // Si el paquete es más grande que el buffer de lectura, guardamos el resto
                if n < packet.len() {
                    self.read_buffer = packet;
                    self.cursor = n;
                }

                Ok(n)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // Retornamos WouldBlock para que OpenSSL sepa que no hay datos por ahora
                Err(io::Error::new(io::ErrorKind::WouldBlock, "No packet in channel"))
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // El canal se cerró
                println!("DEBUG: UdpStream Channel CLOSED (sender dropped)");
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "DTLS Channel closed",
                ))
            }
        }
    }
}

impl Write for UdpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        println!("DEBUG: UdpStream WRITE {} bytes to {}", buf.len(), self.remote_addr);
        // La escritura sigue siendo directa al socket
        let socket = self.socket.lock().unwrap();
        socket.send_to(buf, self.remote_addr)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtlsRole {
    Client,
    Server,
}
pub struct DtlsSession {
    context: SslContext,
    ssl_stream: Option<SslStream<UdpStream>>,
    local_fingerprint: String,
    remote_fingerprint: Option<String>,
    role: DtlsRole,
}

impl DtlsSession {
    pub fn new(role: DtlsRole) -> Result<Self, String> {
        // 1. Generar Certificado y Llave Privada (Self-Signed)
        let rsa = Rsa::generate(2048).map_err(|e| e.to_string())?;
        let pkey = PKey::from_rsa(rsa).map_err(|e| e.to_string())?;

        let mut x509 = X509::builder().map_err(|e| e.to_string())?;
        x509.set_version(2).map_err(|e| e.to_string())?;
        x509.set_pubkey(&pkey).map_err(|e| e.to_string())?;

        // Asignar un "subject name" para que sea un certificado válido
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "webrtc-peer").unwrap();
        let name = name.build();
        x509.set_subject_name(&name).unwrap();

        let not_before = Asn1Time::days_from_now(0).map_err(|e| e.to_string())?;
        let not_after = Asn1Time::days_from_now(365).map_err(|e| e.to_string())?;
        x509.set_not_before(&not_before)
            .map_err(|e| e.to_string())?;
        x509.set_not_after(&not_after).map_err(|e| e.to_string())?;

        // Firma el certificado
        x509.sign(&pkey, MessageDigest::sha256())
            .map_err(|e| e.to_string())?;
        let cert = x509.build();

        // 2. Calcular Fingerprint (SHA-256) para SDP
        let digest = cert
            .digest(MessageDigest::sha256())
            .map_err(|e| e.to_string())?;
        let fingerprint = hex::encode(digest)
            .to_uppercase()
            .as_bytes()
            .chunks(2)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<&str>>()
            .join(":");

        // 3. Configurar Contexto SSL
        let mut ctx = SslContext::builder(SslMethod::dtls()).map_err(|e| e.to_string())?;
        ctx.set_certificate(&cert).map_err(|e| e.to_string())?;
        ctx.set_private_key(&pkey).map_err(|e| e.to_string())?;
        
        // Configurar Mutual TLS: Pedir certificado y aceptar autofirmados (callback retorna true)
        let mut mode = SslVerifyMode::PEER;
        mode.insert(SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        ctx.set_verify_callback(mode, |_, _| true);

        // Habilitar SRTP (RFC 5764)
        ctx.set_tlsext_use_srtp("SRTP_AES128_CM_SHA1_80")
            .map_err(|e| e.to_string())?;

        Ok(Self {
            context: ctx.build(),
            ssl_stream: None,
            local_fingerprint: fingerprint,
            remote_fingerprint: None,
            role,
        })
    }

    pub fn set_remote_fingerprint(&mut self, fp: &str) -> Result<(), String> {
        self.remote_fingerprint = Some(fp.to_string());
        Ok(())
    }

    pub fn certificate_fingerprint(&self) -> String {
        self.local_fingerprint.clone()
    }

    pub fn is_handshake_complete(&self) -> bool {
        self.ssl_stream.is_some()
    }

    pub fn perform_handshake(
        &mut self,
        socket: Arc<Mutex<UdpSocket>>, // Usamos Arc<Mutex> para poder clonarlo dentro del UdpStream
        receiver: Receiver<Vec<u8>>, // El canal por donde llegan los paquetes filtrados (byte 20-63)
        remote_addr: SocketAddr,
    ) -> Result<(), String> {
        println!("DEBUG: Starting DTLS Handshake as {:?} with remote {}", self.role, remote_addr);
        // 1. Crear el wrapper que conecta OpenSSL con el Canal y el Socket
        let stream = UdpStream::new(socket, remote_addr, receiver);

        // 2. Crear la estructura SSL
        let mut ssl = Ssl::new(&self.context).map_err(|e| e.to_string())?;

        // 3. Ejecutar el Handshake (Bloqueante)
        // Manejamos el loop de handshake para soportar retransmisiones (WouldBlock)
        let mut stream_result = match self.role {
            DtlsRole::Client => ssl.connect(stream),
            DtlsRole::Server => ssl.accept(stream),
        };

        let stream = loop {
            match stream_result {
                Ok(s) => break s,
                Err(HandshakeError::WouldBlock(mid_stream)) => {
                    // OpenSSL necesita esperar (timers o datos). Como nuestro UdpStream
                    // retorna WouldBlock en timeout, esto permite que el loop continúe
                    // y OpenSSL verifique si debe retransmitir paquetes perdidos.
                    stream_result = match self.role {
                        DtlsRole::Client => mid_stream.handshake(),
                        DtlsRole::Server => mid_stream.handshake(),
                    };
                }
                Err(HandshakeError::Failure(e)) => return Err(format!("DTLS Handshake Failure: {:?}", e)),
                Err(HandshakeError::SetupFailure(e)) => return Err(format!("DTLS Setup Failure: {:?}", e)),
            }
        };

        println!("DEBUG: DTLS Handshake successfully completed!");

        // 4. VERIFICACIÓN DEL FINGERPRINT (Crucial)
        if let Some(expected_fp) = &self.remote_fingerprint {
            // Obtenemos el certificado que nos envió el peer
            let peer_cert = stream
                .ssl()
                .peer_certificate()
                .ok_or("Peer did not present a certificate")?;

            // Calculamos el hash SHA-256 de ese certificado
            let digest = peer_cert
                .digest(MessageDigest::sha256())
                .map_err(|e| format!("Digest error: {}", e))?;

            // Lo convertimos a formato string "AA:BB:CC..."
            let calculated_fp = hex::encode(digest)
                .to_uppercase()
                .as_bytes()
                .chunks(2)
                .map(|c| std::str::from_utf8(c).unwrap())
                .collect::<Vec<&str>>()
                .join(":");

            // Comparamos
            if calculated_fp != *expected_fp {
                return Err(format!(
                    "Fingerprint mismatch! Expected: {}, Got: {}",
                    expected_fp, calculated_fp
                ));
            }
        }

        // 5. Guardar el stream establecido
        self.ssl_stream = Some(stream);
        println!("DTLS Handshake successfully completed!");

        Ok(())
    }

    pub fn export_srtp_keying_material(&self, len: usize) -> Result<Vec<u8>, String> {
        match &self.ssl_stream {
            Some(s) => {
                let mut buf = vec![0u8; len];
                //Label para WebRTC: "EXTRACTOR-dtls_srtp"
                s.ssl()
                    .export_keying_material(&mut buf, "EXTRACTOR-dtls_srtp", None)
                    .map_err(|e| e.to_string())?;
                Ok(buf)
            }
            None => Err("Handshake not complete".to_string()),
        }
    }

    pub fn write_data(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        if let Some(stream) = &mut self.ssl_stream {
            stream.write(data)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "DTLS not connected"))
        }
    }

    pub fn read_data(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        if let Some(stream) = &mut self.ssl_stream {
            stream.read(buf)
        } else {
             Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "DTLS not connected"))
        }
    }
}
