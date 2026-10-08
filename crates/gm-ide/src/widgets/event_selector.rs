//! The event selector ("Choose the Event to Add", specs/ide.md §13.2).

use eframe::egui::{self, Ui};
use gm_format::{event_type as et, Project, ResourceKind};

use crate::events::{key_groups, key_name, mouse_events, other_events};
use crate::project_ops::tree_order;

/// Outcome of one frame of the selector.
pub enum Selection {
    None,
    Chosen(i32, i32),
    Cancel,
}

/// A fixed-size button with the event type's icon and `text`.
fn ev_button(ui: &mut Ui, ty: i32, number: i32, text: &str) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(140.0, 30.0), egui::Sense::click());
    let enabled = ui.is_enabled();
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
    if ui.is_rect_visible(rect) {
        let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&r));
        let v = if open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&r)
        };
        let mut p = ui.painter().clone();
        p.rect(
            rect,
            v.corner_radius,
            v.weak_bg_fill,
            v.bg_stroke,
            egui::StrokeKind::Inside,
        );
        if !enabled {
            p.multiply_opacity(0.4);
        }
        let icon = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.center().y - 10.0),
            egui::Vec2::splat(20.0),
        );
        crate::icons::actions::paint_event(&p, icon, ty, number);
        p.text(
            egui::pos2(icon.right() + 8.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::TextStyle::Button.resolve(ui.style()),
            egui::Color32::from_gray(10),
        );
    }
    r
}

/// An event button that opens a menu of sub-events.
fn ev_menu(ui: &mut Ui, ty: i32, number: i32, text: &str, add: impl FnOnce(&mut Ui)) {
    let r = ev_button(ui, ty, number, text);
    egui::Popup::menu(&r).show(add);
}

fn key_menu(ui: &mut Ui, title: &str, ty: i32, out: &mut Option<(i32, i32)>) {
    ev_menu(ui, ty, 0, title, |ui| {
        for (group, keys) in key_groups() {
            ui.menu_button(group, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for k in keys {
                            if ui.button(format!("<{}>", key_name(k))).clicked() {
                                *out = Some((ty, k));
                                ui.close();
                            }
                        }
                    });
            });
        }
    });
}

/// Show the modal selector.
pub fn event_selector(ctx: &egui::Context, id: egui::Id, p: &Project) -> Selection {
    let mut out: Option<(i32, i32)> = None;
    let mut cancel = false;
    let d = crate::mdi::dialog(ctx, id, "Choose the Event to Add", |ui| {
        egui::Grid::new(id.with("grid"))
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                if ev_button(ui, et::CREATE, 0, "Create").clicked() {
                    out = Some((et::CREATE, 0));
                }
                if ev_button(ui, et::DESTROY, 0, "Destroy").clicked() {
                    out = Some((et::DESTROY, 0));
                }
                ui.end_row();
                ev_menu(ui, et::ALARM, 0, "Alarm", |ui| {
                    for n in 0..12 {
                        if ui.button(format!("Alarm {n}")).clicked() {
                            out = Some((et::ALARM, n));
                            ui.close();
                        }
                    }
                });
                ev_menu(ui, et::STEP, 0, "Step", |ui| {
                    for (n, l) in [(0, "Step"), (1, "Begin Step"), (2, "End Step")] {
                        if ui.button(l).clicked() {
                            out = Some((et::STEP, n));
                            ui.close();
                        }
                    }
                });
                ui.end_row();
                ev_menu(ui, et::COLLISION, 0, "Collision", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for (idx, name, depth) in tree_order(p, ResourceKind::Object) {
                                let text = format!("{}{name}", "    ".repeat(depth));
                                if idx < 0 {
                                    ui.label(egui::RichText::new(text).weak());
                                } else if ui.button(text).clicked() {
                                    out = Some((et::COLLISION, idx));
                                    ui.close();
                                }
                            }
                        });
                });
                key_menu(ui, "Keyboard", et::KEYBOARD, &mut out);
                ui.end_row();
                ev_menu(ui, et::MOUSE, 0, "Mouse", |ui| {
                    for (group, items) in mouse_events() {
                        let mut body = |ui: &mut Ui| {
                            for (n, l) in &items {
                                if ui.button(l).clicked() {
                                    out = Some((et::MOUSE, *n));
                                    ui.close();
                                }
                            }
                        };
                        if group.is_empty() {
                            body(ui);
                        } else {
                            ui.menu_button(group, body);
                        }
                    }
                });
                ev_menu(ui, et::OTHER, -1, "Other", |ui| {
                    let (main, views, user) = other_events();
                    for (n, l) in main {
                        if ui.button(l).clicked() {
                            out = Some((et::OTHER, n));
                            ui.close();
                        }
                    }
                    ui.menu_button("Views", |ui| {
                        for (n, l) in views {
                            if ui.button(l).clicked() {
                                out = Some((et::OTHER, n));
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("User defined", |ui| {
                        for (n, l) in user {
                            if ui.button(l).clicked() {
                                out = Some((et::OTHER, n));
                                ui.close();
                            }
                        }
                    });
                });
                ui.end_row();
                if ev_button(ui, et::DRAW, 0, "Draw").clicked() {
                    out = Some((et::DRAW, 0));
                }
                key_menu(ui, "Key Press", et::KEY_PRESS, &mut out);
                ui.end_row();
                key_menu(ui, "Key Release", et::KEY_RELEASE, &mut out);
                ui.add_enabled_ui(p.triggers.iter().any(Option::is_some), |ui| {
                    ev_menu(ui, et::TRIGGER, 0, "Trigger", |ui| {
                        for (i, t) in p.triggers.iter().enumerate() {
                            if let Some(t) = t {
                                if ui.button(&t.name).clicked() {
                                    out = Some((et::TRIGGER, i as i32));
                                    ui.close();
                                }
                            }
                        }
                    });
                });
                ui.end_row();
            });
        ui.separator();
        if ui.button("Cancel").clicked() {
            cancel = true;
        }
    });
    cancel |= d.close;
    match (out, cancel) {
        (Some((t, n)), _) => Selection::Chosen(t, n),
        (None, true) => Selection::Cancel,
        _ => Selection::None,
    }
}
