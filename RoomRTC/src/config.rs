use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server_addr: String,
    pub users_file: String,
    pub max_clients: usize,
    pub log_file: String,
    pub video_width: u32,
    pub video_height: u32,
    pub video_fps: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:8443".to_string(),
            //server_addr: "0.0.0.0:8443".to_string(),
            users_file: "users.txt".to_string(),
            max_clients: 100,
            log_file: "roomrtc.log".to_string(),
            video_width: 640,
            video_height: 480,
            video_fps: 30,
        }
    }
}

impl AppConfig {
    pub fn load(path: &str) -> io::Result<Self> {
        let mut cfg = AppConfig::default();
        if !Path::new(path).exists() {
            return Ok(cfg);
        }

        let content = fs::read_to_string(path)?;
        let entries = parse_kv(&content);

        if let Some(addr) = entries.get("server_addr") {
            cfg.server_addr = addr.clone();
        }
        if let Some(users) = entries.get("users_file") {
            cfg.users_file = users.clone();
        }
        if let Some(max) = entries.get("max_clients").and_then(|v| v.parse().ok()) {
            cfg.max_clients = max;
        }
        if let Some(log) = entries.get("log_file") {
            cfg.log_file = log.clone();
        }
        if let Some(w) = entries.get("video_width").and_then(|v| v.parse().ok()) {
            cfg.video_width = w;
        }
        if let Some(h) = entries.get("video_height").and_then(|v| v.parse().ok()) {
            cfg.video_height = h;
        }
        if let Some(fps) = entries.get("video_fps").and_then(|v| v.parse().ok()) {
            cfg.video_fps = fps;
        }

        Ok(cfg)
    }
}

fn parse_kv(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}
