//! Action list, action palette and action argument dialog (specs/ide.md §13.3-13.5),
//! shared by the object and time line forms.

use eframe::egui::{self, Color32, RichText, Sense, Ui, Vec2};
use gm_format::{
    action_kind as ak, arg_kind, builtin_action, builtin_action_summary, builtin_libraries, Action,
    ActionDef, Project, ResourceKind, APPLIES_TO_OTHER, APPLIES_TO_SELF,
};

use super::code_editor::{code_editor, CodeEditorState};
use super::{color_box, none_label, resource_picker};
use crate::editors::Cx;
use crate::project_ops::res_name;

/// Drag-and-drop payload for action lists.
#[derive(Clone, Debug)]
pub enum ActionDrag {
    Palette { lib: usize, def: usize },
    Row { list: egui::Id, index: usize },
}

/// An open argument dialog.
pub struct ArgDialog {
    pub index: usize,
    pub work: Action,
    /// Cancel removes the action (it was just added).
    pub is_new: bool,
    pub code_state: CodeEditorState,
}

/// Per-form state of the action editing widgets.
#[derive(Default)]
pub struct ActionEditState {
    pub selected: Option<usize>,
    pub tab: usize,
    pub dialog: Option<ArgDialog>,
}

/// Resource kind of an argument kind, if it is a resource reference.
pub fn arg_resource_kind(k: i32) -> Option<ResourceKind> {
    Some(match k {
        arg_kind::SPRITE => ResourceKind::Sprite,
        arg_kind::SOUND => ResourceKind::Sound,
        arg_kind::BACKGROUND => ResourceKind::Background,
        arg_kind::PATH => ResourceKind::Path,
        arg_kind::SCRIPT => ResourceKind::Script,
        arg_kind::OBJECT => ResourceKind::Object,
        arg_kind::ROOM => ResourceKind::Room,
        arg_kind::FONT => ResourceKind::Font,
        arg_kind::TIMELINE => ResourceKind::Timeline,
        _ => return None,
    })
}

fn def_of(a: &Action) -> Option<&'static ActionDef> {
    builtin_action(a.library_id, a.action_id)
}

/// Whether the action has an argument dialog.
pub fn has_dialog(a: &Action) -> bool {
    if matches!(
        a.kind,
        ak::BEGIN_GROUP | ak::END_GROUP | ak::ELSE | ak::EXIT
    ) {
        return false;
    }
    if let Some(d) = def_of(a) {
        if d.interface == 1 {
            return false;
        }
    }
    a.argument_count > 0 || a.has_applies_to || a.may_be_relative || a.is_question
}

fn arg_display(a: &Action, i: usize, p: &Project) -> String {
    let v = a.args.get(i).map_or("", String::as_str);
    let k = a.arg_kinds.get(i).copied().unwrap_or(0);
    if let Some(rk) = arg_resource_kind(k) {
        return match v.trim().parse::<i32>() {
            Ok(n) if n >= 0 => res_name(p, rk, n as usize).map_or(v.to_string(), str::to_string),
            _ => none_label(rk).to_string(),
        };
    }
    match k {
        arg_kind::BOOLEAN => if v.trim() == "1" { "true" } else { "false" }.to_string(),
        arg_kind::MENU => def_of(a)
            .and_then(|d| d.args.get(i))
            .and_then(|s| {
                let m = s.menu();
                v.trim()
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| m.get(n).map(|x| x.to_string()))
            })
            .unwrap_or_else(|| v.to_string()),
        arg_kind::COLOR => {
            let c: u32 = v.trim().parse().unwrap_or(0);
            format!("color ${c:06X}")
        }
        _ => v.to_string(),
    }
}

