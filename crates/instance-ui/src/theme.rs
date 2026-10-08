//! Classic light Windows look: grey chrome, white lists, blue selection, Segoe UI /
//! Tahoma and Consolas (loaded from the system when present; otherwise the bundled
//! Roboto and egui's monospace font).
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

/// System UI fonts in order of preference, each with its bold face (Windows; Tahoma
/// also ships with some macOS versions). TrueType collections (.ttc) are avoided.
/// Without them the bundled Roboto is used (web, most macOS and Linux systems).
const UI_FONTS: &[(&str, &str)] = &[
    ("segoeui.ttf", "segoeuib.ttf"),
    ("tahoma.ttf", "tahomabd.ttf"),
];
const MONO_FONTS: &[&str] = &[
    "consola.ttf",
    "cour.ttf",
    "Courier New.ttf",
    "DejaVuSansMono.ttf",
    "LiberationMono-Regular.ttf",
    "NotoSansMono-Regular.ttf",
];

/// Roboto (Apache License 2.0, see `fonts/LICENSE.txt`), from the google/fonts
/// repository: the UI font where no system UI font is found.
const ROBOTO_REGULAR: &[u8] = include_bytes!("../fonts/Roboto-Regular.ttf");
const ROBOTO_BOLD: &[u8] = include_bytes!("../fonts/Roboto-Bold.ttf");

/// Font family of bold UI text (the bold face of the UI font), e.g.
/// `RichText::new(..).family(theme::bold())`.
pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

fn system_font(names: &[&str]) -> Option<Vec<u8>> {
    crate::fonts::find_font(names)
}

/// A system UI font: regular face and, when found, its bold face.
type UiFont = (Vec<u8>, Option<Vec<u8>>);

/// The UI regular / bold faces and the monospace font found on the system, read
/// once per process. `INSTANCE_UI_FONT=roboto` skips the system UI font (to check
/// the look of other platforms).
fn system_fonts() -> &'static (Option<UiFont>, Option<Vec<u8>>) {
    static FONTS: std::sync::OnceLock<(Option<UiFont>, Option<Vec<u8>>)> =
        std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let builtin =
            std::env::var("INSTANCE_UI_FONT").is_ok_and(|v| v.eq_ignore_ascii_case("roboto"));
        let ui = UI_FONTS
            .iter()
            .filter(|_| !builtin)
            .find_map(|(regular, bold)| {
                let r = system_font(&[regular])?;
                Some((r, system_font(&[bold])))
            });
        (ui, system_font(MONO_FONTS))
    })
}

/// The font definitions of the theme: the system UI font (Segoe UI / Tahoma) or the
/// bundled Roboto first, egui's fonts after it for missing glyphs (symbols, emoji).
pub fn font_definitions() -> FontDefinitions {
    let (ui_font, mono_font) = system_fonts();
    definitions(ui_font.as_ref(), mono_font.as_deref())
}

fn definitions(ui_font: Option<&UiFont>, mono_font: Option<&[u8]>) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let font = |b: &[u8]| std::sync::Arc::new(FontData::from_owned(b.to_vec()));
    let (regular, bold_face) = match ui_font {
        Some((r, b)) => (font(r), font(b.as_deref().unwrap_or(ROBOTO_BOLD))),
        None => (
            std::sync::Arc::new(FontData::from_static(ROBOTO_REGULAR)),
            std::sync::Arc::new(FontData::from_static(ROBOTO_BOLD)),
        ),
    };
    fonts.font_data.insert("ui".into(), regular);
    fonts.font_data.insert("ui-bold".into(), bold_face);
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "ui".into());
    let mut bold_family = proportional.clone();
    bold_family[0] = "ui-bold".into();
    fonts.families.insert(bold(), bold_family);
    if let Some(bytes) = mono_font {
        fonts.font_data.insert("system-mono".into(), font(bytes));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "system-mono".into());
    }
    fonts
}

/// Install fonts and visuals on `ctx`.
pub fn apply(ctx: &egui::Context) {
    let fonts = font_definitions();
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

    /// Without system fonts the bundled Roboto is the UI font (regular and bold) and
    /// egui's fonts stay as fallbacks.
    #[test]
    fn roboto_fallback() {
        use egui::{Color32, FontFamily, FontId};
        let defs = super::definitions(None, None);
        let prop = &defs.families[&FontFamily::Proportional];
        assert_eq!(prop[0], "ui");
        assert!(prop.len() > 1, "egui fallbacks kept");
        assert_eq!(defs.families[&super::bold()][0], "ui-bold");
        assert_eq!(defs.font_data["ui"].font.len(), super::ROBOTO_REGULAR.len());
        assert_eq!(
            defs.font_data["ui-bold"].font.len(),
            super::ROBOTO_BOLD.len()
        );
        // the faces load and lay out text
        let ctx = egui::Context::default();
        ctx.set_fonts(defs);
        let mut out = ctx.run_ui(Default::default(), |ui| {
            ui.ctx().fonts_mut(|f| {
                let text = "Instance".to_string();
                let g = f.layout_no_wrap(text.clone(), FontId::proportional(13.0), Color32::BLACK);
                let b = f.layout_no_wrap(text, FontId::new(13.0, super::bold()), Color32::BLACK);
                assert!(g.size().x > 30.0);
                assert!(b.size().x > g.size().x, "bold is wider");
            });
        });
        out.textures_delta.clear();
    }

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
                    let size = egui::vec2(80.0, 24.0);
                    crate::icons::glyph_button(ui, crate::icons::Glyph::Check, "OK", "ok", size);
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
        assert!(before.len() >= 10, "{before:?}");
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
