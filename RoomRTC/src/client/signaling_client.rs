use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerName, StreamOwned};

#[derive(Debug, Clone)]
pub enum SignalingEvent {
    Registered(String),
    RegisterError(String),
    LoginSuccess(()),
    LoginError(String),
    LoggedOut,
    UserList(Vec<(String, String)>),
    UserStatusChanged {
        username: String,
        status: String,
    },
    IncomingCall {
        from: String,
        sdp: String,
    },
    CallAccepted {
        from: String,
        sdp: String,
    },
    CallRejected {
        from: String,
    },
    CallEnded {
        from: String,
    },
    IceCandidate {
        from: String,
        candidate: String,
    },
    Error(String),
    Disconnected,
}

pub struct SignalingClient {
    outgoing: Sender<String>,
    receiver: Receiver<SignalingEvent>,
}

impl SignalingClient {
    pub fn connect(server_addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(server_addr)?;
        stream.set_read_timeout(Some(Duration::from_millis(200)))?;

        let server_name = parse_server_name(server_addr)?;
        let config = build_client_config();
        let connection = ClientConnection::new(config, server_name)
            .map_err(|e| std::io::Error::other(format!("Error TLS: {}", e)))?;
        let tls_stream = StreamOwned::new(connection, stream);

        let (event_tx, event_rx) = mpsc::channel::<SignalingEvent>();
        let (out_tx, out_rx) = mpsc::channel::<String>();

        thread::spawn(move || {
            run_client_loop(tls_stream, event_tx, out_rx);
        });

        Ok(Self {
            outgoing: out_tx,
            receiver: event_rx,
        })
    }

    pub fn try_next_event(&self) -> Option<SignalingEvent> {
        self.receiver.try_recv().ok()
    }

    pub fn register(&self, username: &str, password: &str) -> std::io::Result<()> {
        let msg = format!("REGISTER|username:{}|password:{}", username, password);
        self.send_message(&msg)
    }

    pub fn login(&self, username: &str, password: &str) -> std::io::Result<()> {
        let msg = format!("LOGIN|username:{}|password:{}", username, password);
        self.send_message(&msg)
    }

    pub fn logout(&self) -> std::io::Result<()> {
        self.send_message("LOGOUT")
    }

    pub fn request_users(&self) -> std::io::Result<()> {
        self.send_message("GET_USERS")
    }

    pub fn call(&self, to: &str, sdp: &str) -> std::io::Result<()> {
        let msg = format!(
            "CALL_OFFER|to:{}|sdp:{}",
            to, escape_payload(sdp)
        );
        self.send_message(&msg)
    }

    pub fn answer_call(&self, to: &str, sdp: &str) -> std::io::Result<()> {
        let msg = format!(
            "CALL_ANSWER|to:{}|accept:true|sdp:{}",
            to, escape_payload(sdp)
        );
        self.send_message(&msg)
    }

    pub fn reject_call(&self, to: &str) -> std::io::Result<()> {
        let msg = format!("CALL_REJECT|to:{}", to);
        self.send_message(&msg)
    }

    pub fn end_call(&self, to: &str) -> std::io::Result<()> {
        let msg = format!("CALL_END|to:{}", to);
        self.send_message(&msg)
    }

    fn send_message(&self, msg: &str) -> std::io::Result<()> {
        self.outgoing
            .send(msg.to_string())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
    }
}

fn build_client_config() -> Arc<ClientConfig> {
    let root_store = RootCertStore::empty();
    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(InsecureVerifier));
    Arc::new(config)
}

struct InsecureVerifier;

impl ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

fn parse_server_name(_addr: &str) -> std::io::Result<ServerName> {
    ServerName::try_from("roomrtc.local")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
}

