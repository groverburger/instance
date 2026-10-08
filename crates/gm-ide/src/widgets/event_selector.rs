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

fn key_menu(ui: &mut Ui, title: &str, ty: i32, out: &mut Option<(i32, i32)>) {
    ui.menu_button(title, |ui| {
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
                if ui.button("Create").clicked() {
                    out = Some((et::CREATE, 0));
                }
                if ui.button("Destroy").clicked() {
                    out = Some((et::DESTROY, 0));
                }
                ui.end_row();
                ui.menu_button("Alarm", |ui| {
                    for n in 0..12 {
                        if ui.button(format!("Alarm {n}")).clicked() {
                            out = Some((et::ALARM, n));
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Step", |ui| {
                    for (n, l) in [(0, "Step"), (1, "Begin Step"), (2, "End Step")] {
                        if ui.button(l).clicked() {
                            out = Some((et::STEP, n));
                            ui.close();
                        }
                    }
                });
                ui.end_row();
                ui.menu_button("Collision", |ui| {
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
                ui.menu_button("Mouse", |ui| {
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
                ui.menu_button("Other", |ui| {
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
                if ui.button("Draw").clicked() {
                    out = Some((et::DRAW, 0));
                }
                key_menu(ui, "Key Press", et::KEY_PRESS, &mut out);
                ui.end_row();
                key_menu(ui, "Key Release", et::KEY_RELEASE, &mut out);
                ui.add_enabled_ui(p.triggers.iter().any(Option::is_some), |ui| {
                    ui.menu_button("Trigger", |ui| {
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
