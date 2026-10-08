//! Action list, action palette and action argument dialog (specs/ide.md §13.3-13.5),
//! shared by the object and time line forms.

use eframe::egui::{self, Color32, RichText, Sense, Ui, Vec2};
use instance_format::{
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

/// One-line description of an action (spec §13.3), built from this project's own
/// templates (see [`super::action_text`]) filled with the argument values. Returns the
/// text and whether it is a comment (shown in italics).
pub fn action_label(a: &Action, p: &Project) -> (String, bool) {
    let mut s = String::new();
    if a.has_applies_to {
        match a.applies_to {
            APPLIES_TO_SELF => {}
            APPLIES_TO_OTHER => s.push_str("for other object: "),
            o => s.push_str(&format!(
                "for all {}: ",
                usize::try_from(o)
                    .ok()
                    .and_then(|o| res_name(p, ResourceKind::Object, o))
                    .unwrap_or("<missing>")
            )),
        }
    }
    let comment = a.library_id == 1 && a.action_id == 605;
    let arg = |i: usize| {
        let v = one_line(&arg_display(a, i, p));
        // menu items read as part of the sentence ("is equal to", "only solid")
        if a.arg_kinds.get(i) == Some(&arg_kind::MENU) && !v.starts_with('<') {
            let mut c = v.chars();
            c.next()
                .map_or_else(String::new, |f| f.to_lowercase().chain(c).collect())
        } else {
            v
        }
    };
    let negate = a.is_question && a.negate;
    let relative = a.may_be_relative && a.relative;
    match a.kind {
        ak::CODE => {
            let code = a.args.first().map_or("", String::as_str);
            let first = code.lines().next().unwrap_or("");
            match first.strip_prefix("///") {
                Some(rest) if !rest.trim().is_empty() => s.push_str(rest.trim()),
                _ => s.push_str("Execute a piece of code"),
            }
        }
        _ if comment => s.push_str(&arg(0)),
        _ if a.library_id == 1 && a.action_id == 601 => {
            s.push_str(&format!("Execute script: {}", arg(0)));
            let n = (a.argument_count.max(1) as usize).min(6);
            let mut extra: Vec<String> = (1..n).map(arg).collect();
            while extra
                .last()
                .is_some_and(|v| v.trim().is_empty() || v.trim() == "0")
            {
                extra.pop();
            }
            if !extra.is_empty() {
                s.push_str(&format!(" ({})", extra.join(", ")));
            }
        }
        _ => {
            let tpl = (a.library_id == 1)
                .then(|| super::action_text::template(a.action_id))
                .flatten();
            match tpl {
                Some(t) => s.push_str(&super::action_text::fill(t, &arg, negate, relative)),
                None => {
                    let mut line = def_of(a).map_or_else(
                        || format!("Action {}:{}", a.library_id, a.action_id),
                        |d| d.palette_title.clone(),
                    );
                    let n = a.argument_count.max(0) as usize;
                    if n > 0 {
                        let vals: Vec<String> = (0..n).map(arg).collect();
                        line.push_str(": ");
                        line.push_str(&vals.join(", "));
                    }
                    if negate {
                        s.push_str("NOT ");
                    }
                    s.push_str(&line);
                    if relative {
                        s.push_str(" (relative)");
                    }
                }
            }
        }
    }
    (s, comment)
}

/// First line of a value, marked with an ellipsis when there is more.
fn one_line(v: &str) -> String {
    let mut lines = v.lines();
    let first = lines.next().unwrap_or("");
    if lines.next().is_some() {
        format!("{first} …")
    } else {
        first.to_string()
    }
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

/// Row height and icon size of the action list.
const ROW_H: f32 = 26.0;
const ROW_ICON: f32 = 24.0;
/// Indentation per block level.
const INDENT: f32 = 16.0;

/// An edit command of the action list, applied after the rows are drawn.
enum ListCmd {
    Edit(usize),
    Cut(usize),
    Copy(usize),
    Paste(usize),
    Delete(usize),
    Clear,
    Select(usize),
    Drop(ActionDrag, usize),
}

/// Paint a white list box with a sunken border (classic list view).
pub fn list_box(p: &egui::Painter, r: egui::Rect) {
    p.rect_filled(r, 0.0, crate::theme::LIST_BG);
    p.rect_stroke(
        r,
        0.0,
        egui::Stroke::new(1.0, Color32::from_gray(130)),
        egui::StrokeKind::Inside,
    );
}

/// The action list of one event/moment: a white list of rows with the action's icon and
/// its description, filling the available space. Double-click edits, right-click opens
/// the edit menu, rows are dragged to reorder, palette actions are dropped in, and while
/// the list has the keyboard focus Delete, Up/Down, Ctrl+C/X/V work (spec §13.3).
pub fn action_list(ui: &mut Ui, actions: &mut Vec<Action>, st: &mut ActionEditState, cx: &mut Cx) {
    let list_id = ui.make_persistent_id("action_list");
    let size = ui.available_size();
    let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
    list_box(ui.painter(), outer);
    let inner = outer.shrink(2.0);
    let bg = ui.interact(inner, list_id, Sense::click());
    let levels = indent_levels(actions);
    let mut cmd: Option<ListCmd> = None;
    let has_focus = ui.memory(|m| m.has_focus(list_id));
    if bg.clicked() {
        ui.memory_mut(|m| m.request_focus(list_id));
    }
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("actions_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let width = ui.available_width();
                for (i, a) in actions.iter().enumerate() {
                    let (rect, r) =
                        ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click_and_drag());
                    let selected = st.selected == Some(i);
                    let (label, comment) = action_label(a, cx.project);
                    let r = r.on_hover_text_at_pointer(&label);
                    r.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            ui.is_enabled(),
                            selected,
                            &label,
                        )
                    });
                    if ui.is_rect_visible(rect) {
                        paint_row(
                            ui,
                            rect,
                            a,
                            levels[i],
                            &label,
                            comment,
                            selected,
                            r.hovered(),
                        );
                    }
                    if r.clicked() || r.secondary_clicked() || r.drag_started() {
                        cmd = Some(ListCmd::Select(i));
                        ui.memory_mut(|m| m.request_focus(list_id));
                    }
                    if r.double_clicked() {
                        cmd = Some(ListCmd::Edit(i));
                    }
                    r.dnd_set_drag_payload(ActionDrag::Row {
                        list: list_id,
                        index: i,
                    });
                    if r.dnd_hover_payload::<ActionDrag>().is_some() {
                        let below = ui
                            .input(|inp| inp.pointer.hover_pos())
                            .is_some_and(|p| p.y > rect.center().y);
                        let y = if below { rect.bottom() } else { rect.top() };
                        insertion_mark(ui.painter(), rect.x_range(), y);
                    }
                    if let Some(pl) = r.dnd_release_payload::<ActionDrag>() {
                        let below = ui
                            .input(|inp| inp.pointer.interact_pos())
                            .is_some_and(|p| p.y > rect.center().y);
                        cmd = Some(ListCmd::Drop((*pl).clone(), if below { i + 1 } else { i }));
                    }
                    r.context_menu(|ui| {
                        if ui
                            .add_enabled(has_dialog(a), egui::Button::new("Edit Values…"))
                            .clicked()
                        {
                            cmd = Some(ListCmd::Edit(i));
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Cut").clicked() {
                            cmd = Some(ListCmd::Cut(i));
                            ui.close();
                        }
                        if ui.button("Copy").clicked() {
                            cmd = Some(ListCmd::Copy(i));
                            ui.close();
                        }
                        if ui
                            .add_enabled(!cx.clipboard.is_empty(), egui::Button::new("Paste"))
                            .clicked()
                        {
                            cmd = Some(ListCmd::Paste(i + 1));
                            ui.close();
                        }
                        if ui.button("Delete").clicked() {
                            cmd = Some(ListCmd::Delete(i));
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Clear").clicked() {
                            cmd = Some(ListCmd::Clear);
                            ui.close();
                        }
                    });
                }
                // Space below the last row: click deselects, drops append.
                let rest = ui.available_size().max(Vec2::new(width, ROW_H));
                let (rect, r) = ui.allocate_exact_size(rest, Sense::click());
                if r.clicked() {
                    st.selected = None;
                    ui.memory_mut(|m| m.request_focus(list_id));
                }
                if r.dnd_hover_payload::<ActionDrag>().is_some() {
                    insertion_mark(ui.painter(), rect.x_range(), rect.top());
                }
                if let Some(pl) = r.dnd_release_payload::<ActionDrag>() {
                    cmd = Some(ListCmd::Drop((*pl).clone(), actions.len()));
                }
                r.context_menu(|ui| {
                    if ui
                        .add_enabled(!cx.clipboard.is_empty(), egui::Button::new("Paste"))
                        .clicked()
                    {
                        cmd = Some(ListCmd::Paste(actions.len()));
                        ui.close();
                    }
                    if ui
                        .add_enabled(!actions.is_empty(), egui::Button::new("Clear"))
                        .clicked()
                    {
                        cmd = Some(ListCmd::Clear);
                        ui.close();
                    }
                });
            });
    });
    if has_focus {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                list_id,
                egui::EventFilter {
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        let sel = st.selected.filter(|&i| i < actions.len());
        let (del, up, down, copy, cut, paste) = ui.input(|i| {
            let ev = |f: fn(&egui::Event) -> bool| i.events.iter().any(f);
            (
                i.key_pressed(egui::Key::Delete),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                ev(|e| matches!(e, egui::Event::Copy)),
                ev(|e| matches!(e, egui::Event::Cut)),
                ev(|e| matches!(e, egui::Event::Paste(_)))
                    || (i.modifiers.command && i.key_pressed(egui::Key::V)),
            )
        });
        if let Some(i) = sel {
            if del {
                cmd = Some(ListCmd::Delete(i));
            } else if copy {
                cmd = Some(ListCmd::Copy(i));
            } else if cut {
                cmd = Some(ListCmd::Cut(i));
            } else if up && i > 0 {
                cmd = Some(ListCmd::Select(i - 1));
            } else if down && i + 1 < actions.len() {
                cmd = Some(ListCmd::Select(i + 1));
            }
        } else if down && !actions.is_empty() {
            cmd = Some(ListCmd::Select(0));
        }
        if paste && !cx.clipboard.is_empty() {
            cmd = Some(ListCmd::Paste(sel.map_or(actions.len(), |i| i + 1)));
        }
    }
    match cmd {
        Some(ListCmd::Select(i)) => st.selected = Some(i),
        Some(ListCmd::Edit(i)) => {
            st.selected = Some(i);
            open_dialog(actions, st, i);
        }
        Some(ListCmd::Copy(i)) => {
            if let Some(a) = actions.get(i) {
                *cx.clipboard = vec![a.clone()];
            }
        }
        Some(ListCmd::Cut(i)) => {
            if i < actions.len() {
                *cx.clipboard = vec![actions.remove(i)];
                st.selected = None;
            }
        }
        Some(ListCmd::Paste(at)) => {
            let at = at.min(actions.len());
            for (k, a) in cx.clipboard.iter().enumerate() {
                actions.insert(at + k, a.clone());
            }
            st.selected = Some(at);
        }
        Some(ListCmd::Delete(i)) => {
            if i < actions.len() {
                actions.remove(i);
                st.selected = if actions.is_empty() {
                    None
                } else {
                    Some(i.min(actions.len() - 1))
                };
            }
        }
        Some(ListCmd::Clear) => {
            actions.clear();
            st.selected = None;
        }
        Some(ListCmd::Drop(pl, at)) => apply_drop(actions, st, list_id, &pl, at),
        None => {}
    }
}