/// One-line description of an action (spec §13.3). List templates are not bundled, so
/// the label is built from the palette title and the argument values.
pub fn action_label(a: &Action, p: &Project) -> (String, bool) {
    let mut s = String::new();
    if a.has_applies_to {
        match a.applies_to {
            APPLIES_TO_SELF => {}
            APPLIES_TO_OTHER => s.push_str("for other: "),
            o => s.push_str(&format!(
                "for all {}: ",
                usize::try_from(o)
                    .ok()
                    .and_then(|o| res_name(p, ResourceKind::Object, o))
                    .unwrap_or("<missing>")
            )),
        }
    }
    if a.is_question && a.negate {
        s.push_str("NOT ");
    }
    let def = def_of(a);
    let title = def.map_or_else(
        || format!("action {}:{}", a.library_id, a.action_id),
        |d| d.palette_title.clone(),
    );
    let italic = a.library_id == 1 && a.action_id == 605;
    match a.kind {
        ak::CODE => {
            let code = a.args.first().map_or("", String::as_str);
            let first = code.lines().next().unwrap_or("");
            if let Some(rest) = first.strip_prefix("///") {
                s.push_str(rest.trim());
            } else {
                s.push_str("Execute a piece of code");
            }
        }
        ak::VARIABLE => {
            s.push_str(&format!(
                "set variable {} {}to {}",
                arg_display(a, 0, p),
                if a.relative { "relative " } else { "" },
                arg_display(a, 1, p)
            ));
        }
        ak::BEGIN_GROUP => s.push_str("Start of a block"),
        ak::END_GROUP => s.push_str("End of a block"),
        ak::ELSE => s.push_str("Else"),
        ak::EXIT => s.push_str("Exit this event"),
        _ if italic => s.push_str(&arg_display(a, 0, p)),
        _ => {
            s.push_str(&title);
            let n = a.argument_count.max(0) as usize;
            if n > 0 {
                let vals: Vec<String> = (0..n).map(|i| arg_display(a, i, p)).collect();
                s.push_str(": ");
                s.push_str(&vals.join(", "));
            }
            if a.may_be_relative && a.relative {
                s.push_str(" (relative)");
            }
        }
    }
    (s, italic)
}

/// Indentation level of each action (blocks nest).
pub fn indent_levels(actions: &[Action]) -> Vec<usize> {
    let mut d: usize = 0;
    actions
        .iter()
        .map(|a| {
            if a.kind == ak::END_GROUP {
                d = d.saturating_sub(1);
            }
            let here = d;
            if a.kind == ak::BEGIN_GROUP {
                d += 1;
            }
            here
        })
        .collect()
}

/// Insert a new action from a definition at `at`; opens its dialog when it has one.
pub fn insert_action(
    actions: &mut Vec<Action>,
    st: &mut ActionEditState,
    def: &ActionDef,
    lib_id: i32,
    at: usize,
) {
    let a = def.instantiate(lib_id);
    let at = at.min(actions.len());
    actions.insert(at, a.clone());
    st.selected = Some(at);
    if has_dialog(&a) {
        st.dialog = Some(ArgDialog {
            index: at,
            work: a,
            is_new: true,
            code_state: CodeEditorState::default(),
        });
    }
}

fn open_dialog(actions: &[Action], st: &mut ActionEditState, i: usize) {
    if let Some(a) = actions.get(i) {
        if has_dialog(a) {
            st.dialog = Some(ArgDialog {
                index: i,
                work: a.clone(),
                is_new: false,
                code_state: CodeEditorState::default(),
            });
        }
    }
}

/// Apply a drop of `payload` before row `at`.
fn apply_drop(
    actions: &mut Vec<Action>,
    st: &mut ActionEditState,
    list: egui::Id,
    payload: &ActionDrag,
    at: usize,
) {
    match payload {
        ActionDrag::Palette { lib, def } => {
            if let Some(l) = builtin_libraries().get(*lib) {
                if let Some(d) = l.actions.get(*def) {
                    insert_action(actions, st, d, l.library_id, at);
                }
            }
        }
        ActionDrag::Row { list: from, index } if *from == list => {
            let i = *index;
            if i < actions.len() {
                let a = actions.remove(i);
                let to = if at > i { at - 1 } else { at }.min(actions.len());
                actions.insert(to, a);
                st.selected = Some(to);
            }
        }
        ActionDrag::Row { .. } => {}
    }
}

