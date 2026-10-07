//! Resource tree panel (specs/ide.md §3.10): selection, double-click to open,
//! inline rename, context menu, drag-and-drop reordering within a kind.
//!
//! Drawn like a classic Windows tree view: dotted connector lines, +/- expander boxes,
//! folder icons for roots and groups, and resource thumbnails for leaves.

use eframe::egui::{self, pos2, vec2, Color32, Id, Painter, Rect, Sense, Stroke, Ui};
use gm_format::{NodeStatus, Project, ResourceKind, TreeNode};

use crate::icons::{self, Icon};
use crate::project_ops::{is_list_kind, kind_label, Placement, TreePath};
use crate::theme;
use crate::widgets::{owner_key, TextureCache};

/// Something the user asked the tree to do; applied by the main window.
#[derive(Debug, Clone)]
pub enum TreeAction {
    Open(TreePath),
    Create(ResourceKind, Placement),
    Duplicate(TreePath),
    Delete(TreePath),
    StartRename(TreePath),
    FinishRename(TreePath, String),
    AddGroup(TreePath),
    Move {
        from: TreePath,
        to_parent: TreePath,
        pos: usize,
    },
}

/// Payload of a tree drag.
#[derive(Debug, Clone)]
struct TreeDrag(TreePath);

/// Tree UI state kept by the app.
#[derive(Default)]
pub struct TreeState {
    pub selected: Option<TreePath>,
    pub renaming: Option<(TreePath, String)>,
    /// Set to force all nodes open (Some(true)) or closed (Some(false)) next frame.
    pub expand_all: Option<bool>,
}

const ROW_H: f32 = 18.0;
const INDENT: f32 = 18.0;
const LINE: Color32 = Color32::from_rgb(160, 160, 160);

/// Everything the row painter needs besides the node.
struct Cx<'a> {
    p: &'a Project,
    st: &'a mut TreeState,
    tex: &'a mut TextureCache,
    gen: u64,
    out: Vec<TreeAction>,
}

/// Draw the whole tree.
pub fn tree_ui(
    ui: &mut Ui,
    p: &Project,
    st: &mut TreeState,
    tex: &mut TextureCache,
    gen: u64,
) -> Vec<TreeAction> {
    let mut cx = Cx {
        p,
        st,
        tex,
        gen,
        out: Vec::new(),
    };
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(2.0);
            let n = p.resource_tree.len();
            for (i, node) in p.resource_tree.iter().enumerate() {
                node_ui(ui, node, vec![i], &mut Vec::new(), i + 1 < n, &mut cx);
            }
        });
    cx.st.expand_all = None;
    cx.out
}

fn dotted_v(p: &Painter, x: f32, y0: f32, y1: f32) {
    let x = x.round() + 0.5;
    let mut y = (y0 / 2.0).ceil() * 2.0;
    while y < y1 {
        p.rect_filled(Rect::from_min_size(pos2(x - 0.5, y), vec2(1.0, 1.0)), 0.0, LINE);
        y += 2.0;
    }
}

fn dotted_h(p: &Painter, y: f32, x0: f32, x1: f32) {
    let y = y.round();
    let mut x = (x0 / 2.0).ceil() * 2.0;
    while x < x1 {
        p.rect_filled(Rect::from_min_size(pos2(x, y), vec2(1.0, 1.0)), 0.0, LINE);
        x += 2.0;
    }
}

/// Thumbnail texture for a leaf (sprite first frame, object's sprite, background).
fn thumbnail(cx: &mut Cx, ctx: &egui::Context, n: &TreeNode) -> Option<(egui::TextureId, egui::Vec2)> {
    let idx = usize::try_from(n.index).ok()?;
    let sprite = |cx: &mut Cx, si: usize| {
        let s = cx.p.sprites.get(si)?.as_ref()?;
        let f = s.frames.first()?;
        let t = cx.tex.get(ctx, owner_key("spr", si), 0, cx.gen, f)?;
        Some((t, vec2(f.width as f32, f.height as f32)))
    };
    match n.kind {
        ResourceKind::Sprite => sprite(cx, idx),
        ResourceKind::Object => {
            let o = cx.p.objects.get(idx)?.as_ref()?;
            sprite(cx, usize::try_from(o.sprite).ok()?)
        }
        ResourceKind::Background => {
            let b = cx.p.backgrounds.get(idx)?.as_ref()?;
            let t = cx.tex.get(ctx, owner_key("bck", idx), 0, cx.gen, &b.image)?;
            Some((t, vec2(b.image.width as f32, b.image.height as f32)))
        }
        _ => None,
    }
}