/// Paint one action row: selection, icon (indented by block level) and description.
#[allow(clippy::too_many_arguments)]
fn paint_row(
    ui: &Ui,
    rect: egui::Rect,
    a: &Action,
    level: usize,
    label: &str,
    comment: bool,
    selected: bool,
    hovered: bool,
) {
    let p = ui.painter();
    let x = rect.left() + 3.0 + level as f32 * INDENT;
    let icon = egui::Rect::from_min_size(
        egui::pos2(x, rect.center().y - ROW_ICON / 2.0),
        Vec2::splat(ROW_ICON),
    );
    let text_x = icon.right() + 6.0;
    let mut text = RichText::new(label);
    if comment {
        text = text.italics();
    }
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        (rect.right() - text_x - 4.0).max(10.0),
        egui::TextStyle::Body,
    );
    let text_rect = egui::Rect::from_min_size(
        egui::pos2(text_x - 3.0, rect.center().y - galley.size().y / 2.0 - 1.0),
        galley.size() + Vec2::new(6.0, 2.0),
    );
    if selected {
        p.rect_filled(text_rect, 0.0, crate::theme::SELECTION);
    } else if hovered {
        p.rect_filled(text_rect, 0.0, Color32::from_rgb(229, 241, 251));
    }
    crate::icons::actions::paint_action(p, icon, a.library_id, a.action_id, a.kind, a.is_question);
    let color = if selected {
        Color32::WHITE
    } else if comment {
        Color32::from_rgb(40, 90, 40)
    } else {
        Color32::from_gray(10)
    };
    p.galley(
        egui::pos2(text_x, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

fn insertion_mark(p: &egui::Painter, x: egui::Rangef, y: f32) {
    p.hline(x, y, egui::Stroke::new(2.0, Color32::from_rgb(0, 120, 215)));
}

/// Size of a palette button.
const BTN: f32 = 36.0;
/// Width of the vertical section labels and of the tab strip of the palette.
const SECTION_LABEL_W: f32 = 15.0;
const TAB_W: f32 = 22.0;

/// One palette section: optional label and its grid cells (`None` = empty cell).
struct Section<'a> {
    label: Option<&'a str>,
    cells: Vec<Option<(usize, &'a ActionDef)>>,
}

fn palette_sections(lib: &instance_format::ActionLibrary) -> Vec<Section<'_>> {
    let mut out = vec![Section {
        label: None,
        cells: Vec::new(),
    }];
    for (di, d) in lib.actions.iter().enumerate() {
        let cur = out.last_mut().expect("non-empty");
        match d.kind {
            ak::LABEL => {
                if cur.cells.is_empty() {
                    cur.label = Some(&d.name);
                } else {
                    out.push(Section {
                        label: Some(&d.name),
                        cells: Vec::new(),
                    });
                }
            }
            ak::SEPARATOR => {
                if !cur.cells.is_empty() || cur.label.is_some() {
                    out.push(Section {
                        label: None,
                        cells: Vec::new(),
                    });
                }
            }
            ak::PLACEHOLDER => cur.cells.push(None),
            _ if d.hidden => {}
            _ => cur.cells.push(Some((di, d))),
        }
    }
    out.retain(|s| !s.cells.is_empty());
    out
}

/// Tooltip text of a palette action: its title and a short description.
fn palette_tip(lib_id: i32, d: &ActionDef) -> (String, Option<String>) {
    let desc = builtin_action_summary(lib_id, d.action_id).map(|s| {
        let s = s.strip_prefix("Question: ").unwrap_or(s);
        let s = s
            .split(';')
            .next()
            .unwrap_or(s)
            .trim()
            .trim_end_matches('.');
        format!("{s}.")
    });
    (d.palette_title.clone(), desc)
}

/// The action palette (spec §13.4): the actions of the selected library as a grid of
/// icon buttons in labelled sections, and one vertical tab per library along the right
/// edge. Clicking an action inserts it after the selected one; actions can also be
/// dragged into the list. Fills the available space.
pub fn palette(ui: &mut Ui, actions: &mut Vec<Action>, st: &mut ActionEditState) {
    let libs = builtin_libraries();
    let enabled = ui.is_enabled();
    let size = ui.available_size();
    let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
    let body =
        egui::Rect::from_min_max(outer.min, egui::pos2(outer.right() - TAB_W, outer.bottom()));
    let face = Color32::from_rgb(248, 248, 248);
    let edge = Color32::from_gray(150);
    // tabs
    let mut y = outer.top() + 6.0;
    let mut sel_tab = egui::Rect::NOTHING;
    for (i, l) in libs.iter().enumerate() {
        let galley = ui.painter().layout_no_wrap(
            l.tab_caption.clone(),
            egui::FontId::proportional(12.0),
            Color32::from_gray(10),
        );
        let h = galley.size().x + 14.0;
        let selected = st.tab == i;
        let r = egui::Rect::from_min_size(
            egui::pos2(body.right() - 1.0, y),
            Vec2::new(TAB_W - if selected { 0.0 } else { 3.0 }, h),
        );
        let resp = ui.interact(r, ui.id().with(("palette_tab", i)), Sense::click());
        resp.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                selected,
                &l.tab_caption,
            )
        });
        if resp.clicked() {
            st.tab = i;
        }
        let p = ui.painter();
        let fill = if selected {
            face
        } else if resp.hovered() {
            Color32::from_rgb(229, 241, 251)
        } else {
            Color32::from_gray(225)
        };
        p.rect(
            r,
            egui::CornerRadius {
                nw: 0,
                sw: 0,
                ne: 3,
                se: 3,
            },
            fill,
            egui::Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
        if selected {
            sel_tab = r;
        }
        // text reads top to bottom
        let gh = galley.size().y;
        let pos = egui::pos2(r.center().x + gh / 2.0, r.top() + 7.0);
        p.add(
            egui::epaint::TextShape::new(pos, galley, Color32::from_gray(10))
                .with_angle(std::f32::consts::FRAC_PI_2),
        );
        y += h - 1.0;
    }
    {
        let p = ui.painter();
        p.rect(
            body,
            0.0,
            face,
            egui::Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
        if sel_tab.is_positive() {
            // join the selected tab with the page
            p.vline(
                body.right() - 1.0,
                sel_tab.y_range().shrink(1.0),
                egui::Stroke::new(1.0, face),
            );
        }
    }
    let Some(lib) = libs.get(st.tab) else { return };
    let sections = palette_sections(lib);
    let content = body.shrink2(Vec2::new(4.0, 4.0));
    let cols = (((content.width() - SECTION_LABEL_W - 2.0) / BTN).floor() as usize).max(1);
    let mut clicked: Option<&ActionDef> = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        egui::ScrollArea::vertical()
            .id_salt(("palette_scroll", st.tab))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let width = ui.available_width();
                for (si, sec) in sections.iter().enumerate() {
                    if si > 0 {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(width, 9.0), Sense::hover());
                        let yy = r.center().y.round();
                        let xr = egui::Rangef::new(r.left() + 2.0, r.right() - 2.0);
                        ui.painter().hline(
                            xr,
                            yy - 0.5,
                            egui::Stroke::new(1.0, Color32::from_gray(170)),
                        );
                        ui.painter()
                            .hline(xr, yy + 0.5, egui::Stroke::new(1.0, Color32::WHITE));
                    }
                    let rows = sec.cells.len().div_ceil(cols);
                    let label_galley = sec.label.map(|t| {
                        ui.painter().layout_no_wrap(
                            t.to_string(),
                            egui::FontId::proportional(11.0),
                            Color32::from_gray(70),
                        )
                    });
                    let min_h = label_galley.as_ref().map_or(0.0, |g| g.size().x + 6.0);
                    let h = (rows as f32 * BTN).max(min_h);
                    let (sr, _) = ui.allocate_exact_size(Vec2::new(width, h), Sense::hover());
                    if let Some(g) = label_galley {
                        let gh = g.size().y;
                        let gw = g.size().x;
                        let pos = egui::pos2(
                            sr.left() + (SECTION_LABEL_W - gh) / 2.0,
                            sr.top() + h / 2.0 + gw / 2.0,
                        );
                        ui.painter().add(
                            egui::epaint::TextShape::new(pos, g, Color32::from_gray(70))
                                .with_angle(-std::f32::consts::FRAC_PI_2),
                        );
                    }
                    for (k, cell) in sec.cells.iter().enumerate() {
                        let Some((di, d)) = cell else { continue };
                        let r = egui::Rect::from_min_size(
                            egui::pos2(
                                sr.left() + SECTION_LABEL_W + (k % cols) as f32 * BTN,
                                sr.top() + (k / cols) as f32 * BTN,
                            ),
                            Vec2::splat(BTN),
                        );
                        let resp = ui.interact(
                            r,
                            ui.id().with(("palette_btn", st.tab, *di)),
                            if enabled {
                                Sense::click_and_drag()
                            } else {
                                Sense::hover()
                            },
                        );
                        resp.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                enabled,
                                &d.palette_title,
                            )
                        });
                        paint_palette_button(ui, r, lib.library_id, d, &resp, enabled);
                        let (title, desc) = palette_tip(lib.library_id, d);
                        let resp = resp.on_hover_ui(|ui| {
                            ui.set_max_width(260.0);
                            ui.strong(title);
                            if let Some(desc) = desc {
                                ui.label(desc);
                            }
                        });
                        if enabled {
                            resp.dnd_set_drag_payload(ActionDrag::Palette {
                                lib: st.tab,
                                def: *di,
                            });
                            if resp.clicked() {
                                clicked = Some(d);
                            }
                        }
                    }
                }
            });
    });
    if let Some(d) = clicked {
        let at = st
            .selected
            .map_or(actions.len(), |i| (i + 1).min(actions.len()));
        insert_action(actions, st, d, lib.library_id, at);
    }
    drag_ghost(ui.ctx());
}