/// The action list of one event/moment.
pub fn action_list(ui: &mut Ui, actions: &mut Vec<Action>, st: &mut ActionEditState, cx: &mut Cx) {
    let list_id = ui.make_persistent_id("action_list");
    ui.horizontal(|ui| {
        let sel = st.selected.filter(|&i| i < actions.len());
        if ui
            .add_enabled(sel.is_some(), egui::Button::new("Edit"))
            .clicked()
        {
            open_dialog(actions, st, sel.unwrap_or(0));
        }
        if ui
            .add_enabled(sel.is_some(), egui::Button::new("Delete"))
            .clicked()
        {
            if let Some(i) = sel {
                actions.remove(i);
                st.selected = if actions.is_empty() {
                    None
                } else {
                    Some(i.min(actions.len() - 1))
                };
            }
        }
        if ui
            .add_enabled(sel.is_some_and(|i| i > 0), egui::Button::new("Up"))
            .clicked()
        {
            let i = sel.unwrap_or(0);
            actions.swap(i, i - 1);
            st.selected = Some(i - 1);
        }
        if ui
            .add_enabled(
                sel.is_some_and(|i| i + 1 < actions.len()),
                egui::Button::new("Down"),
            )
            .clicked()
        {
            let i = sel.unwrap_or(0);
            actions.swap(i, i + 1);
            st.selected = Some(i + 1);
        }
        if ui
            .add_enabled(sel.is_some(), egui::Button::new("Copy"))
            .clicked()
        {
            *cx.clipboard = vec![actions[sel.unwrap_or(0)].clone()];
        }
        if ui
            .add_enabled(sel.is_some(), egui::Button::new("Cut"))
            .clicked()
        {
            let i = sel.unwrap_or(0);
            *cx.clipboard = vec![actions.remove(i)];
            st.selected = None;
        }
        if ui
            .add_enabled(!cx.clipboard.is_empty(), egui::Button::new("Paste"))
            .clicked()
        {
            let at = sel.map_or(actions.len(), |i| i + 1);
            for (k, a) in cx.clipboard.iter().enumerate() {
                actions.insert(at + k, a.clone());
            }
            st.selected = Some(at);
        }
        if ui
            .add_enabled(!actions.is_empty(), egui::Button::new("Clear"))
            .clicked()
        {
            actions.clear();
            st.selected = None;
        }
    });
    ui.separator();
    let levels = indent_levels(actions);
    let row_h = 20.0;
    let mut drop: Option<(ActionDrag, usize)> = None;
    let mut open: Option<usize> = None;
    egui::ScrollArea::vertical()
        .id_salt("actions_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = ui.available_width();
            for (i, a) in actions.iter().enumerate() {
                let (r, painter) =
                    ui.allocate_painter(Vec2::new(width, row_h), Sense::click_and_drag());
                let selected = st.selected == Some(i);
                if selected {
                    painter.rect_filled(r.rect, 2.0, ui.visuals().selection.bg_fill);
                } else if r.hovered() {
                    painter.rect_filled(r.rect, 2.0, ui.visuals().widgets.hovered.weak_bg_fill);
                }
                let (label, italic) = action_label(a, cx.project);
                let mut text = RichText::new(label);
                if italic {
                    text = text.italics();
                }
                let marker = if a.is_question {
                    "◆ "
                } else if a.kind == ak::CODE {
                    "▤ "
                } else {
                    "● "
                };
                let x = r.rect.left() + 4.0 + levels[i] as f32 * 16.0;
                let color = if a.is_question {
                    Color32::from_rgb(220, 160, 60)
                } else {
                    ui.visuals().text_color()
                };
                painter.text(
                    egui::pos2(x, r.rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    marker,
                    egui::FontId::proportional(13.0),
                    color,
                );
                let galley = egui::WidgetText::from(text).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    width - (x - r.rect.left()) - 20.0,
                    egui::TextStyle::Body,
                );
                painter.galley(
                    egui::pos2(x + 16.0, r.rect.center().y - galley.size().y / 2.0),
                    galley,
                    ui.visuals().text_color(),
                );
                if r.clicked() {
                    st.selected = Some(i);
                }
                if r.double_clicked() {
                    open = Some(i);
                }
                r.dnd_set_drag_payload(ActionDrag::Row {
                    list: list_id,
                    index: i,
                });
                if let Some(pl) = r.dnd_hover_payload::<ActionDrag>() {
                    let _ = pl;
                    let below = ui
                        .input(|inp| inp.pointer.hover_pos())
                        .is_some_and(|p| p.y > r.rect.center().y);
                    let y = if below { r.rect.bottom() } else { r.rect.top() };
                    painter.hline(
                        r.rect.x_range(),
                        y,
                        egui::Stroke::new(2.0, Color32::LIGHT_BLUE),
                    );
                }
                if let Some(pl) = r.dnd_release_payload::<ActionDrag>() {
                    let below = ui
                        .input(|inp| inp.pointer.interact_pos())
                        .is_some_and(|p| p.y > r.rect.center().y);
                    drop = Some(((*pl).clone(), if below { i + 1 } else { i }));
                }
                r.context_menu(|ui| {
                    if ui.button("Edit Values").clicked() {
                        open = Some(i);
                        ui.close();
                    }
                    if ui.button("Copy").clicked() {
                        *cx.clipboard = vec![a.clone()];
                        ui.close();
                    }
                    if ui.button("Paste").clicked() {
                        drop = None;
                        st.selected = Some(i);
                        open = None;
                        ui.close();
                        // handled below via clipboard
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(list_id.with("paste"), i + 1));
                    }
                    if ui.button("Delete").clicked() {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(list_id.with("delete"), i));
                        ui.close();
                    }
                });
            }
            // Drop area below the last row: append.
            let rest = ui.available_size().max(Vec2::new(width, row_h * 2.0));
            let r = ui.allocate_response(rest, Sense::click());
            if r.clicked() {
                st.selected = None;
            }
            if r.dnd_hover_payload::<ActionDrag>().is_some() {
                ui.painter().hline(
                    r.rect.x_range(),
                    r.rect.top(),
                    egui::Stroke::new(2.0, Color32::LIGHT_BLUE),
                );
            }
            if let Some(pl) = r.dnd_release_payload::<ActionDrag>() {
                drop = Some(((*pl).clone(), actions.len()));
            }
        });
    if let Some(i) = ui
        .ctx()
        .data_mut(|d| d.remove_temp::<usize>(list_id.with("delete")))
    {
        if i < actions.len() {
            actions.remove(i);
            st.selected = None;
        }
    }
    if let Some(at) = ui
        .ctx()
        .data_mut(|d| d.remove_temp::<usize>(list_id.with("paste")))
    {
        for (k, a) in cx.clipboard.iter().enumerate() {
            actions.insert((at + k).min(actions.len()), a.clone());
        }
    }
    if let Some((pl, at)) = drop {
        apply_drop(actions, st, list_id, &pl, at);
    }
    if let Some(i) = open {
        st.selected = Some(i);
        open_dialog(actions, st, i);
    }
}

