//! Main window (specs/ide.md §1, §3): menus, toolbar, resource tree, form windows,
//! save prompts and running the game.

use std::path::{Path, PathBuf};
use std::process::Child;

use eframe::egui::{self, Context, Id, Vec2};
use gm_format::{Action, NodeStatus, Project, ResourceKind};

use crate::editors::{open_editor, Cx, Editor, IdAlloc, Request, ResKey};
use crate::icons::{self, Icon};
use crate::mdi::{self, MdiState};
use crate::prefs::Prefs;
use crate::project_ops::{self as ops, Placement, TreePath, LIST_KINDS};
use crate::theme;
use crate::tree::{tree_ui, TreeAction, TreeState};
use crate::widgets::code_editor::ProjectNames;
use crate::widgets::TextureCache;

struct OpenForm {
    editor: Box<dyn Editor>,
    uid: u64,
    /// Asking "save changes?" after the X button.
    close_prompt: bool,
    bring_to_front: bool,
    /// Position/size inside the workspace.
    mdi: MdiState,
}

/// What to do after the "save changes?" question.
#[derive(Clone)]
enum Pending {
    New,
    Open(Option<PathBuf>),
    Exit,
}

pub struct IdeApp {
    project: Project,
    path: Option<PathBuf>,
    modified: bool,
    prefs: Prefs,
    tree: TreeState,
    forms: Vec<OpenForm>,
    next_uid: u64,
    tex: TextureCache,
    gen: u64,
    clipboard: Vec<Action>,
    names: ProjectNames,
    alloc: IdAlloc,
    messages: Vec<String>,
    pending: Option<Pending>,
    confirm_delete: Option<TreePath>,
    group_prompt: Option<(TreePath, String)>,
    runner: Option<Child>,
    show_prefs: bool,
    allow_close: bool,
    last_title: String,
    status: String,
    /// uid of the active (focused) form.
    active: Option<u64>,
    /// Workspace rectangle of the last frame (where forms live).
    workspace: egui::Rect,
    show_about: bool,
}

