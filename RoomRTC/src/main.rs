mod client;
mod config;
mod logger;
mod server;
mod ui;

use config::AppConfig;

fn main() -> eframe::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "client.conf".to_string());
    let config = match AppConfig::load(&config_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!(
                "No se pudo cargar config {} ({}), usando valores por defecto",
                config_path, err
            );
            AppConfig::default()
        }
    };
    ui::launcher::run(config)
}
