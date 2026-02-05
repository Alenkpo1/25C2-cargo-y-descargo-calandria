use crate::config::AppConfig;
use crate::ui::screen_manager::MainApp;

pub fn run(config: AppConfig) -> eframe::Result<()> {
    let opt = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([630.0, 400.0])
            .with_min_inner_size([630.0, 400.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "RoomRTC - P2P Video Meets",
        opt,
        Box::new(|cc| {
            crate::ui::theme::configure_visuals(&cc.egui_ctx);
            Ok(Box::new(MainApp::new(config)))
        }),
    )
}
