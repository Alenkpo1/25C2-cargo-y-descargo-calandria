use crate::ui::screens::lobby::LobbyAction;
use crate::ui::screens::status_utils::ui_status::egui::Button;
use eframe::egui;
use eframe::egui::{Color32, Stroke, Vec2};
pub enum Status {
    Connected,
    Disconnected,
    Busy,
}

impl Status {
    fn color(&self) -> Color32 {
        match self {
            Status::Connected => Color32::GREEN,
            Status::Disconnected => Color32::GRAY,
            Status::Busy => Color32::RED,
        }
    }

    pub fn is_callable(&self, user_name: &str, current_user: Option<&str>) -> bool {
        matches!(self, Status::Connected) && Some(user_name) != current_user
    }
}

pub fn user_row(
    ui: &mut eframe::egui::Ui,
    name: &str,
    status: &str,
    current_user: Option<&str>,
) -> Option<LobbyAction> {
    //DEBUG: println!("Drawing user: {} with status: {}", name, status);
    let status = match status {
        "AVAILABLE" => Status::Connected,
        "DISCONNECTED" => Status::Disconnected,
        "BUSY" => Status::Busy,
        _ => Status::Disconnected,
    };
    let mut action = None;
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        let avatar_size = 28.0;

        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(avatar_size, avatar_size),
            eframe::egui::Sense::hover(),
        );

        let painter = ui.painter();

        painter.circle_stroke(
            rect.center(),
            avatar_size * 0.45,
            Stroke {
                width: 2.0,
                color: status.color(),
            },
        );

        let initial = name.chars().next().unwrap_or('?');

        painter.text(
            rect.center(),
            eframe::egui::Align2::CENTER_CENTER,
            initial.to_string(),
            eframe::egui::FontId::proportional(16.0),
            ui.visuals().text_color(),
        );
        ui.label(name);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if status.is_callable(name, current_user) {
                let call_btn = Button::new(egui::RichText::new("📷 Call").color(egui::Color32::WHITE))
                    .fill(crate::ui::theme::colors::SUCCESS)
                    .rounding(4.0)
                    .min_size(Vec2::new(60.0, 24.0));
                    
                let res_call_btn = ui.add(call_btn);
                if res_call_btn.clicked() {
                    action = Some(LobbyAction::GoToWaitingCall(name.to_string()));
                }
            } else {
                ui.label(egui::RichText::new("Busy/Offline").size(10.0).color(crate::ui::theme::colors::TEXT_MUTED));
            }
        });
    });
    action
}