fn context_menu(ui: &mut Ui, n: &TreeNode, path: &TreePath, out: &mut Vec<TreeAction>) {
    let list = is_list_kind(n.kind);
    let label = kind_label(n.kind);
    match n.status {
        NodeStatus::Leaf if list && path.len() > 1 => {
            if ui.button("Properties").clicked() {
                out.push(TreeAction::Open(path.clone()));
                ui.close();
            }
            if ui.button(format!("Insert {label}")).clicked() {
                out.push(TreeAction::Create(n.kind, Placement::Before(path.clone())));
                ui.close();
            }
            if ui.button(format!("Duplicate {label}")).clicked() {
                out.push(TreeAction::Duplicate(path.clone()));
                ui.close();
            }
            ui.separator();
            if ui.button("Rename").clicked() {
                out.push(TreeAction::StartRename(path.clone()));
                ui.close();
            }
            if ui.button("Delete").clicked() {
                out.push(TreeAction::Delete(path.clone()));
                ui.close();
            }
        }
        NodeStatus::Leaf => {
            if ui.button("Properties").clicked() {
                out.push(TreeAction::Open(path.clone()));
                ui.close();
            }
        }
        _ if list => {
            if ui.button(format!("Create {label}")).clicked() {
                out.push(TreeAction::Create(n.kind, Placement::Into(path.clone())));
                ui.close();
            }
            if ui.button("Add Group").clicked() {
                out.push(TreeAction::AddGroup(path.clone()));
                ui.close();
            }
            if n.status == NodeStatus::Group {
                ui.separator();
                if ui.button("Rename").clicked() {
                    out.push(TreeAction::StartRename(path.clone()));
                    ui.close();
                }
                if ui.button("Delete group").clicked() {
                    out.push(TreeAction::Delete(path.clone()));
                    ui.close();
                }
            }
        }
        _ => {
            ui.label("(no actions)");
        }
    }
}