/// The action palette with one tab per library (spec §13.4). Clicking an action inserts
/// it after the selected one; actions can also be dragged into the list.
pub fn palette(ui: &mut Ui, actions: &mut Vec<Action>, st: &mut ActionEditState) {
    let libs = builtin_libraries();
    ui.horizontal_wrapped(|ui| {
        for (i, l) in libs.iter().enumerate() {
            ui.selectable_value(&mut st.tab, i, &l.tab_caption);
        }
    });
    ui.separator();
    let Some(lib) = libs.get(st.tab) else { return };
    egui::ScrollArea::vertical()
        .id_salt("palette_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            type Row<'a> = Vec<(usize, &'a ActionDef)>;
            let mut row: Row = Vec::new();
            let mut sections: Vec<(Option<String>, Row)> = Vec::new();
            let mut heading: Option<String> = None;
            for (di, d) in lib.actions.iter().enumerate() {
                match d.kind {
                    ak::LABEL => {
                        sections.push((heading.take(), std::mem::take(&mut row)));
                        heading = Some(d.name.clone());
                    }
                    ak::SEPARATOR => {
                        sections.push((heading.take(), std::mem::take(&mut row)));
                    }
                    ak::PLACEHOLDER => {}
                    _ if d.hidden => {}
                    _ => row.push((di, d)),
                }
            }
            sections.push((heading, row));
            for (h, items) in sections {
                if h.is_none() && items.is_empty() {
                    continue;
                }
                if let Some(h) = h {
                    ui.label(RichText::new(h).strong());
                }
                ui.horizontal_wrapped(|ui| {
                    for (di, d) in items {
                        let b = egui::Button::new(RichText::new(&d.palette_title).small())
                            .sense(Sense::click_and_drag());
                        let mut r = ui.add(b);
                        if let Some(s) = builtin_action_summary(lib.library_id, d.action_id) {
                            r = r.on_hover_text(s);
                        }
                        r.dnd_set_drag_payload(ActionDrag::Palette {
                            lib: st.tab,
                            def: di,
                        });
                        if r.clicked() {
                            let at = st.selected.map_or(actions.len(), |i| i + 1);
                            insert_action(actions, st, d, lib.library_id, at);
                        }
                    }
                });
                ui.add_space(4.0);
            }
        });
}

