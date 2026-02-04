//! Estado global del servidor de señalización.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::sync::mpsc::Sender;
use std::sync::RwLock;

use crate::config::AppConfig;
use crate::logger::Logger;

use super::types::{ConnectedClient, User, UserStatus};
use super::validation::{validate_password, validate_username};

/// Estado compartido del servidor.
pub struct ServerState {
    pub users_file: String,
    pub users: RwLock<HashMap<String, User>>,
    pub connected_clients: RwLock<HashMap<String, ConnectedClient>>,
    pub user_statuses: RwLock<HashMap<String, UserStatus>>,
    pub active_calls: RwLock<HashMap<String, String>>, // caller -> callee
    pub logger: Logger,
}

impl ServerState {
    pub fn new(config: &AppConfig, logger: Logger) -> Self {
        Self {
            users_file: config.users_file.clone(),
            users: RwLock::new(HashMap::new()),
            connected_clients: RwLock::new(HashMap::new()),
            user_statuses: RwLock::new(HashMap::new()),
            active_calls: RwLock::new(HashMap::new()),
            logger,
        }
    }

    pub fn load_users(&self) -> std::io::Result<()> {
        let file = match File::open(&self.users_file) {
            Ok(f) => f,
            Err(_) => {
                File::create(&self.users_file)?;
                return Ok(());
            }
        };

        let reader = BufReader::new(file);
        let mut users = self
            .users
            .write()
            .map_err(|_| io::Error::other("users lock poisoned"))?;
        let mut statuses = self
            .user_statuses
            .write()
            .map_err(|_| io::Error::other("statuses lock poisoned"))?;

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split(':').collect();

            if parts.len() >= 2 {
                let metadata = match parts.get(2) {
                    Some(val) => val.to_string(),
                    None => String::new(),
                };
                let user = User {
                    username: parts[0].to_string(),
                    password: parts[1].to_string(),
                    metadata,
                };
                statuses.insert(user.username.clone(), UserStatus::Disconnected);
                users.insert(user.username.clone(), user);
            }
        }
        self.logger
            .info(&format!("Usuarios cargados desde {}", self.users_file));

        Ok(())
    }

    pub fn save_user(&self, user: &User) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.users_file)?;

        writeln!(
            file,
            "{}:{}:{}",
            user.username, user.password, user.metadata
        )?;
        Ok(())
    }

    pub fn register_user(&self, username: String, password: String) -> Result<(), String> {
        validate_username(&username)?;
        validate_password(&password)?;
        let mut users = self
            .users
            .write()
            .map_err(|_| "Users lock poisoned".to_string())?;

        if users.contains_key(&username) {
            return Err("User already exist".to_string());
        }

        let user = User {
            username: username.clone(),
            password,
            metadata: String::new(),
        };

        if let Err(e) = self.save_user(&user) {
            return Err(format!("Error saving user: {}", e));
        }

        users.insert(username.clone(), user);

        let mut statuses = self
            .user_statuses
            .write()
            .map_err(|_| "Statuses lock poisoned".to_string())?;
        statuses.insert(username, UserStatus::Disconnected);

        self.logger
            .info("Nuevo usuario registrado en el archivo de usuarios");
        Ok(())
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<(), String> {
        validate_username(username)?;
        validate_password(password)?;
        let users = self
            .users
            .read()
            .map_err(|_| "Users lock poisoned".to_string())?;

        match users.get(username) {
            Some(user) if user.password == password => Ok(()),
            Some(_) => Err("Invalid password".to_string()),
            None => Err("User does not exist".to_string()),
        }
    }

    pub fn get_user_list(&self) -> Vec<(String, UserStatus)> {
        let statuses = match self.user_statuses.read() {
            Ok(guard) => guard,
            Err(_) => {
                self.logger.error("Statuses lock poisoned");
                return Vec::new();
            }
        };
        let users = match self.users.read() {
            Ok(guard) => guard,
            Err(_) => {
                self.logger.error("Users lock poisoned");
                return Vec::new();
            }
        };

        users
            .keys()
            .map(|u| {
                let status = match statuses.get(u) {
                    Some(st) => st.clone(),
                    None => UserStatus::Disconnected,
                };
                (u.clone(), status)
            })
            .collect()
    }

    pub fn set_user_status(&self, username: &str, status: UserStatus) {
        let mut statuses = match self.user_statuses.write() {
            Ok(guard) => guard,
            Err(_) => {
                self.logger
                    .error("No se pudo actualizar estado: lock envenenado");
                return;
            }
        };
        statuses.insert(username.to_string(), status.clone());
        drop(statuses);

        // Notificar a todos los clientes conectados
        let clients = match self.connected_clients.read() {
            Ok(guard) => guard,
            Err(_) => {
                self.logger
                    .error("No se pudo notificar estado: lock envenenado");
                return;
            }
        };
        let msg = format!(
            "USER_STATUS_CHANGED|username:{}|status:{}",
            username,
            status.to_string()
        );

        for client in clients.values() {
            Self::send_message(&client.sender, &msg);
        }
        self.logger
            .info(&format!("Estado de {} -> {}", username, status.to_string()));
    }

    pub fn send_message(sender: &Sender<String>, msg: &str) {
        let _ = sender.send(msg.to_string());
    }
}
