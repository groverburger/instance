//! Classic light Windows look: grey chrome, white lists, blue selection, Segoe UI /
//! Tahoma and Consolas (loaded from the system when present).
//!
//! Hover never changes a widget's size: every interactive state has the same frame
//! stroke width and expansion (see [`apply`]), so nothing reflows when the pointer
//! moves over a button.

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};

pub const PANEL: Color32 = Color32::from_rgb(240, 240, 240);
pub const WORKSPACE: Color32 = Color32::from_rgb(171, 171, 171);
pub const SELECTION: Color32 = Color32::from_rgb(51, 153, 255);
pub const LIST_BG: Color32 = Color32::WHITE;
pub const SHADOW: Color32 = Color32::from_rgb(160, 160, 160);
/// Text colour.
pub const INK: Color32 = Color32::from_gray(10);
/// Frame of sunken list / text areas.
pub const LIST_FRAME: Color32 = Color32::from_rgb(130, 135, 144);
/// Hovered button fill and frame.
pub const HOVER_FILL: Color32 = Color32::from_rgb(229, 241, 251);
pub const HOVER_STROKE: Color32 = Color32::from_rgb(0, 120, 215);
/// Pressed button fill.
pub const PRESSED_FILL: Color32 = Color32::from_rgb(204, 228, 247);
/// Error text.
pub const ERROR_TEXT: Color32 = Color32::from_rgb(190, 0, 0);

/// Frame stroke width of interactive widgets, and their expansion (see [`apply`]).
const EXPANSION: f32 = 1.0;

/// UI fonts in order of preference: Windows, macOS, Linux. TrueType collections
/// (.ttc) are avoided. On the web nothing is found and egui's built-in fonts are used.
const UI_FONTS: &[&str] = &[
    "segoeui.ttf",
    "tahoma.ttf",
    "Arial.ttf",
    "DejaVuSans.ttf",
    "LiberationSans-Regular.ttf",
    "NotoSans-Regular.ttf",
];
const MONO_FONTS: &[&str] = &[
    "consola.ttf",
    "cour.ttf",
    "Courier New.ttf",
    "DejaVuSansMono.ttf",
    "LiberationMono-Regular.ttf",
    "NotoSansMono-Regular.ttf",
];

fn system_font(names: &[&str]) -> Option<Vec<u8>> {
    crate::fonts::find_font(names)
}

/// The UI and monospace system fonts, read once per process.
fn system_fonts() -> &'static (Option<Vec<u8>>, Option<Vec<u8>>) {
    static FONTS: std::sync::OnceLock<(Option<Vec<u8>>, Option<Vec<u8>>)> =
        std::sync::OnceLock::new();
    FONTS.get_or_init(|| (system_font(UI_FONTS), system_font(MONO_FONTS)))
}

/// Install fonts and visuals on `ctx`.
pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let (ui_font, mono_font) = system_fonts();
    if let Some(bytes) = ui_font.clone() {
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
    if let Some(bytes) = mono_font.clone() {
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
    // Every interactive state uses the same 1px frame stroke and an expansion equal to
    // it. egui drops the frame of an unselected selectable button at rest but keeps its
    // inner margin (padding + expansion - stroke), so only expansion == stroke width
    // makes hovered and resting buttons the same size (no reflow on hover). The frame
    // is then painted 1px outside the allocated rect, which the padding / spacing
    // below compensate for (same look as a 2px-padded button with 6px gaps).
    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(7.0, 1.0);
    style.spacing.interact_size.y = 18.0;
    style.spacing.icon_width = 12.0;
    style.spacing.icon_width_inner = 7.0;
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
    v.widgets.inactive.bg_stroke = Stroke::new(EXPANSION, Color32::from_gray(173));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.inactive.corner_radius = r;
    v.widgets.inactive.expansion = EXPANSION;
    // hovered: light blue
    v.widgets.hovered.bg_fill = Color32::from_rgb(229, 241, 251);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(229, 241, 251);
    v.widgets.hovered.bg_stroke = Stroke::new(EXPANSION, Color32::from_rgb(0, 120, 215));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.hovered.corner_radius = r;
    v.widgets.hovered.expansion = EXPANSION;
    // pressed
    v.widgets.active.bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.active.bg_stroke = Stroke::new(EXPANSION, Color32::from_rgb(0, 84, 153));
    v.widgets.active.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.active.corner_radius = r;
    v.widgets.active.expansion = EXPANSION;
    // open menus / combo boxes
    v.widgets.open.bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.open.weak_bg_fill = Color32::from_rgb(204, 228, 247);
    v.widgets.open.bg_stroke = Stroke::new(EXPANSION, Color32::from_rgb(0, 120, 215));
    v.widgets.open.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.open.corner_radius = r;
    v.widgets.open.expansion = EXPANSION;
    v.striped = true;
    style.visuals = v;
    ctx.set_style_of(egui::Theme::Light, style);
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::{by, NodeT, Queryable};

    /// With the theme, every kind of button keeps its size when hovered (egui drops
    /// the frame of unselected selectable buttons at rest, so the frame stroke must
    /// not add to their size).
    #[test]
    fn hover_keeps_widget_sizes() {
        let mut flag = false;
        let mut b = false;
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(760.0, 120.0))
            .build_ui(move |ui| {
                if !flag {
                    super::apply(ui.ctx());
                    flag = true;
                }
                ui.horizontal(|ui| {
                    let _ = ui.button("plain");
                    ui.add(egui::Button::selectable(false, "sel-off"));
                    ui.add(egui::Button::selectable(true, "sel-on"));
                    let _ = ui.selectable_label(false, "label-off");
                    ui.checkbox(&mut b, "check");
                    ui.menu_button("menu", |ui| ui.label("x"));
                    ui.add(egui::Button::new("frameless").frame(false));
                    crate::icons::tool_button(ui, crate::icons::Glyph::Play, "tool", true);
                    let _ = ui.button("last");
                });
            });
        h.run_steps(3);
        let rects = |h: &egui_kittest::Harness<'_>| -> Vec<(String, egui::Rect)> {
            h.query_all(by().recursive(true))
                .filter_map(|n| {
                    let l = n.accesskit_node().label()?;
                    (!l.is_empty()).then(|| (l, n.rect()))
                })
                .collect()
        };
        h.hover_at(egui::pos2(750.0, 110.0));
        h.run_steps(2);
        let before = rects(&h);
        assert!(before.len() >= 9, "{before:?}");
        for (label, r) in &before {
            h.hover_at(r.center());
            h.run_steps(2);
            for ((l0, r0), (_, r1)) in before.iter().zip(&rects(&h)) {
                assert!(
                    (r0.min - r1.min).length() < 0.01 && (r0.max - r1.max).length() < 0.01,
                    "hovering {label:?} moved {l0:?} from {r0:?} to {r1:?}"
                );
            }
        }
    }
}
