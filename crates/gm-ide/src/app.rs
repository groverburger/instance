//! Main window (specs/ide.md §1, §3): menus, toolbar, resource tree, form windows,
//! save prompts and running the game.

use std::path::{Path, PathBuf};
use std::process::Child;

use eframe::egui::{self, Context, Id, Vec2};
use gm_format::{Action, NodeStatus, Project, ResourceKind};

use crate::editors::{open_editor, Cx, Editor, IdAlloc, Request, ResKey};
use crate::prefs::Prefs;
use crate::project_ops::{self as ops, Placement, TreePath, LIST_KINDS};
use crate::tree::{tree_ui, TreeAction, TreeState};
use crate::widgets::code_editor::ProjectNames;
use crate::widgets::TextureCache;

struct OpenForm {
    editor: Box<dyn Editor>,
    uid: u64,
    /// Asking "save changes?" after the X button.
    close_prompt: bool,
    bring_to_front: bool,
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
}

impl IdeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
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
            self.forms.push(OpenForm {
                editor,
                uid,
                close_prompt: false,
                bring_to_front: true,
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
            ui.menu_button("Run", |ui| {
                if ui.button("Run normally (F5)").clicked() {
                    self.run_game();
                    ui.close();
                }
            });
            ui.menu_button("Window", |ui| {
                if ui.button("Close All").clicked() {
                    for f in &mut self.forms {
                        f.close_prompt = true;
                    }
                    ui.close();
                }
            });
            ui.menu_button("Help", |ui| {
                ui.label("GM81 Clean IDE — clean-room GameMaker 8.1 compatible editor.");
            });
        });
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
        ui.horizontal_wrapped(|ui| {
            if ui.button("🗋 New").on_hover_text("New game").clicked() {
                self.request(Pending::New);
            }
            if ui.button("🗁 Open").on_hover_text("Open a game").clicked() {
                self.request(Pending::Open(None));
            }
            if ui
                .button("💾 Save")
                .on_hover_text("Save the game")
                .clicked()
            {
                self.save(false);
            }
            ui.separator();
            let running = self.runner.is_some();
            if ui
                .add_enabled(!running, egui::Button::new("▶ Run"))
                .on_hover_text("Run the game")
                .clicked()
            {
                self.run_game();
            }
            ui.separator();
            for (k, label, _) in LIST_KINDS {
                if ui
                    .small_button(format!("+{label}"))
                    .on_hover_text(format!("Create a {label}"))
                    .clicked()
                {
                    self.create(k, self.placement_for(k));
                }
            }
            ui.separator();
            if ui.small_button("Game Info").clicked() {
                self.open_form(ResKey::GameInfo);
            }
            if ui.small_button("Settings").clicked() {
                self.open_form(ResKey::Settings);
            }
        });
    }

    fn forms_ui(&mut self, ctx: &Context, area: egui::Rect) {
        let mut forms = std::mem::take(&mut self.forms);
        let mut requests: Vec<Request> = Vec::new();
        let mut to_commit: Vec<usize> = Vec::new();
        let mut to_close: Vec<usize> = Vec::new();
        for (fi, f) in forms.iter_mut().enumerate() {
            let wid = Id::new(("form", f.uid));
            if f.bring_to_front {
                ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, wid));
                f.bring_to_front = false;
            }
            let mut open = true;
            let dirty = f.editor.is_dirty(&self.project);
            let title = format!("{}{}", f.editor.title(), if dirty { " *" } else { "" });
            let size = f.editor.default_size();
            let mut ok = false;
            let mut cancel = false;
            {
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
                egui::Window::new(title)
                    .id(wid)
                    .open(&mut open)
                    .default_size(size.min(area.size()))
                    .default_pos(area.min + Vec2::splat(20.0 + 24.0 * (fi % 8) as f32))
                    .constrain_to(area)
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("✔ OK").clicked() {
                                ok = true;
                            }
                            if ui.button("Cancel").clicked() {
                                cancel = true;
                            }
                        });
                        ui.separator();
                        f.editor.ui(ui, &mut cx);
                    });
            }
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

        egui::Panel::top("menu").show(ui, |ui| {
            self.menu_bar(ui);
            self.toolbar(ui);
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} forms open", self.forms.len()));
                });
            });
        });
        let actions = egui::Panel::left("tree")
            .resizable(true)
            .default_size(240.0)
            .show(ui, |ui| tree_ui(ui, &self.project, &mut self.tree))
            .inner;
        for a in actions {
            self.apply_tree(a);
        }
        let area = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_gray(150)))
            .show(ui, |ui| {
                let r = ui.available_rect_before_wrap();
                ui.allocate_rect(r, egui::Sense::hover());
                r
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