impl IdeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        crate::theme::apply(&cc.egui_ctx);
        let mut app = IdeApp {
            project: ops::new_project(),
            path: None,
            modified: false,
            prefs: Prefs::load(),
            tree: TreeState::default(),
            forms: Vec::new(),
            next_uid: 1,
            tex: TextureCache::default(),
            gen: 1,
            clipboard: Vec::new(),
            names: ProjectNames::default(),
            alloc: IdAlloc::default(),
            messages: Vec::new(),
            pending: None,
            confirm_delete: None,
            group_prompt: None,
            runner: None,
            show_prefs: false,
            allow_close: false,
            last_title: String::new(),
            status: String::new(),
            active: None,
            workspace: egui::Rect::from_min_size(egui::pos2(250.0, 60.0), Vec2::new(1000.0, 700.0)),
            show_about: false,
        };
        app.project_replaced();
        if let Some(p) = open {
            app.open_path(&p);
        }
        app
    }

    /// Reset per-project state after New/Open.
    fn project_replaced(&mut self) {
        self.forms.clear();
        self.tex.clear();
        self.gen += 1;
        self.names = ProjectNames::from_project(&self.project);
        self.alloc = IdAlloc::from_project(&self.project);
        self.tree = TreeState::default();
        self.modified = false;
    }

    fn any_dirty_form(&self) -> bool {
        self.forms.iter().any(|f| f.editor.is_dirty(&self.project))
    }

    fn needs_save_prompt(&self) -> bool {
        self.modified || self.any_dirty_form()
    }

    fn open_path(&mut self, p: &Path) {
        match ops::load_project(p) {
            Ok(proj) => {
                self.project = proj;
                self.path = Some(p.to_path_buf());
                self.project_replaced();
                self.prefs.add_recent(p.to_path_buf());
                self.prefs.save();
                self.status = format!("Opened {}", p.display());
            }
            Err(e) => self.messages.push(format!("Cannot open: {e}")),
        }
    }

    fn do_new(&mut self) {
        self.project = ops::new_project();
        self.path = None;
        self.project_replaced();
    }

    fn do_open_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("GameMaker projects", &["gm81", "gmk"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.open_path(&p);
        }
    }

    /// Save to the current path (or ask). Returns true on success.
    fn save(&mut self, save_as: bool) -> bool {
        let path = match (&self.path, save_as) {
            (Some(p), false)
                if p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("gm81")) =>
            {
                p.clone()
            }
            _ => {
                let mut d = rfd::FileDialog::new().add_filter("GameMaker 8.1 project", &["gm81"]);
                if let Some(p) = &self.path {
                    if let Some(stem) = p.file_stem() {
                        d = d.set_file_name(format!("{}.gm81", stem.to_string_lossy()));
                    }
                    if let Some(dir) = p.parent() {
                        d = d.set_directory(dir);
                    }
                }
                match d.save_file() {
                    Some(p) => p.with_extension("gm81"),
                    None => return false,
                }
            }
        };
        match ops::save_project(&self.project, &path) {
            Ok(()) => {
                self.modified = false;
                self.path = Some(path.clone());
                self.prefs.add_recent(path.clone());
                self.prefs.save();
                self.status = format!("Saved {}", path.display());
                true
            }
            Err(e) => {
                self.messages.push(format!("Cannot save: {e}"));
                false
            }
        }
    }

    fn commit_all_forms(&mut self) {
        let mut forms = std::mem::take(&mut self.forms);
        for f in &mut forms {
            self.commit_form(f.editor.as_mut());
        }
        self.forms = forms;
    }

    fn commit_form(&mut self, ed: &mut dyn Editor) {
        if ed.commit(&mut self.project) {
            self.modified = true;
            self.gen += 1;
            if let ResKey::Res(kind, i) = ed.key() {
                ops::sync_tree_name(&mut self.project, kind, i);
            }
            self.names = ProjectNames::from_project(&self.project);
        }
    }

    /// Run `p` now, or after asking to save.
    fn request(&mut self, p: Pending) {
        if self.needs_save_prompt() {
            self.pending = Some(p);
        } else {
            self.perform(p);
        }
    }

    fn perform(&mut self, p: Pending) {
        match p {
            Pending::New => self.do_new(),
            Pending::Open(Some(path)) => self.open_path(&path),
            Pending::Open(None) => self.do_open_dialog(),
            Pending::Exit => {
                self.allow_close = true;
            }
        }
    }

    pub(crate) fn open_form(&mut self, key: ResKey) {
        if let Some(f) = self.forms.iter_mut().find(|f| f.editor.key() == key) {
            f.bring_to_front = true;
            return;
        }
        let uid = self.next_uid;
        self.next_uid += 1;
        if let Some(editor) = open_editor(key, &self.project, uid) {
            let size = mdi::outer_size(editor.default_size());
            let rect = mdi::cascade_rect(self.workspace, self.forms.len(), size);
            self.forms.push(OpenForm {
                editor,
                uid,
                close_prompt: false,
                bring_to_front: true,
                mdi: MdiState::new(rect),
            });
        }
    }

    fn create(&mut self, kind: ResourceKind, at: Placement) {
        let (idx, path) = ops::create_resource(&mut self.project, kind, &at);
        self.modified = true;
        self.names = ProjectNames::from_project(&self.project);
        self.tree.selected = Some(path);
        self.open_form(ResKey::Res(kind, idx));
    }

    fn key_of_node(&self, path: &TreePath) -> Option<ResKey> {
        let n = ops::node(&self.project.resource_tree, path)?;
        if n.status != NodeStatus::Leaf {
            return None;
        }
        Some(match n.kind {
            ResourceKind::GameInfo => ResKey::GameInfo,
            ResourceKind::Settings => ResKey::Settings,
            ResourceKind::Extensions => ResKey::Extensions,
            k if ops::is_list_kind(k) && n.index >= 0 => ResKey::Res(k, n.index as usize),
            _ => return None,
        })
    }

    fn apply_tree(&mut self, a: TreeAction) {
        match a {
            TreeAction::Open(p) => {
                if let Some(k) = self.key_of_node(&p) {
                    self.open_form(k);
                }
            }
            TreeAction::Create(kind, at) => self.create(kind, at),
            TreeAction::Duplicate(p) => {
                if let Some(ResKey::Res(kind, i)) = self.key_of_node(&p) {
                    if let Some((ni, np)) = ops::duplicate_resource(&mut self.project, kind, i) {
                        self.modified = true;
                        self.tree.selected = Some(np);
                        self.alloc = IdAlloc::from_project(&self.project);
                        self.names = ProjectNames::from_project(&self.project);
                        self.open_form(ResKey::Res(kind, ni));
                    }
                }
            }
            TreeAction::Delete(p) => self.confirm_delete = Some(p),
            TreeAction::StartRename(p) => {
                if let Some(n) = ops::node(&self.project.resource_tree, &p) {
                    if p.len() > 1 {
                        self.tree.renaming = Some((p, n.name.clone()));
                    }
                }
            }
            TreeAction::FinishRename(p, name) => {
                self.tree.renaming = None;
                let name = name.trim().to_string();
                if name.is_empty() {
                    return;
                }
                match self.key_of_node(&p) {
                    Some(ResKey::Res(kind, i)) => {
                        if ops::res_name(&self.project, kind, i) != Some(name.as_str()) {
                            ops::rename_resource(&mut self.project, kind, i, &name);
                            self.modified = true;
                            self.names = ProjectNames::from_project(&self.project);
                            for f in &mut self.forms {
                                if f.editor.key() == ResKey::Res(kind, i) {
                                    f.editor.on_renamed(&name);
                                }
                            }
                        }
                    }
                    _ => {
                        if let Some(n) = ops::node_mut(&mut self.project.resource_tree, &p) {
                            if n.status == NodeStatus::Group && n.name != name {
                                n.name = name;
                                self.modified = true;
                            }
                        }
                    }
                }
            }
            TreeAction::AddGroup(p) => self.group_prompt = Some((p, "group".into())),
            TreeAction::Move {
                from,
                to_parent,
                pos,
            } => {
                if let Some(np) = ops::move_node(&mut self.project, &from, &to_parent, pos) {
                    self.modified = true;
                    self.tree.selected = Some(np);
                }
            }
        }
    }

    fn run_game(&mut self) {
        if self.runner.is_some() {
            self.messages.push("The game is already running.".into());
            return;
        }
        let rp =
            (!self.prefs.runner_path.trim().is_empty()).then(|| self.prefs.runner_path.clone());
        match crate::run::launch(&self.project, rp.as_deref(), self.path.as_deref()) {
            Ok(child) => {
                self.runner = Some(child);
                self.status = "Game running…".into();
            }
            Err(e) => self.messages.push(e),
        }
    }

    fn poll_runner(&mut self, ctx: &Context) {
        if let Some(c) = &mut self.runner {
            match c.try_wait() {
                Ok(Some(st)) => {
                    self.runner = None;
                    self.status = format!("Game finished ({st})");
                    if !st.success() {
                        self.messages.push(format!("The runner exited with {st}."));
                    }
                }
                Ok(None) => ctx.request_repaint_after(std::time::Duration::from_millis(300)),
                Err(e) => {
                    self.runner = None;
                    self.messages.push(e.to_string());
                }
            }
        }
    }

    fn title(&self) -> String {
        let name = self.path.as_ref().and_then(|p| p.file_name()).map_or_else(
            || "<new game>".to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        format!(
            "{name}{} - GM81 Clean IDE",
            if self.modified { " *" } else { "" }
        )
    }

    #[cfg(test)]
    pub(crate) fn test_set_project(&mut self, p: Project) {
        self.project = p;
        self.path = None;
        self.project_replaced();
    }

    #[cfg(test)]
    pub(crate) fn test_state(&self) -> (&Project, bool, usize, usize) {
        (
            &self.project,
            self.modified,
            self.forms.len(),
            self.messages.len(),
        )
    }

    /// Window ids of the open forms (for layout tests).
    #[cfg(test)]
    pub(crate) fn test_form_ids(&self) -> Vec<Id> {
        self.forms.iter().map(|f| Id::new(("form", f.uid))).collect()
    }

    #[cfg(test)]
    pub(crate) fn test_close_forms(&mut self) {
        self.forms.clear();
    }

    // ------------------------------------------------------------------ UI parts

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    self.request(Pending::New);
                    ui.close();
                }
                if ui.button("Open…").clicked() {
                    self.request(Pending::Open(None));
                    ui.close();
                }
                ui.menu_button("Recent Files", |ui| {
                    if self.prefs.recent_files.is_empty() {
                        ui.label("(none)");
                    }
                    for p in self.prefs.recent_files.clone() {
                        if ui.button(p.display().to_string()).clicked() {
                            self.request(Pending::Open(Some(p)));
                            ui.close();
                        }
                    }
                });
                if ui.button("Save").clicked() {
                    self.save(false);
                    ui.close();
                }
                if ui.button("Save As…").clicked() {
                    self.save(true);
                    ui.close();
                }
                if ui.button("Create Executable…").clicked() {
                    self.create_executable();
                    ui.close();
                }
                ui.separator();
                if ui.button("Preferences…").clicked() {
                    self.show_prefs = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    self.request(Pending::Exit);
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                let sel = self.tree.selected.clone();
                let key = sel.as_ref().and_then(|p| self.key_of_node(p));
                let is_res = matches!(key, Some(ResKey::Res(..)));
                let node = sel
                    .as_ref()
                    .and_then(|p| ops::node(&self.project.resource_tree, p))
                    .cloned();
                if ui
                    .add_enabled(is_res, egui::Button::new("Insert resource"))
                    .clicked()
                {
                    if let (Some(ResKey::Res(k, _)), Some(p)) = (key, sel.clone()) {
                        self.create(k, Placement::Before(p));
                    }
                    ui.close();
                }
                if ui
                    .add_enabled(is_res, egui::Button::new("Duplicate"))
                    .clicked()
                {
                    self.apply_tree(TreeAction::Duplicate(sel.clone().unwrap_or_default()));
                    ui.close();
                }
                let deletable = sel.as_ref().is_some_and(|p| p.len() > 1);
                if ui
                    .add_enabled(deletable, egui::Button::new("Delete"))
                    .clicked()
                {
                    self.confirm_delete = sel.clone();
                    ui.close();
                }
                if ui
                    .add_enabled(deletable, egui::Button::new("Rename"))
                    .clicked()
                {
                    self.apply_tree(TreeAction::StartRename(sel.clone().unwrap_or_default()));
                    ui.close();
                }
                if ui
                    .add_enabled(key.is_some(), egui::Button::new("Properties"))
                    .clicked()
                {
                    if let Some(k) = key {
                        self.open_form(k);
                    }
                    ui.close();
                }
                let can_group = node.as_ref().is_some_and(|n| ops::is_list_kind(n.kind));
                if ui
                    .add_enabled(can_group, egui::Button::new("Add Group"))
                    .clicked()
                {
                    if let (Some(p), Some(n)) = (sel.clone(), node) {
                        let target = if n.status == NodeStatus::Leaf {
                            p[..p.len() - 1].to_vec()
                        } else {
                            p
                        };
                        self.group_prompt = Some((target, "group".into()));
                    }
                    ui.close();
                }
                ui.separator();
                if ui.button("Expand Resource Tree").clicked() {
                    self.tree.expand_all = Some(true);
                    ui.close();
                }
                if ui.button("Collapse Resource Tree").clicked() {
                    self.tree.expand_all = Some(false);
                    ui.close();
                }
            });
            ui.menu_button("Resources", |ui| {
                for (k, label, _) in LIST_KINDS {
                    if ui.button(format!("Create {label}")).clicked() {
                        self.create(k, self.placement_for(k));
                        ui.close();
                    }
                }
                ui.separator();
                for (key, label) in [
                    (ResKey::GameInfo, "Change Game Information"),
                    (ResKey::Settings, "Change Global Game Settings"),
                    (ResKey::Extensions, "Select Extension Packages"),
                    (ResKey::Constants, "Define Constants"),
                    (ResKey::Triggers, "Define Triggers"),
                    (ResKey::IncludedFiles, "Included Files"),
                ] {
                    if ui.button(label).clicked() {
                        self.open_form(key);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Scripts", |ui| {
                if ui.button("Create Script").clicked() {
                    self.create(ResourceKind::Script, self.placement_for(ResourceKind::Script));
                    ui.close();
                }
                ui.separator();
                let scripts: Vec<(usize, String)> = self
                    .project
                    .scripts
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| Some((i, s.as_ref()?.name.clone())))
                    .collect();
                if scripts.is_empty() {
                    ui.add_enabled(false, egui::Button::new("(no scripts)"));
                }
                egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for (i, name) in scripts {
                        if ui.button(name).clicked() {
                            self.open_form(ResKey::Res(ResourceKind::Script, i));
                            ui.close();
                        }
                    }
                });
            });
            ui.menu_button("Run", |ui| {
                if ui.button("Run normally (F5)").clicked() {
                    self.run_game();
                    ui.close();
                }
                ui.add_enabled(false, egui::Button::new("Run in Debug mode (F6)"));
            });
            ui.menu_button("Window", |ui| self.window_menu(ui));
            ui.menu_button("Help", |ui| {
                if ui.button("About gm81-clean…").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    fn window_menu(&mut self, ui: &mut egui::Ui) {
        let ws = self.workspace;
        let ctx = ui.ctx().clone();
        let any = !self.forms.is_empty();
        if ui.add_enabled(any, egui::Button::new("Cascade")).clicked() {
            for (i, f) in self.forms.iter_mut().enumerate() {
                let size = mdi::outer_size(f.editor.default_size());
                f.mdi = MdiState::new(mdi::cascade_rect(ws, i, size));
                mdi::bring_to_front(&ctx, Id::new(("form", f.uid)));
            }
            ui.close();
        }
        if ui.add_enabled(any, egui::Button::new("Arrange Tiled")).clicked() {
            let n = self.forms.len().max(1);
            let cols = (n as f32).sqrt().ceil() as usize;
            let rows = n.div_ceil(cols);
            let w = ws.width() / cols as f32;
            let h = ws.height() / rows as f32;
            for (i, f) in self.forms.iter_mut().enumerate() {
                let (c, r) = (i % cols, i / cols);
                f.mdi = MdiState::new(egui::Rect::from_min_size(
                    ws.min + Vec2::new(c as f32 * w, r as f32 * h),
                    Vec2::new(w, h),
                ));
            }
            ui.close();
        }
        if ui.add_enabled(any, egui::Button::new("Minimise All")).clicked() {
            for f in &mut self.forms {
                f.mdi.minimised = true;
            }
            ui.close();
        }
        if ui.add_enabled(any, egui::Button::new("Close All")).clicked() {
            for f in &mut self.forms {
                f.close_prompt = true;
            }
            ui.close();
        }
        if any {
            ui.separator();
        }
        let mut pick = None;
        for (i, f) in self.forms.iter().enumerate() {
            let label = format!("{} {}", i + 1, f.editor.title());
            if ui
                .add(egui::Button::selectable(self.active == Some(f.uid), label))
                .clicked()
            {
                pick = Some(i);
                ui.close();
            }
        }
        if let Some(i) = pick {
            self.forms[i].bring_to_front = true;
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let cell = |ui: &mut egui::Ui, w: f32, text: &str| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, 18.0), egui::Sense::hover());
            let p = ui.painter();
            p.hline(r.x_range(), r.top() + 0.5, egui::Stroke::new(1.0, theme::SHADOW));
            p.vline(r.left() + 0.5, r.y_range(), egui::Stroke::new(1.0, theme::SHADOW));
            p.hline(r.x_range(), r.bottom() - 0.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
            p.vline(r.right() - 0.5, r.y_range(), egui::Stroke::new(1.0, egui::Color32::WHITE));
            p.with_clip_rect(r.shrink(2.0)).text(
                r.left_center() + Vec2::new(5.0, 0.0),
                egui::Align2::LEFT_CENTER,
                text,
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::from_gray(20),
            );
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let total = ui.available_width();
            let file = self
                .path
                .as_ref()
                .map_or_else(|| "<new game>".to_string(), |p| p.display().to_string());
            let right = 120.0;
            let mid = (total * 0.45).min(520.0);
            let left = (total - mid - right - 4.0).max(40.0);
            let status = if self.status.is_empty() { "Ready" } else { &self.status };
            cell(ui, left, status);
            cell(ui, mid, &file);
            let n = self.forms.len();
            cell(ui, right, &format!("{n} window{} open", if n == 1 { "" } else { "s" }));
        });
    }

    /// File > Create Executable: runner + project payload + footer (specs/exe-format.md).
    fn create_executable(&mut self) {
        let rp =
            (!self.prefs.runner_path.trim().is_empty()).then(|| self.prefs.runner_path.clone());
        let Some(runner) = crate::run::find_runner(rp.as_deref()) else {
            self.messages.push(format!(
                "Runner executable '{}' not found next to the IDE or in a cargo target \
                 directory. Set its path in File > Preferences.",
                crate::run::runner_file_name()
            ));
            return;
        };
        self.commit_all_forms();
        let stem = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| "game".to_string(), |s| s.to_string_lossy().into_owned());
        let mut d = rfd::FileDialog::new()
            .add_filter("Executable", &[if cfg!(windows) { "exe" } else { "bin" }])
            .set_file_name(if cfg!(windows) { format!("{stem}.exe") } else { stem });
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            d = d.set_directory(dir);
        }
        let Some(out) = d.save_file() else { return };
        match crate::exe::create_executable(&self.project, &runner, &out, self.path.as_deref()) {
            Ok(warnings) => {
                self.status = format!("Created {}", out.display());
                self.messages.extend(warnings);
            }
            Err(e) => self.messages.push(format!("Cannot create the executable: {e}")),
        }
    }

    /// Placement for Resources > Create: into the selected group of that kind, else end.
    fn placement_for(&self, kind: ResourceKind) -> Placement {
        if let Some(p) = &self.tree.selected {
            if let Some(n) = ops::node(&self.project.resource_tree, p) {
                if n.kind == kind && n.status == NodeStatus::Group {
                    return Placement::Into(p.clone());
                }
            }
        }
        Placement::End
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            let tb = icons::tool_button;
            if tb(ui, Icon::New, "Start a new game (Ctrl+N)", true).clicked() {
                self.request(Pending::New);
            }
            if tb(ui, Icon::Open, "Open a game (Ctrl+O)", true).clicked() {
                self.request(Pending::Open(None));
            }
            if tb(ui, Icon::Save, "Save the game (Ctrl+S)", true).clicked() {
                self.save(false);
            }
            icons::tool_separator(ui);
            if tb(ui, Icon::CreateExe, "Create a stand-alone executable for the game", true)
                .clicked()
            {
                self.create_executable();
            }
            tb(ui, Icon::Publish, "Publish your game (not available)", false);
            icons::tool_separator(ui);
            let running = self.runner.is_some();
            if tb(ui, Icon::Run, "Run the game (F5)", !running).clicked() {
                self.run_game();
            }
            tb(ui, Icon::Debug, "Run the game in debug mode (not available)", false);
            icons::tool_separator(ui);
            for (k, label, _) in LIST_KINDS {
                let article = if label.starts_with(['A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
                if tb(ui, Icon::of_kind(k), &format!("Create {article} {}", label.to_lowercase()), true)
                    .clicked()
                {
                    self.create(k, self.placement_for(k));
                }
            }
            icons::tool_separator(ui);
            if tb(ui, Icon::GameInfo, "Change the game information", true).clicked() {
                self.open_form(ResKey::GameInfo);
            }
            if tb(ui, Icon::Settings, "Change the global game settings", true).clicked() {
                self.open_form(ResKey::Settings);
            }
            if tb(ui, Icon::Extensions, "Select the extension packages", true).clicked() {
                self.open_form(ResKey::Extensions);
            }
            icons::tool_separator(ui);
            if tb(ui, Icon::Help, "About gm81-clean", true).clicked() {
                self.show_about = true;
            }
        });
    }

    fn forms_ui(&mut self, ctx: &Context, area: egui::Rect) {
        let mut forms = std::mem::take(&mut self.forms);
        let mut requests: Vec<Request> = Vec::new();
        let mut to_commit: Vec<usize> = Vec::new();
        let mut to_close: Vec<usize> = Vec::new();
        self.workspace = area;
        let mut n_min = 0;
        for (fi, f) in forms.iter_mut().enumerate() {
            let wid = Id::new(("form", f.uid));
            if f.bring_to_front {
                f.mdi.minimised = false;
                mdi::bring_to_front(ctx, wid);
                self.active = Some(f.uid);
                f.bring_to_front = false;
            }
            let slot = mdi::minimised_slot(area, n_min);
            if f.mdi.minimised {
                n_min += 1;
            }
            let dirty = f.editor.is_dirty(&self.project);
            let title = format!("{}{}", f.editor.title(), if dirty { " *" } else { "" });
            let icon = form_icon(f.editor.key());
            let active = self.active == Some(f.uid);
            let mut ok = false;
            let mut cancel = false;
            let out = {
                let mut cx = Cx {
                    ctx: ctx.clone(),
                    project: &self.project,
                    tex: &mut self.tex,
                    gen: self.gen,
                    clipboard: &mut self.clipboard,
                    requests: &mut requests,
                    names: &self.names,
                    prefs: &self.prefs,
                    alloc: &mut self.alloc,
                };
                mdi::show(ctx, wid, area, slot, &mut f.mdi, &title, icon, active, |ui| {
                    ui.horizontal(|ui| {
                        if ok_button(ui).clicked() {
                            ok = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                    ui.separator();
                    f.editor.ui(ui, &mut cx);
                })
            };
            if out.activated {
                self.active = Some(f.uid);
                mdi::bring_to_front(ctx, wid);
            }
            let open = !out.close;
            if ok {
                to_commit.push(fi);
                to_close.push(fi);
            } else if cancel {
                to_close.push(fi);
            } else if !open {
                if dirty {
                    f.close_prompt = true;
                } else {
                    to_close.push(fi);
                }
            }
            if f.close_prompt && !to_close.contains(&fi) {
                if !f.editor.is_dirty(&self.project) {
                    to_close.push(fi);
                    continue;
                }
                egui::Modal::new(wid.with("close")).show(ctx, |ui| {
                    ui.label(format!("Save changes to \"{}\"?", f.editor.title()));
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            to_commit.push(fi);
                            to_close.push(fi);
                        }
                        if ui.button("No").clicked() {
                            to_close.push(fi);
                        }
                        if ui.button("Cancel").clicked() {
                            f.close_prompt = false;
                        }
                    });
                });
            }
        }
        for &i in &to_commit {
            let ed = forms[i].editor.as_mut();
            if ed.commit(&mut self.project) {
                self.modified = true;
                self.gen += 1;
                if let ResKey::Res(kind, idx) = ed.key() {
                    ops::sync_tree_name(&mut self.project, kind, idx);
                }
                self.names = ProjectNames::from_project(&self.project);
            }
        }
        to_close.sort_unstable();
        to_close.dedup();
        for &i in to_close.iter().rev() {
            forms.remove(i);
        }
        // Keep forms opened during this frame (by requests below) after the old ones.
        self.forms.splice(0..0, forms);
        for r in requests {
            match r {
                Request::Open(k) => self.open_form(k),
                Request::Message(m) => self.messages.push(m),
            }
        }
    }

    fn dialogs(&mut self, ctx: &Context) {
        if let Some(m) = self.messages.first().cloned() {
            egui::Modal::new(Id::new("message")).show(ctx, |ui| {
                ui.set_max_width(420.0);
                ui.label(m);
                if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.messages.remove(0);
                }
            });
            return;
        }
        if let Some(p) = self.pending.clone() {
            let mut choice = None;
            egui::Modal::new(Id::new("save_prompt")).show(ctx, |ui| {
                ui.label("The game has been changed. Save the changes?");
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        choice = Some(0);
                    }
                    if ui.button("No").clicked() {
                        choice = Some(1);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(2);
                    }
                });
            });
            match choice {
                Some(0) => {
                    self.pending = None;
                    self.commit_all_forms();
                    if self.save(false) {
                        self.perform(p);
                    }
                }
                Some(1) => {
                    self.pending = None;
                    self.perform(p);
                }
                Some(_) => self.pending = None,
                None => {}
            }
        }
        if let Some(path) = self.confirm_delete.clone() {
            let name = ops::node(&self.project.resource_tree, &path).map(|n| n.name.clone());
            let mut choice = None;
            egui::Modal::new(Id::new("confirm_delete")).show(ctx, |ui| {
                ui.label(format!(
                    "Are you sure you want to delete \"{}\"? This cannot be undone.",
                    name.as_deref().unwrap_or("?")
                ));
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        choice = Some(true);
                    }
                    if ui.button("No").clicked() {
                        choice = Some(false);
                    }
                });
            });
            if let Some(c) = choice {
                self.confirm_delete = None;
                if c {
                    let deleted = ops::delete_node(&mut self.project, &path);
                    self.forms.retain(|f| match f.editor.key() {
                        ResKey::Res(k, i) => !deleted.contains(&(k, i)),
                        _ => true,
                    });
                    self.tree.selected = None;
                    self.modified = true;
                    self.names = ProjectNames::from_project(&self.project);
                }
            }
        }
        if let Some((path, mut name)) = self.group_prompt.take() {
            let mut done = None;
            egui::Modal::new(Id::new("group_prompt")).show(ctx, |ui| {
                ui.label("Name of the new group:");
                let r = ui.text_edit_singleline(&mut name);
                r.request_focus();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        done = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        done = Some(false);
                    }
                });
            });
            match done {
                Some(true) => {
                    if let Some(np) = ops::add_group(&mut self.project, &path, name.trim()) {
                        self.modified = true;
                        self.tree.selected = Some(np);
                    }
                }
                Some(false) => {}
                None => self.group_prompt = Some((path, name)),
            }
        }
        if self.show_about {
            egui::Modal::new(Id::new("about")).show(ctx, |ui| {
                ui.set_width(360.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(360.0, 56.0), egui::Sense::hover());
                paint_logo(ui.painter(), r);
                ui.label("A clean-room, GameMaker 8.1 compatible game development environment.");
                ui.label(egui::RichText::new("Not affiliated with or endorsed by YoYo Games.").small().weak());
                ui.add_space(6.0);
                if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.show_about = false;
                }
            });
        }
        if self.show_prefs {
            let mut open = true;
            egui::Window::new("Preferences").open(&mut open).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Runner executable:");
                    ui.text_edit_singleline(&mut self.prefs.runner_path);
                    if ui.button("…").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_file() {
                            self.prefs.runner_path = p.display().to_string();
                        }
                    }
                });
                ui.label(
                    egui::RichText::new(format!(
                        "Empty = auto-detect ({} next to the IDE or in the cargo target dir). Found: {}",
                        crate::run::runner_file_name(),
                        crate::run::find_runner(Some(&self.prefs.runner_path))
                            .or_else(|| crate::run::find_runner(None))
                            .map_or("no".into(), |p| p.display().to_string())
                    ))
                    .small(),
                );
                ui.separator();
                ui.add(egui::Slider::new(&mut self.prefs.code_font_size, 8.0..=28.0).text("Code font size"));
                ui.checkbox(&mut self.prefs.show_line_numbers, "Show line numbers");
                ui.checkbox(&mut self.prefs.syntax_colouring, "Use color coding");
                ui.checkbox(&mut self.prefs.show_origin, "Show the origin in sprite images");
                if ui.button("Save preferences").clicked() {
                    self.prefs.save();
                }
            });
            if !open {
                self.prefs.save();
                self.show_prefs = false;
            }
        }
    }

    fn shortcuts(&mut self, ctx: &Context) {
        use egui::{Key, Modifiers};
        let (s, o, n, f5) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::S),
                i.consume_key(Modifiers::COMMAND, Key::O),
                i.consume_key(Modifiers::COMMAND, Key::N),
                i.consume_key(Modifiers::NONE, Key::F5),
            )
        });
        if s {
            self.save(false);
        }
        if o {
            self.request(Pending::Open(None));
        }
        if n {
            self.request(Pending::New);
        }
        if f5 {
            self.run_game();
        }
        // Dropped files: projects open, images become sprites.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        for p in dropped {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "gm81" || ext == "gmk" {
                self.request(Pending::Open(Some(p)));
            } else if ["png", "bmp", "gif", "jpg", "jpeg"].contains(&ext.as_str()) {
                match crate::imageio::load_frames(
                    std::slice::from_ref(&p),
                    crate::imageio::LoadOptions::SPRITE,
                ) {
                    Ok(frames) => {
                        let (idx, _) = ops::create_resource(
                            &mut self.project,
                            ResourceKind::Sprite,
                            &Placement::End,
                        );
                        if let Some(Some(s)) = self.project.sprites.get_mut(idx) {
                            s.frames = frames;
                            crate::editors::sprite::update_bbox(s);
                            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                let name = stem.split("_strip").next().unwrap_or(stem).to_string();
                                if !ops::name_in_use(&self.project, &name) {
                                    ops::rename_resource(
                                        &mut self.project,
                                        ResourceKind::Sprite,
                                        idx,
                                        &name,
                                    );
                                }
                            }
                        }
                        self.modified = true;
                        self.gen += 1;
                        self.names = ProjectNames::from_project(&self.project);
                        self.open_form(ResKey::Res(ResourceKind::Sprite, idx));
                    }
                    Err(e) => self.messages.push(e),
                }
            }
        }
    }
}