/// One row; `cont[k]` tells whether the ancestor at depth k has siblings below it
/// (so its vertical connector continues through this row). `more` is the same for
/// this node.
fn node_ui(
    ui: &mut Ui,
    n: &TreeNode,
    path: TreePath,
    cont: &mut Vec<bool>,
    more: bool,
    cx: &mut Cx,
) {
    let depth = cont.len();
    let is_folder = n.status != NodeStatus::Leaf;
    let open_id = Id::new(("tree", &path));
    let mut open = is_folder && ui.data_mut(|d| *d.get_persisted_mut_or(open_id, false));
    if is_folder {
        if let Some(o) = cx.st.expand_all {
            open = o;
            ui.data_mut(|d| d.insert_persisted(open_id, o));
        }
    }
    let width = ui.available_width().max(60.0);
    let (row, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    let left = row.min.x + 2.0;
    let col_x = |k: usize| left + k as f32 * INDENT + INDENT / 2.0;
    let cy = row.center().y;
    let icon_x = left + depth as f32 * INDENT + INDENT;
    let painter = ui.painter().clone();
    let visible = ui.is_rect_visible(row);

    // connector lines
    if visible {
        for (k, c) in cont.iter().enumerate() {
            if *c {
                dotted_v(&painter, col_x(k), row.top(), row.bottom());
            }
        }
        let x = col_x(depth);
        dotted_v(&painter, x, row.top(), if more { row.bottom() } else { cy });
        dotted_h(&painter, cy, x, icon_x - 1.0);
    }

    // expander
    let has_children = is_folder && !n.children.is_empty();
    if has_children {
        let b = egui::Rect::from_center_size(pos2(col_x(depth).round() + 0.5, cy.round() + 0.5), vec2(9.0, 9.0));
        let r = ui.interact(b.expand(2.0), open_id.with("exp"), Sense::click());
        if r.clicked() {
            open = !open;
            ui.data_mut(|d| d.insert_persisted(open_id, open));
        }
        if visible {
            painter.rect(b, 0.0, Color32::WHITE, Stroke::new(1.0, Color32::from_gray(145)), egui::StrokeKind::Middle);
            let ink = Stroke::new(1.0, Color32::from_gray(30));
            painter.line_segment([b.center() - vec2(2.5, 0.0), b.center() + vec2(2.5, 0.0)], ink);
            if !open {
                painter.line_segment([b.center() - vec2(0.0, 2.5), b.center() + vec2(0.0, 2.5)], ink);
            }
        }
    }

    // icon
    let icon_r = Rect::from_min_size(pos2(icon_x, cy - 8.0), vec2(16.0, 16.0));
    if visible {
        if is_folder {
            icons::paint(&painter, icon_r, if open && has_children { Icon::FolderOpen } else { Icon::Folder }, true);
        } else if let Some((t, size)) = thumbnail(cx, ui.ctx(), n) {
            let s = 16.0 / size.x.max(size.y).max(1.0);
            let fit = Rect::from_center_size(icon_r.center(), size * s);
            painter.image(t, fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            icons::paint(&painter, icon_r, Icon::of_kind(n.kind), true);
        }
    }

    // label
    let text_x = icon_r.max.x + 3.0;
    let font = egui::TextStyle::Body.resolve(ui.style());
    let galley = painter.layout_no_wrap(n.name.clone(), font.clone(), Color32::BLACK);
    let label_r = Rect::from_min_size(pos2(text_x, row.min.y + 1.0), vec2(galley.size().x + 6.0, ROW_H - 2.0));
    let renaming = cx.st.renaming.as_ref().is_some_and(|(rp, _)| *rp == path);
    if renaming {
        let mut finished = None;
        if let Some((_, name)) = cx.st.renaming.as_mut() {
            let edit_r = Rect::from_min_size(label_r.min, vec2((row.max.x - text_x - 4.0).max(80.0), ROW_H - 2.0));
            let r = ui.put(edit_r, egui::TextEdit::singleline(name).margin(vec2(2.0, 0.0)));
            r.request_focus();
            if r.lost_focus() {
                finished = Some(name.clone());
            }
        }
        if let Some(name) = finished {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cx.st.renaming = None;
            } else {
                cx.out.push(TreeAction::FinishRename(path.clone(), name));
            }
        }
    } else {
        let hit = Rect::from_min_max(icon_r.min - vec2(1.0, 0.0), label_r.max);
        let r = ui.interact(hit, open_id.with("row"), Sense::click_and_drag());
        r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, cx.st.selected.as_ref() == Some(&path), &n.name));
        let selected = cx.st.selected.as_ref() == Some(&path);
        if visible {
            let color = if selected {
                painter.rect_filled(label_r, 0.0, theme::SELECTION);
                Color32::WHITE
            } else if r.hovered() {
                painter.rect_filled(label_r, 0.0, Color32::from_rgb(229, 243, 255));
                Color32::BLACK
            } else {
                Color32::BLACK
            };
            painter.galley_with_override_text_color(
                pos2(text_x + 3.0, cy - galley.size().y / 2.0),
                galley,
                color,
            );
        }
        if r.clicked() || r.secondary_clicked() {
            cx.st.selected = Some(path.clone());
        }
        if r.double_clicked() {
            if n.status == NodeStatus::Leaf {
                cx.out.push(TreeAction::Open(path.clone()));
            } else if has_children {
                open = !open;
                ui.data_mut(|d| d.insert_persisted(open_id, open));
            }
        }
        if selected && ui.input(|i| i.key_pressed(egui::Key::F2)) && path.len() > 1 {
            cx.out.push(TreeAction::StartRename(path.clone()));
        }
        if path.len() > 1 {
            r.dnd_set_drag_payload(TreeDrag(path.clone()));
        }
        if r.dnd_hover_payload::<TreeDrag>().is_some() {
            let below = ui
                .input(|i| i.pointer.hover_pos())
                .is_some_and(|p| p.y > row.center().y);
            let stroke = Stroke::new(2.0, Color32::from_rgb(0, 120, 215));
            if n.status == NodeStatus::Leaf {
                let y = if below { row.bottom() } else { row.top() };
                painter.hline(hit.x_range(), y, stroke);
            } else {
                painter.rect_stroke(hit, 0.0, stroke, egui::StrokeKind::Inside);
            }
        }
        if let Some(src) = r.dnd_release_payload::<TreeDrag>() {
            let below = ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| p.y > row.center().y);
            let (to_parent, pos) = if n.status == NodeStatus::Leaf {
                let parent = path[..path.len() - 1].to_vec();
                let i = path[path.len() - 1] + usize::from(below);
                (parent, i)
            } else {
                (path.clone(), n.children.len())
            };
            if src.0 != path {
                cx.out.push(TreeAction::Move {
                    from: src.0.clone(),
                    to_parent,
                    pos,
                });
            }
        }
        r.context_menu(|ui| context_menu(ui, n, &path, &mut cx.out));
    }

    if open && has_children {
        cont.push(more);
        let k = n.children.len();
        for (i, c) in n.children.iter().enumerate() {
            let mut cp = path.clone();
            cp.push(i);
            node_ui(ui, c, cp, cont, i + 1 < k, cx);
        }
        cont.pop();
    }
}