/// A raised classic button face with the action's 32×32 icon.
fn paint_palette_button(
    ui: &Ui,
    r: egui::Rect,
    lib_id: i32,
    d: &ActionDef,
    resp: &egui::Response,
    enabled: bool,
) {
    if !ui.is_rect_visible(r) {
        return;
    }
    let mut p = ui.painter().clone();
    let b = r.shrink(1.0);
    let down = enabled && resp.is_pointer_button_down_on();
    let hot = enabled && resp.hovered();
    let fill = if down {
        Color32::from_rgb(204, 228, 247)
    } else if hot {
        Color32::from_rgb(229, 241, 251)
    } else {
        Color32::from_rgb(252, 252, 252)
    };
    p.rect_filled(b, 2.0, fill);
    if hot || down {
        p.rect_stroke(
            b,
            2.0,
            egui::Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
            egui::StrokeKind::Inside,
        );
    } else {
        // bevel: light top-left, dark bottom-right
        let (tl, br) = (Color32::WHITE, Color32::from_gray(160));
        p.hline(b.x_range(), b.top() + 0.5, egui::Stroke::new(1.0, tl));
        p.vline(b.left() + 0.5, b.y_range(), egui::Stroke::new(1.0, tl));
        p.hline(b.x_range(), b.bottom() - 0.5, egui::Stroke::new(1.0, br));
        p.vline(b.right() - 0.5, b.y_range(), egui::Stroke::new(1.0, br));
    }
    if !enabled {
        p.multiply_opacity(0.4);
    }
    let icon = egui::Rect::from_center_size(
        b.center() + if down { Vec2::splat(1.0) } else { Vec2::ZERO },
        Vec2::splat(30.0),
    );
    crate::icons::actions::paint_action(&p, icon, lib_id, d.action_id, d.kind, d.is_question);
}

/// While a palette action is dragged, show its icon under the pointer.
fn drag_ghost(ctx: &egui::Context) {
    let Some(pl) = egui::DragAndDrop::payload::<ActionDrag>(ctx) else {
        return;
    };
    let ActionDrag::Palette { lib, def } = *pl else {
        return;
    };
    let Some(pos) = ctx.pointer_latest_pos() else {
        return;
    };
    let Some(l) = builtin_libraries().get(lib) else {
        return;
    };
    let Some(d) = l.actions.get(def) else { return };
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("action_drag_ghost"),
    ));
    let r = egui::Rect::from_min_size(pos + Vec2::new(8.0, 8.0), Vec2::splat(30.0));
    let mut p = p.clone();
    p.multiply_opacity(0.8);
    crate::icons::actions::paint_action(&p, r, l.library_id, d.action_id, d.kind, d.is_question);
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
    let d = crate::mdi::dialog(ctx, id, &title, |ui| {
        if is_code {
            ui.set_min_size(Vec2::new(640.0, 420.0));
        }
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
            ui.allocate_ui(Vec2::new(620.0, 360.0), |ui| {
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
    if d.close && result.is_none() {
        result = Some(false);
    }
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
