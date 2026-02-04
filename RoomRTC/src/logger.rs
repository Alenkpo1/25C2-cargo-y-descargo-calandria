use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Logger {
    tx: Sender<String>,
}

impl Logger {
    /// No-op logger used as last resort if file logging cannot start.
    #[allow(dead_code)]
    pub fn noop() -> Self {
        let (tx, _rx) = mpsc::channel();
        Logger { tx }
    }

    pub fn start(log_path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = log_path.into();
        let (tx, rx) = mpsc::channel::<String>();

        thread::spawn(move || {
            while let Ok(line) = rx.recv() {
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
                    let _ = writeln!(file, "{}", line);
                }
            }
        });

        Ok(Logger { tx })
    }

    pub fn info(&self, msg: &str) {
        let _ = self.tx.send(format!("[INFO][{}] {}", timestamp(), msg));
    }

    pub fn warn(&self, msg: &str) {
        let _ = self.tx.send(format!("[WARN][{}] {}", timestamp(), msg));
    }

    pub fn error(&self, msg: &str) {
        let _ = self.tx.send(format!("[ERROR][{}] {}", timestamp(), msg));
    }
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_else(|_| 0)
}