fn run_client_loop(
    tls_stream: StreamOwned<ClientConnection, TcpStream>,
    event_tx: Sender<SignalingEvent>,
    outgoing: Receiver<String>,
) {
    let mut reader = BufReader::new(tls_stream);

    loop {
        if let Err(e) = flush_outgoing(&mut reader, &outgoing) {
            let _ = event_tx.send(SignalingEvent::Disconnected);
            eprintln!("Error sending message: {}", e);
            break;
        }

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let _ = event_tx.send(SignalingEvent::Disconnected);
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let msg = parse_message(trimmed);
                if let Some(event) = map_to_event(msg) {
                    let _ = event_tx.send(event);
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => {
                let _ = event_tx.send(SignalingEvent::Error(format!("Connection close: {}", e)));
                break;
            }
        }
    }
}

fn flush_outgoing(
    reader: &mut BufReader<StreamOwned<ClientConnection, TcpStream>>,
    outgoing: &Receiver<String>,
) -> std::io::Result<()> {
    while let Ok(msg) = outgoing.try_recv() {
        let stream = reader.get_mut();
        stream.write_all(msg.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;
    }
    Ok(())
}

fn parse_message(msg: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let parts: Vec<&str> = msg.split('|').collect();

    if !parts.is_empty() {
        map.insert("type".to_string(), parts[0].to_string());

        for part in &parts[1..] {
            if let Some(pos) = part.find(':') {
                let key = &part[..pos];
                let value = &part[pos + 1..];
                map.insert(key.to_string(), value.to_string());
            }
        }
    }

    map
}

fn map_to_event(msg: HashMap<String, String>) -> Option<SignalingEvent> {
    let msg_type = msg.get("type")?.as_str();

    let missing = |field: &str| {
        Some(SignalingEvent::Error(format!(
            "Campo faltante '{}' en {}",
            field, msg_type
        )))
    };

    match msg_type {
        "REGISTER_SUCCESS" => {
            let message = msg.get("message")?.clone();
            Some(SignalingEvent::Registered(message))
        }
        "REGISTER_ERROR" => {
            let error = msg.get("error")?.clone();
            Some(SignalingEvent::RegisterError(error))
        }
        "LOGIN_SUCCESS" => Some(SignalingEvent::LoginSuccess(())),
        "LOGIN_ERROR" => {
            let error = msg.get("error")?.clone();
            Some(SignalingEvent::LoginError(error))
        }
        "LOGOUT_SUCCESS" => Some(SignalingEvent::LoggedOut),
        "USER_LIST" => {
            let mut users = Vec::new();
            for (key, value) in msg.iter() {
                if key != "type" {
                    users.push((key.clone(), value.clone()));
                }
            }
            Some(SignalingEvent::UserList(users))
        }
        "USER_STATUS_CHANGED" => {
            let username = msg.get("username").cloned()?;
            let status = msg.get("status").cloned()?;
            Some(SignalingEvent::UserStatusChanged { username, status })
        }
        "INCOMING_CALL" => {
            let from = msg.get("from").cloned()?;
            let sdp = unescape_payload(msg.get("sdp"));
            Some(SignalingEvent::IncomingCall {
                from,
                sdp,
            })
        }
        "CALL_ACCEPTED" => {
            let from = msg.get("from").cloned()?;
            let sdp = unescape_payload(msg.get("sdp"));
            Some(SignalingEvent::CallAccepted {
                from,
                sdp,
            })
        }
        "CALL_REJECTED" => {
            let from = msg.get("from").cloned()?;
            Some(SignalingEvent::CallRejected { from })
        }
        "CALL_ENDED" => {
            let from = msg.get("from").cloned()?;
            Some(SignalingEvent::CallEnded { from })
        }
        "ICE_CANDIDATE" => {
            let from = msg.get("from").cloned()?;
            let candidate = unescape_payload(msg.get("candidate"));
            Some(SignalingEvent::IceCandidate { from, candidate })
        }
        "ERROR" | "CALL_ERROR" => {
            let err = msg.get("error").cloned()?;
            Some(SignalingEvent::Error(err))
        }
        _ => missing("type"),
    }
}

fn escape_payload(data: &str) -> String {
    let mut out = String::with_capacity(data.len());
    for ch in data.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_payload(value: Option<&String>) -> String {
    let Some(raw) = value else {
        return String::new();
    };
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push(other);
                }
                None => break,
            }
        } else {
            out.push(ch);
        }
    }
    out
}
