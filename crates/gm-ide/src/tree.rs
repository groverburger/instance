//! Resource tree panel (specs/ide.md §3.10): selection, double-click to open,
//! inline rename, context menu, drag-and-drop reordering within a kind.

use eframe::egui::{self, collapsing_header::CollapsingState, Color32, Sense, Ui};
use gm_format::{NodeStatus, Project, ResourceKind, TreeNode};

use crate::project_ops::{is_list_kind, kind_label, Placement, TreePath};

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

fn node_icon(n: &TreeNode) -> &'static str {
    match n.status {
        NodeStatus::Root | NodeStatus::Group => "📁",
        _ => match n.kind {
            ResourceKind::Sprite => "🖼",
            ResourceKind::Sound => "🔊",
            ResourceKind::Background => "🏞",
            ResourceKind::Path => "〰",
            ResourceKind::Script => "📜",
            ResourceKind::Font => "🔤",
            ResourceKind::Timeline => "⏱",
            ResourceKind::Object => "🔷",
            ResourceKind::Room => "▦",
            ResourceKind::GameInfo => "ℹ",
            ResourceKind::Settings => "⚙",
            ResourceKind::Extensions => "🧩",
            _ => "•",
        },
    }
}

/// Draw the whole tree.
pub fn tree_ui(ui: &mut Ui, p: &Project, st: &mut TreeState) -> Vec<TreeAction> {
    let mut out = Vec::new();
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, n) in p.resource_tree.iter().enumerate() {
                node_ui(ui, n, vec![i], st, &mut out);
            }
        });
    st.expand_all = None;
    out
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

/// One clickable/draggable node label; returns its response.
fn label(
    ui: &mut Ui,
    n: &TreeNode,
    path: &TreePath,
    st: &mut TreeState,
    out: &mut Vec<TreeAction>,
) {
    if let Some((rp, name)) = st.renaming.as_mut() {
        if rp == path {
            let r = ui.add(egui::TextEdit::singleline(name).desired_width(140.0));
            r.request_focus();
            if r.lost_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    st.renaming = None;
                } else {
                    out.push(TreeAction::FinishRename(path.clone(), name.clone()));
                }
            }
            return;
        }
    }
    let selected = st.selected.as_ref() == Some(path);
    let text = format!("{} {}", node_icon(n), n.name);
    let mut rich = egui::RichText::new(text);
    if n.status == NodeStatus::Root {
        rich = rich.strong();
    }
    let r = ui.add(egui::Button::selectable(selected, rich).sense(Sense::click_and_drag()));
    if r.clicked() {
        st.selected = Some(path.clone());
    }
    if r.double_clicked() {
        out.push(TreeAction::Open(path.clone()));
    }
    if selected && ui.input(|i| i.key_pressed(egui::Key::F2)) && path.len() > 1 {
        out.push(TreeAction::StartRename(path.clone()));
    }
    if path.len() > 1 {
        r.dnd_set_drag_payload(TreeDrag(path.clone()));
    }
    if r.dnd_hover_payload::<TreeDrag>().is_some() {
        let below = ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| p.y > r.rect.center().y);
        let stroke = egui::Stroke::new(2.0, Color32::LIGHT_BLUE);
        if n.status == NodeStatus::Leaf {
            let y = if below { r.rect.bottom() } else { r.rect.top() };
            ui.painter().hline(r.rect.x_range(), y, stroke);
        } else {
            ui.painter()
                .rect_stroke(r.rect, 2.0, stroke, egui::StrokeKind::Outside);
        }
    }
    if let Some(src) = r.dnd_release_payload::<TreeDrag>() {
        let below = ui
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|p| p.y > r.rect.center().y);
        let (to_parent, pos) = if n.status == NodeStatus::Leaf {
            let parent = path[..path.len() - 1].to_vec();
            let i = path[path.len() - 1] + usize::from(below);
            (parent, i)
        } else {
            (path.clone(), n.children.len())
        };
        if src.0 != *path {
            out.push(TreeAction::Move {
                from: src.0.clone(),
                to_parent,
                pos,
            });
        }
    }
    r.context_menu(|ui| context_menu(ui, n, path, out));
}

fn node_ui(
    ui: &mut Ui,
    n: &TreeNode,
    path: TreePath,
    st: &mut TreeState,
    out: &mut Vec<TreeAction>,
) {
    if n.status == NodeStatus::Leaf {
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            label(ui, n, &path, st, out);
        });
        return;
    }
    let id = ui.make_persistent_id(("tree", &path));
    let mut cs = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    if let Some(open) = st.expand_all {
        cs.set_open(open);
    }
    cs.show_header(ui, |ui| label(ui, n, &path, st, out))
        .body(|ui| {
            for (i, c) in n.children.iter().enumerate() {
                let mut cp = path.clone();
                cp.push(i);
                node_ui(ui, c, cp, st, out);
            }
            if n.children.is_empty() {
                ui.label(egui::RichText::new("(empty)").weak().small());
            }
        });
}