/// Show the argument dialog if open. Returns true when an action was changed.
pub fn arg_dialog(
    ctx: &egui::Context,
    id: egui::Id,
    actions: &mut Vec<Action>,
    st: &mut ActionEditState,
    cx: &mut Cx,
) -> bool {
    let Some(dlg) = st.dialog.as_mut() else {
        return false;
    };
    let mut result: Option<bool> = None;
    let def = def_of(&dlg.work);
    let title = def.map_or_else(|| "Action".to_string(), |d| d.palette_title.clone());
    let is_code = dlg.work.kind == ak::CODE;
    egui::Modal::new(id).show(ctx, |ui| {
        if is_code {
            ui.set_min_size(Vec2::new(640.0, 420.0));
        }
        ui.heading(&title);
        let a = &mut dlg.work;
        if a.has_applies_to {
            ui.horizontal(|ui| {
                ui.label("Applies to:");
                let mut mode = match a.applies_to {
                    APPLIES_TO_SELF => 0,
                    APPLIES_TO_OTHER => 1,
                    _ => 2,
                };
                let old = mode;
                ui.radio_value(&mut mode, 0, "Self");
                ui.radio_value(&mut mode, 1, "Other");
                ui.radio_value(&mut mode, 2, "Object");
                if mode != old {
                    a.applies_to = match mode {
                        0 => APPLIES_TO_SELF,
                        1 => APPLIES_TO_OTHER,
                        _ => tree_first_object(cx.project).unwrap_or(0),
                    };
                }
                if mode == 2 {
                    let mut v = a.applies_to;
                    if resource_picker(
                        ui,
                        "applies_obj",
                        cx.project,
                        ResourceKind::Object,
                        &mut v,
                        "<no object>",
                        -1,
                    ) && v >= 0
                    {
                        a.applies_to = v;
                    }
                }
            });
        }
        if is_code {
            if a.args.is_empty() {
                a.args.push(String::new());
            }
            let mut code = std::mem::take(&mut a.args[0]);
            ui.allocate_ui(Vec2::new(ui.available_width().max(620.0), 360.0), |ui| {
                code_editor(
                    ui,
                    "code_action",
                    &mut code,
                    &mut dlg.code_state,
                    cx.names,
                    cx.prefs,
                    18,
                );
            });
            a.args[0] = code;
        } else {
            let interface = def.map_or(0, |d| d.interface);
            egui::Grid::new(id.with("args"))
                .num_columns(2)
                .show(ui, |ui| {
                    let n = a.argument_count.clamp(0, 8) as usize;
                    if a.args.len() < n {
                        a.args.resize(n, String::new());
                    }
                    for i in 0..n {
                        let caption = def
                            .and_then(|d| d.args.get(i))
                            .map_or_else(|| format!("Argument {i}:"), |s| s.caption.clone());
                        ui.label(caption);
                        let kind = a.arg_kinds.get(i).copied().unwrap_or(0);
                        if interface == 2 && i == 0 {
                            arrow_grid(ui, &mut a.args[0]);
                        } else {
                            arg_widget(ui, i, kind, def, &mut a.args[i], cx);
                        }
                        ui.end_row();
                    }
                });
        }
        if a.may_be_relative {
            ui.checkbox(&mut a.relative, "Relative");
        }
        if a.is_question {
            ui.checkbox(&mut a.negate, "NOT");
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("OK").clicked() {
                result = Some(true);
            }
            if ui.button("Cancel").clicked() {
                result = Some(false);
            }
        });
    });
    match result {
        Some(true) => {
            let d = st.dialog.take().expect("open");
            if let Some(slot) = actions.get_mut(d.index) {
                *slot = d.work;
            }
            true
        }
        Some(false) => {
            let d = st.dialog.take().expect("open");
            if d.is_new && d.index < actions.len() {
                actions.remove(d.index);
                st.selected = None;
            }
            d.is_new
        }
        None => false,
    }
}

