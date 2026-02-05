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
    
    // Apply global theme (Discord style)
    // We need a dummy context here or apply it inside launcher::run FIRST frame.
    // However, launcher::run takes ownership.
    // Checking launcher.rs run function usually creates the native options.
    // The theme must be set on the context provided by eframe during setup.
    // So we will modify ui::launcher::run instead to apply theme on startup.
    
    ui::launcher::run(config)
}
