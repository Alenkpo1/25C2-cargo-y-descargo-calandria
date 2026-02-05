use eframe::egui::{self, Color32, Visuals, Style, Rounding, Stroke, Shadow, FontId, FontFamily, TextStyle};

pub mod colors {
    use eframe::egui::Color32;

    pub const BACKGROUND: Color32 = Color32::from_rgb(54, 57, 63);       // rgba(20, 20, 32, 1)
    pub const BACKGROUND_SECONDARY: Color32 = Color32::from_rgb(47, 49, 54); // #121213ff
    pub const BACKGROUND_TERTIARY: Color32 = Color32::from_rgb(32, 34, 37);  // #202225
    
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 221, 222);      // #dcddde
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(114, 118, 125);        // #72767d
    
    pub const PRIMARY: Color32 = Color32::from_rgb(88, 101, 242);            // #5865F2 (Blurple)
    pub const PRIMARY_HOVER: Color32 = Color32::from_rgb(71, 82, 196);       // #4752c4
    
    pub const SUCCESS: Color32 = Color32::from_rgb(87, 242, 135);            // #57F287
    pub const DANGER: Color32 = Color32::from_rgb(237, 66, 69);              // #ED4245
    
    pub const BORDER: Color32 = Color32::from_rgb(32, 34, 37);               // #202225
}

pub fn configure_visuals(ctx: &eframe::egui::Context) {
    let mut visuals = Visuals::dark();
    
    visuals.window_fill = colors::BACKGROUND;
    visuals.panel_fill = colors::BACKGROUND_SECONDARY;
    
    // Widgets
    visuals.widgets.noninteractive.bg_fill = colors::BACKGROUND;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
    
    visuals.widgets.inactive.bg_fill = colors::BACKGROUND_TERTIARY;
    visuals.widgets.inactive.rounding = Rounding::same(4.0);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
    
    visuals.widgets.hovered.bg_fill = colors::BACKGROUND_SECONDARY;
    visuals.widgets.hovered.rounding = Rounding::same(4.0);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
    
    visuals.widgets.active.bg_fill = colors::PRIMARY;
    visuals.widgets.active.rounding = Rounding::same(4.0);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    
    visuals.selection.bg_fill = colors::PRIMARY;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    
    ctx.set_visuals(visuals);
    
    // Styles
    let mut style = (*ctx.style()).clone();
    style.visuals.window_shadow = Shadow::default();
    style.visuals.popup_shadow = Shadow::default();
    style.spacing.item_spacing = eframe::egui::vec2(10.0, 10.0);
    style.spacing.button_padding = eframe::egui::vec2(16.0, 8.0);
    
    ctx.set_style(style);
}