fn tree_first_object(p: &Project) -> Option<i32> {
    crate::project_ops::tree_order(p, ResourceKind::Object)
        .into_iter()
        .find(|x| x.0 >= 0)
        .map(|x| x.0)
}

/// Widget for one argument value by kind (spec §13.5).
fn arg_widget(
    ui: &mut Ui,
    i: usize,
    kind: i32,
    def: Option<&ActionDef>,
    v: &mut String,
    cx: &mut Cx,
) {
    if let Some(rk) = arg_resource_kind(kind) {
        let mut n: i32 = v.trim().parse().unwrap_or(-1);
        if resource_picker(ui, ("arg", i), cx.project, rk, &mut n, none_label(rk), -1) {
            *v = n.to_string();
        }
        return;
    }
    match kind {
        arg_kind::BOOLEAN => {
            let mut b = v.trim() == "1";
            egui::ComboBox::from_id_salt(("argb", i))
                .selected_text(if b { "true" } else { "false" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut b, false, "false");
                    ui.selectable_value(&mut b, true, "true");
                });
            let nv = if b { "1" } else { "0" };
            if v != nv {
                *v = nv.to_string();
            }
        }
        arg_kind::MENU => {
            let items: Vec<String> = def
                .and_then(|d| d.args.get(i))
                .map(|s| s.menu().iter().map(|x| x.to_string()).collect())
                .unwrap_or_default();
            let mut sel: usize = v.trim().parse().unwrap_or(0);
            egui::ComboBox::from_id_salt(("argm", i))
                .selected_text(items.get(sel).cloned().unwrap_or_else(|| v.clone()))
                .show_ui(ui, |ui| {
                    for (k, it) in items.iter().enumerate() {
                        ui.selectable_value(&mut sel, k, it);
                    }
                });
            if items.get(sel).is_some() && *v != sel.to_string() {
                *v = sel.to_string();
            }
        }
        arg_kind::COLOR => {
            let mut c: u32 = v.trim().parse().unwrap_or(0);
            if color_box(ui, &mut c) {
                *v = c.to_string();
            }
        }
        _ => {
            if v.contains('\n') || (kind == arg_kind::STRING && v.len() > 40) {
                ui.add(
                    egui::TextEdit::multiline(v)
                        .desired_width(260.0)
                        .desired_rows(3),
                );
            } else {
                ui.add(egui::TextEdit::singleline(v).desired_width(260.0));
            }
        }
    }
}

/// The 3x3 direction toggle grid of "Move Fixed" (numeric keypad order mask).
fn arrow_grid(ui: &mut Ui, v: &mut String) {
    let mut bits: Vec<bool> = v.chars().map(|c| c == '1').collect();
    bits.resize(9, false);
    let labels = ["↙", "↓", "↘", "←", "■", "→", "↖", "↑", "↗"];
    let mut changed = false;
    egui::Grid::new("arrow_grid").show(ui, |ui| {
        for row in [6, 3, 0] {
            for k in row..row + 3 {
                if ui.toggle_value(&mut bits[k], labels[k]).changed() {
                    changed = true;
                }
            }
            ui.end_row();
        }
    });
    if changed {
        *v = bits.iter().map(|&b| if b { '1' } else { '0' }).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_indent() {
        let p = Project::default();
        let block = builtin_action(1, 422).unwrap().instantiate(1);
        let end = builtin_action(1, 424).unwrap().instantiate(1);
        let mut code = builtin_action(1, 603).unwrap().instantiate(1);
        code.args[0] = "/// move player\nx += 1;".into();
        assert_eq!(action_label(&code, &p).0, "move player");
        let acts = vec![block.clone(), code.clone(), end.clone(), code];
        assert_eq!(indent_levels(&acts), vec![0, 1, 0, 0]);
        assert!(!has_dialog(&block));
        let mv = builtin_action(1, 101).unwrap().instantiate(1);
        assert!(has_dialog(&mv));
    }
}
