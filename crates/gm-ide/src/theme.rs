//! Classic light Windows look: grey chrome, white lists, blue selection, Segoe UI
//! (loaded from the system when present).

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};

pub const PANEL: Color32 = Color32::from_rgb(240, 240, 240);
pub const WORKSPACE: Color32 = Color32::from_rgb(171, 171, 171);
pub const SELECTION: Color32 = Color32::from_rgb(51, 153, 255);
pub const LIST_BG: Color32 = Color32::WHITE;
pub const SHADOW: Color32 = Color32::from_rgb(160, 160, 160);

fn system_font(names: &[&str]) -> Option<Vec<u8>> {
    let dirs: Vec<std::path::PathBuf> = if cfg!(windows) {
        let root = std::env::var_os("WINDIR").map_or_else(
            || std::path::PathBuf::from(r"C:\Windows"),
            std::path::PathBuf::from,
        );
        vec![root.join("Fonts")]
    } else {
        vec![
            "/usr/share/fonts/truetype/dejavu".into(),
            "/usr/share/fonts/TTF".into(),
            "/Library/Fonts".into(),
        ]
    };
    for d in dirs {
        for n in names {
            if let Ok(b) = std::fs::read(d.join(n)) {
                return Some(b);
            }
        }
    }
    None
}

/// Install fonts and visuals on `ctx`.
pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    if let Some(bytes) = system_font(&["segoeui.ttf", "tahoma.ttf"]) {
        fonts.font_data.insert(
            "system-ui".into(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "system-ui".into());
    }
    if let Some(bytes) = system_font(&["consola.ttf", "cour.ttf"]) {
        fonts.font_data.insert(
            "system-mono".into(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "system-mono".into());
    }
    ctx.set_fonts(fonts);

    ctx.set_theme(egui::Theme::Light);
    let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(10.5)),
        (TextStyle::Body, FontId::proportional(12.5)),
        (TextStyle::Button, FontId::proportional(12.5)),
        (TextStyle::Heading, FontId::proportional(16.0)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 2.0);
    style.spacing.interact_size.y = 20.0;
    style.spacing.menu_margin = egui::Margin::same(2);

    let mut v = egui::Visuals::light();
    let r = CornerRadius::same(2);
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.faint_bg_color = Color32::from_rgb(246, 246, 246);
    v.extreme_bg_color = Color32::WHITE;
    v.code_bg_color = Color32::WHITE;
    v.window_corner_radius = CornerRadius::same(0);
    v.menu_corner_radius = CornerRadius::same(0);
    v.window_stroke = Stroke::new(1.0, Color32::from_gray(130));
    v.window_shadow = egui::Shadow {
        offset: [2, 2],
        blur: 6,
        spread: 0,
        color: Color32::from_black_alpha(60),
    };
    v.popup_shadow = egui::Shadow {
        offset: [2, 2],
        blur: 4,
        spread: 0,
        color: Color32::from_black_alpha(50),
    };
    v.selection.bg_fill = SELECTION;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.hyperlink_color = Color32::from_rgb(0, 102, 204);
    v.override_text_color = None;

    let ink = Color32::from_gray(10);
    // non-interactive (labels, frames)
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.weak_bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_gray(190));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.noninteractive.corner_radius = r;
    // buttons at rest
    v.widgets.inactive.bg_fill = Color32::from_rgb(225, 225, 225);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(225, 225, 225);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(173));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.inactive.corner_radius = r;
    v.widgets.inactive.expansion = 0.0;
    // hovered: light blue
    v.widgets.hovered.bg_fill = Color32::from_rgb(229, 241, 251);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(229, 241, 251);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0, 120, 215));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.hovered.corner_radius = r;
    v.widgets.hovered.expansion = 0.0;
    // pressed
    v.widgets.active.bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0, 84, 153));
    v.widgets.active.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.active.corner_radius = r;
    v.widgets.active.expansion = 0.0;
    // open menus / combo boxes
    v.widgets.open.bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.open.weak_bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.open.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0, 120, 215));
    v.widgets.open.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.open.corner_radius = r;
    v.striped = true;
    style.visuals = v;
    ctx.set_style_of(egui::Theme::Light, style);
}