impl eframe::App for IdeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_runner(&ctx);
        // Close requests (window X) ask to save first.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.needs_save_prompt()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Exit);
        }
        if self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let title = self.title();
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        if self.messages.is_empty() && self.pending.is_none() {
            self.shortcuts(&ctx);
        }

        let chrome = egui::Frame::new()
            .fill(theme::PANEL)
            .inner_margin(egui::Margin::symmetric(2, 1));
        egui::Panel::top("menu")
            .frame(chrome)
            .show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("toolbar")
            .frame(chrome.inner_margin(egui::Margin {
                left: 4,
                right: 4,
                top: 2,
                bottom: 3,
            }))
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().hline(r.x_range(), r.top() - 1.5, egui::Stroke::new(1.0, theme::SHADOW));
                ui.painter().hline(r.x_range(), r.top() - 0.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
                self.toolbar(ui);
            });
        egui::Panel::bottom("status")
            .frame(chrome.inner_margin(egui::Margin::symmetric(2, 2)))
            .show(ui, |ui| self.status_bar(ui));
        let actions = egui::Panel::left("tree")
            .resizable(true)
            .default_size(230.0)
            .size_range(140.0..=600.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::LIST_BG)
                    .stroke(egui::Stroke::new(1.0, theme::SHADOW)),
            )
            .show(ui, |ui| {
                logo(ui);
                tree_ui(ui, &self.project, &mut self.tree, &mut self.tex, self.gen)
            })
            .inner;
        for a in actions {
            self.apply_tree(a);
        }
        let area = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::WORKSPACE))
            .show(ui, |ui| {
                let r = ui.available_rect_before_wrap();
                // sunken edge of the workspace
                let p = ui.painter();
                p.hline(r.x_range(), r.top() + 0.5, egui::Stroke::new(1.0, egui::Color32::from_gray(110)));
                p.vline(r.left() + 0.5, r.y_range(), egui::Stroke::new(1.0, egui::Color32::from_gray(110)));
                ui.allocate_rect(r, egui::Sense::hover());
                r.shrink(1.0)
            })
            .inner;
        self.forms_ui(&ctx, area);
        self.dialogs(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.prefs.save();
        crate::run::cleanup();
    }
}

/// Title-bar icon of a form.
fn form_icon(key: ResKey) -> Icon {
    match key {
        ResKey::Res(kind, _) => Icon::of_kind(kind),
        ResKey::GameInfo => Icon::GameInfo,
        ResKey::Settings => Icon::Settings,
        ResKey::Extensions => Icon::Extensions,
        ResKey::Constants | ResKey::Triggers => Icon::Constants,
        ResKey::IncludedFiles => Icon::Extensions,
    }
}

/// The form's "OK" button with a green check mark.
fn ok_button(ui: &mut egui::Ui) -> egui::Response {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::TextStyle::Button.resolve(ui.style());
    job.append(
        "✔",
        0.0,
        egui::TextFormat::simple(font.clone(), egui::Color32::from_rgb(20, 140, 40)),
    );
    job.append(" OK", 0.0, egui::TextFormat::simple(font, egui::Color32::from_gray(10)));
    ui.add(egui::Button::new(job).min_size(Vec2::new(64.0, 22.0)))
}

/// Wordmark banner at the top of the resource panel.
fn logo(ui: &mut egui::Ui) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 44.0), egui::Sense::hover());
    paint_logo(ui.painter(), r);
}

fn paint_logo(p: &egui::Painter, r: egui::Rect) {
    use egui::{Color32, Mesh, Shape};
    let mut mesh = Mesh::default();
    let (a, b) = (Color32::from_rgb(255, 255, 255), Color32::from_rgb(214, 228, 245));
    mesh.colored_vertex(r.left_top(), a);
    mesh.colored_vertex(r.right_top(), b);
    mesh.colored_vertex(r.right_bottom(), b);
    mesh.colored_vertex(r.left_bottom(), a);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(Shape::mesh(mesh));
    p.hline(r.x_range(), r.bottom() - 0.5, egui::Stroke::new(1.0, theme::SHADOW));
    let icon = egui::Rect::from_min_size(r.min + Vec2::new(8.0, (r.height() - 28.0) / 2.0), Vec2::splat(28.0));
    icons::paint(p, icon, Icon::Sprite, true);
    let x = icon.max.x + 8.0;
    p.text(
        egui::pos2(x, r.center().y - 1.0),
        egui::Align2::LEFT_BOTTOM,
        "gm81-clean",
        egui::FontId::proportional(19.0),
        Color32::from_rgb(16, 48, 120),
    );
    p.text(
        egui::pos2(x + 1.0, r.center().y + 1.0),
        egui::Align2::LEFT_TOP,
        "game development environment",
        egui::FontId::proportional(10.5),
        Color32::from_gray(90),
    );
}
