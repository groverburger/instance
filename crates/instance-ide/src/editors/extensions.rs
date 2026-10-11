//! Extension Packages dialog (specs/ide.md §18.4, specs/extensions.md §2).
//!
//! Two lists like GameMaker 8.1: the packages the project uses (in order) and the
//! installed packages it does not use. Installed packages are found on the extension
//! search path ([`crate::exe::extension_dirs`], first hit wins by package name) as
//! `<name>.ged` + `<name>.dat` (+ `<name>.chm`). Used packages that are not installed
//! stay in the list, marked as missing. Install copies a `.ged` set into the user's
//! extension folder; Uninstall removes one from there (both act on disk at once,
//! independent of OK / Cancel, which only concern the project's package list).

use std::path::{Path, PathBuf};

use eframe::egui::{self, pos2, Color32, Sense, Ui, UiBuilder, Vec2};
use instance_format::extpkg::{self, ExtPackage};
use instance_format::Project;

use super::{Cx, Editor, Request, ResKey};
use crate::dialogs::{self, Picked};
use crate::theme;

/// One installed package found on the search path.
#[derive(Debug, Clone)]
pub struct Installed {
    /// Package name: the `.ged` file name without extension (what projects store and
    /// what the runner and Create Executable look up).
    pub name: String,
    /// Folder holding the files.
    pub dir: PathBuf,
    /// The parsed `.ged`, or why it could not be read.
    pub pkg: Result<ExtPackage, String>,
    /// Whether `<name>.dat` exists next to the `.ged`.
    pub has_dat: bool,
    /// The help file, if present.
    pub chm: Option<PathBuf>,
}

/// The installed packages in `dirs` (search order; the first folder holding a name
/// wins), sorted by name (case-insensitively).
pub fn scan(dirs: &[PathBuf]) -> Vec<Installed> {
    let mut out: Vec<Installed> = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut names: Vec<String> = rd
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                let ext = p.extension()?.to_str()?;
                if !ext.eq_ignore_ascii_case("ged") || !p.is_file() {
                    return None;
                }
                Some(p.file_stem()?.to_str()?.to_string())
            })
            .collect();
        names.sort();
        for name in names {
            if out.iter().any(|i| i.name == name) {
                continue;
            }
            let pkg = std::fs::read(dir.join(format!("{name}.ged")))
                .map_err(|e| e.to_string())
                .and_then(|b| extpkg::read_ged(&b).map_err(|e| e.to_string()));
            let chm = help_path(dir, &name, pkg.as_ref().ok());
            out.push(Installed {
                has_dat: dir.join(format!("{name}.dat")).is_file(),
                name,
                dir: dir.clone(),
                pkg,
                chm,
            });
        }
    }
    out.sort_by_key(|i| i.name.to_lowercase());
    out
}

/// `<name>.chm` in `dir`, else the package's own help file name there.
fn help_path(dir: &Path, name: &str, pkg: Option<&ExtPackage>) -> Option<PathBuf> {
    let own = dir.join(format!("{name}.chm"));
    if own.is_file() {
        return Some(own);
    }
    let hf = pkg.map(|p| p.help_file.trim()).filter(|h| !h.is_empty())?;
    let file = Path::new(hf).file_name()?;
    let p = dir.join(file);
    p.is_file().then_some(p)
}

/// The user's extension folder, where Install puts packages.
pub fn user_dir() -> Option<PathBuf> {
    instance_brand::appdirs::data_dir().map(|d| d.join("extensions"))
}

/// Whether `s` can be used as a file name on every desktop system.
fn valid_file_name(s: &str) -> bool {
    !s.is_empty()
        && !s.ends_with(['.', ' '])
        && !s.starts_with(' ')
        && !s
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
}

fn ext_is(name: &str, ext: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

fn stem(name: &str) -> &str {
    Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
}

/// The bytes of the companion `<stem>.<ext>` of `ged`: picked together with it, or
/// (natively) next to it on disk.
fn companion(files: &[Picked], ged: &Picked, ext: &str) -> Option<Vec<u8>> {
    let st = stem(&ged.name);
    if let Some(f) = files
        .iter()
        .find(|f| ext_is(&f.name, ext) && stem(&f.name).eq_ignore_ascii_case(st))
    {
        return Some(f.bytes.clone());
    }
    let p = ged.path.as_ref()?.with_extension(ext);
    std::fs::read(p).ok()
}

/// Installs the package among `files` (a `.ged`, plus its `.dat` and optional `.chm`
/// when they were picked too; otherwise they are taken from beside the `.ged`) into
/// `dest`, named after the package. Returns the package name.
pub fn install_picked(files: &[Picked], dest: &Path) -> Result<String, String> {
    let Some(ged) = files.iter().find(|f| ext_is(&f.name, "ged")) else {
        if files.iter().any(|f| ext_is(&f.name, "gex")) {
            return Err("Packages in the .gex distribution format cannot be installed \
                        yet: its layout is unknown. Install the package's .ged and .dat \
                        files instead (for example from a GameMaker extensions folder)."
                .into());
        }
        return Err("Choose a package description file (.ged).".into());
    };
    let pkg = extpkg::read_ged(&ged.bytes)
        .map_err(|e| format!("{} is not a valid package description: {e}", ged.name))?;
    let st = stem(&ged.name);
    let dat = companion(files, ged, "dat").ok_or_else(|| {
        format!("The package data file \"{st}.dat\" was not found next to \"{}\".", ged.name)
    })?;
    extpkg::read_dat(&dat, pkg.files.len())
        .map_err(|e| format!("The package data file \"{st}.dat\" is damaged: {e}"))?;
    let chm = companion(files, ged, "chm").or_else(|| {
        let hf = Path::new(pkg.help_file.trim()).file_name()?;
        std::fs::read(ged.path.as_ref()?.parent()?.join(hf)).ok()
    });
    let name = if valid_file_name(&pkg.name) {
        pkg.name.clone()
    } else {
        st.to_string()
    };
    let err = |e: std::io::Error| format!("{}: {e}", dest.display());
    std::fs::create_dir_all(dest).map_err(err)?;
    std::fs::write(dest.join(format!("{name}.dat")), &dat).map_err(err)?;
    let chm_path = dest.join(format!("{name}.chm"));
    match chm {
        Some(c) => std::fs::write(&chm_path, c).map_err(err)?,
        None => {
            let _ = std::fs::remove_file(&chm_path);
        }
    }
    // the .ged last: a package only counts as installed once it is complete
    std::fs::write(dest.join(format!("{name}.ged")), &ged.bytes).map_err(err)?;
    Ok(name)
}

/// Removes the files of package `name` from `dir`.
pub fn uninstall(name: &str, dir: &Path) -> Result<(), String> {
    let ged = dir.join(format!("{name}.ged"));
    std::fs::remove_file(&ged).map_err(|e| format!("{}: {e}", ged.display()))?;
    for ext in ["dat", "chm"] {
        let _ = std::fs::remove_file(dir.join(format!("{name}.{ext}")));
    }
    Ok(())
}

/// Readable signature of an extension function: `name(string, real)`.
pub fn signature(f: &extpkg::ExtFunction) -> String {
    if f.argument_count < 0 {
        return format!("{}(...)", f.name);
    }
    let args: Vec<&str> = f
        .arg_types
        .iter()
        .take(f.argument_count.clamp(0, 17) as usize)
        .map(|&t| if t == extpkg::ARG_STRING { "string" } else { "real" })
        .collect();
    format!("{}({})", f.name, args.join(", "))
}

#[derive(Debug, Clone, PartialEq)]
enum Sel {
    Used(usize),
    Available(String),
}

pub struct ExtensionsEditor {
    work: Vec<String>,
    sel: Option<Sel>,
    /// Search path the installed list was scanned from (rescanned when it changes).
    dirs: Option<Vec<PathBuf>>,
    installed: Vec<Installed>,
    /// The user's extension folder (Install target; Uninstall only works there).
    user_dir: Option<PathBuf>,
    /// Package awaiting the Uninstall confirmation.
    confirm_uninstall: Option<Installed>,
}

impl ExtensionsEditor {
    pub fn new(p: &Project) -> Self {
        ExtensionsEditor {
            work: p.extension_packages.clone(),
            sel: None,
            dirs: None,
            installed: Vec::new(),
            user_dir: None,
            confirm_uninstall: None,
        }
    }

    fn refresh(&mut self, project_path: Option<&Path>) {
        let dirs = crate::exe::extension_dirs(project_path);
        if self.dirs.as_ref() != Some(&dirs) {
            self.installed = scan(&dirs);
            self.user_dir = user_dir();
            self.dirs = Some(dirs);
        }
    }

    fn rescan(&mut self) {
        self.dirs = None;
    }

    fn installed(&self, name: &str) -> Option<&Installed> {
        self.installed.iter().find(|i| i.name == name)
    }

    /// Installed packages the project does not use (hidden packages are not offered).
    fn available(&self) -> Vec<&Installed> {
        self.installed
            .iter()
            .filter(|i| !self.work.contains(&i.name))
            .filter(|i| !i.pkg.as_ref().is_ok_and(|p| p.hidden))
            .collect()
    }

    fn selected_name(&self) -> Option<&str> {
        match self.sel.as_ref()? {
            Sel::Used(i) => self.work.get(*i).map(String::as_str),
            Sel::Available(n) => Some(n),
        }
    }

    fn use_package(&mut self, name: &str) {
        if !self.work.iter().any(|n| n == name) {
            self.work.push(name.to_string());
        }
        self.sel = self.work.iter().position(|n| n == name).map(Sel::Used);
    }

    fn stop_using(&mut self, i: usize) {
        if i < self.work.len() {
            let name = self.work.remove(i);
            self.sel = if self.installed(&name).is_some() {
                Some(Sel::Available(name))
            } else {
                None
            };
        }
    }

    fn handle_install(&mut self, cx: &mut Cx, key: u64) {
        match dialogs::take_opened(key) {
            Some(Ok(files)) => {
                let Some(dest) = self.user_dir.clone().or_else(user_dir) else {
                    cx.requests.push(Request::Message(
                        "Packages cannot be installed here: there is no user data folder."
                            .into(),
                    ));
                    return;
                };
                match install_picked(&files, &dest) {
                    Ok(name) => {
                        self.rescan();
                        self.refresh(cx.project_path);
                        match self.installed(&name) {
                            Some(i) if i.dir != dest => {
                                cx.requests.push(Request::Message(format!(
                                    "\"{name}\" was installed in {}, but the package with the \
                                     same name in {} is used instead.",
                                    dest.display(),
                                    i.dir.display()
                                )))
                            }
                            _ => {}
                        }
                        self.sel = match self.work.iter().position(|n| *n == name) {
                            Some(i) => Some(Sel::Used(i)),
                            None => Some(Sel::Available(name)),
                        };
                    }
                    Err(e) => cx.requests.push(Request::Message(e)),
                }
            }
            Some(Err(e)) => cx.requests.push(Request::Message(e)),
            None => {}
        }
    }

    fn confirm_dialog(&mut self, cx: &mut Cx) {
        let Some(pkg) = self.confirm_uninstall.clone() else {
            return;
        };
        let mut choice = None;
        let id = egui::Id::new(("ext_uninstall", self as *const Self as usize));
        let d = crate::mdi::dialog(&cx.ctx, id, "Uninstall Package", |ui| {
            ui.label(format!(
                "Remove the package \"{}\" from {}?",
                pkg.name,
                pkg.dir.display()
            ));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if instance_ui::chrome::dialog_button(ui, "Yes", true).clicked() {
                    choice = Some(true);
                }
                if instance_ui::chrome::dialog_button(ui, "No", false).clicked() {
                    choice = Some(false);
                }
            });
        });
        if d.close {
            choice = Some(false);
        }
        if let Some(c) = choice {
            self.confirm_uninstall = None;
            if c {
                match uninstall(&pkg.name, &pkg.dir) {
                    Ok(()) => {
                        self.rescan();
                        if matches!(&self.sel, Some(Sel::Available(n)) if *n == pkg.name) {
                            self.sel = None;
                        }
                    }
                    Err(e) => cx.requests.push(Request::Message(e)),
                }
            }
        }
    }

    /// A list box (white, sunken) of `rows`; returns (clicked, double-clicked) row.
    fn list_box(
        ui: &mut Ui,
        id: &str,
        size: Vec2,
        rows: &[(String, bool, bool)],
    ) -> (Option<usize>, Option<usize>) {
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 0.0, theme::LIST_BG);
        instance_ui::chrome::sunken(p, rect.shrink(2.0));
        let inner = rect.shrink(3.0);
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner).id_salt(id));
        child.set_clip_rect(inner);
        let mut out = (None, None);
        egui::ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width();
                for (i, (text, selected, missing)) in rows.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::click());
                    resp.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            true,
                            *selected,
                            text,
                        )
                    });
                    let p = ui.painter();
                    if *selected {
                        p.rect_filled(r, 0.0, theme::SELECTION);
                    } else if resp.hovered() {
                        p.rect_filled(r, 0.0, theme::HOVER_FILL);
                    }
                    let color = match (*selected, *missing) {
                        (true, _) => Color32::WHITE,
                        (false, true) => theme::ERROR_TEXT,
                        (false, false) => theme::INK,
                    };
                    p.with_clip_rect(r).text(
                        pos2(r.left() + 4.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        text,
                        egui::TextStyle::Body.resolve(ui.style()),
                        color,
                    );
                    if resp.clicked() {
                        out.0 = Some(i);
                    }
                    if resp.double_clicked() {
                        out.1 = Some(i);
                    }
                }
            });
        out
    }

    /// The package information below the lists.
    fn info(&self, ui: &mut Ui) {
        let Some(name) = self.selected_name() else {
            ui.label(egui::RichText::new("Select a package to see its information.").weak());
            return;
        };
        let Some(inst) = self.installed(name) else {
            ui.label(egui::RichText::new(name).strong());
            ui.colored_label(
                theme::ERROR_TEXT,
                "This package is not installed. The project keeps using it; install it to \
                 run the game or create an executable.",
            );
            return;
        };
        let pkg = match &inst.pkg {
            Ok(p) => p,
            Err(e) => {
                ui.label(egui::RichText::new(name).strong());
                ui.colored_label(theme::ERROR_TEXT, format!("The package cannot be read: {e}"));
                return;
            }
        };
        let title = if pkg.name.is_empty() { name } else { &pkg.name };
        ui.label(egui::RichText::new(title).strong());
        egui::Grid::new("ext_info")
            .num_columns(2)
            .spacing([10.0, 2.0])
            .striped(false)
            .show(ui, |ui| {
                for (label, value) in [
                    ("Version:", &pkg.version_text),
                    ("Author:", &pkg.author),
                    ("Date:", &pkg.date),
                    ("License:", &pkg.license),
                ] {
                    if !value.trim().is_empty() {
                        ui.label(label);
                        ui.label(value.trim());
                        ui.end_row();
                    }
                }
                if !pkg.uses.is_empty() {
                    ui.label("Uses:");
                    ui.label(pkg.uses.join(", "));
                    ui.end_row();
                }
                ui.label("Location:");
                ui.label(inst.dir.display().to_string());
                ui.end_row();
            });
        if !inst.has_dat {
            ui.colored_label(
                theme::ERROR_TEXT,
                format!("The data file \"{name}.dat\" is missing."),
            );
        }
        if !pkg.description.trim().is_empty() {
            ui.add_space(4.0);
            ui.label(pkg.description.trim());
        }
        let funcs: Vec<&extpkg::ExtFunction> = pkg
            .files
            .iter()
            .flat_map(|f| &f.functions)
            .filter(|f| !f.hidden)
            .collect();
        let consts: Vec<&extpkg::ExtConstant> = pkg
            .files
            .iter()
            .flat_map(|f| &f.constants)
            .filter(|c| !c.hidden)
            .collect();
        if !funcs.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(format!("Functions ({}):", funcs.len())).strong());
            for f in funcs {
                let r = ui.label(egui::RichText::new(signature(f)).monospace());
                if !f.help_line.trim().is_empty() {
                    r.on_hover_text(f.help_line.trim());
                }
            }
        }
        if !consts.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(format!("Constants ({}):", consts.len())).strong());
            for c in consts {
                ui.label(egui::RichText::new(format!("{} = {}", c.name, c.value)).monospace());
            }
        }
    }
}

/// A push button with a painted arrow (left or right), of fixed size.
fn arrow_button(ui: &mut Ui, left: bool, label: &str, enabled: bool) -> egui::Response {
    let size = Vec2::new(32.0, 24.0);
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let v = if enabled {
            ui.style().interact(&resp)
        } else {
            &ui.style().visuals.widgets.inactive
        };
        let p = ui.painter();
        p.rect(rect, v.corner_radius, v.weak_bg_fill, v.bg_stroke, egui::StrokeKind::Inside);
        let c = rect.center();
        let (dx, h) = (5.0, 6.0);
        let pts = if left {
            vec![pos2(c.x - dx, c.y), pos2(c.x + dx, c.y - h), pos2(c.x + dx, c.y + h)]
        } else {
            vec![pos2(c.x + dx, c.y), pos2(c.x - dx, c.y - h), pos2(c.x - dx, c.y + h)]
        };
        let fill = if enabled {
            Color32::from_rgb(30, 110, 40)
        } else {
            Color32::from_gray(170)
        };
        p.add(egui::Shape::convex_polygon(pts, fill, egui::Stroke::NONE));
    }
    resp.on_hover_text(label)
}

impl Editor for ExtensionsEditor {
    fn key(&self) -> ResKey {
        ResKey::Extensions
    }
    fn title(&self) -> String {
        "Extension Packages".into()
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(600.0, 520.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        self.refresh(cx.project_path);
        let install_key = dialogs::key("ext-install", self as *const Self);
        self.handle_install(cx, install_key);
        self.confirm_dialog(cx);
        // an uninstall above asks for a rescan
        self.refresh(cx.project_path);
        if let Some(Sel::Used(i)) = self.sel {
            if i >= self.work.len() {
                self.sel = None;
            }
        }

        let full_w = ui.available_width();
        let full_h = ui.available_height();
        let mid_w = 44.0;
        let col_w = ((full_w - mid_w - 2.0 * ui.spacing().item_spacing.x) / 2.0).max(80.0);
        let list_h = (full_h * 0.36).clamp(90.0, 220.0);

        let used_rows: Vec<(String, bool, bool)> = self
            .work
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let missing = self.installed(n).is_none();
                let text = if missing {
                    format!("{n} (not installed)")
                } else {
                    n.clone()
                };
                (text, self.sel == Some(Sel::Used(i)), missing)
            })
            .collect();
        let avail: Vec<String> = self.available().iter().map(|i| i.name.clone()).collect();
        let avail_rows: Vec<(String, bool, bool)> = avail
            .iter()
            .map(|n| {
                let selected = matches!(&self.sel, Some(Sel::Available(s)) if s == n);
                (n.clone(), selected, false)
            })
            .collect();

        let mut action_use: Option<String> = None;
        let mut action_unuse: Option<usize> = None;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(col_w);
                ui.label("Used packages:");
                let (c, d) = Self::list_box(ui, "ext_used", Vec2::new(col_w, list_h), &used_rows);
                if let Some(i) = c {
                    self.sel = Some(Sel::Used(i));
                }
                if d.is_some() {
                    action_unuse = d;
                }
            });
            ui.vertical(|ui| {
                ui.set_width(mid_w);
                ui.add_space(18.0 + (list_h - 24.0 * 2.0 - 8.0) / 2.0);
                let sel_avail = match &self.sel {
                    Some(Sel::Available(n)) => Some(n.clone()),
                    _ => None,
                };
                let sel_used = match self.sel {
                    Some(Sel::Used(i)) => Some(i),
                    _ => None,
                };
                ui.horizontal(|ui| {
                    ui.add_space((mid_w - 32.0) / 2.0);
                    if arrow_button(ui, true, "Use the package", sel_avail.is_some()).clicked() {
                        action_use = sel_avail.clone();
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space((mid_w - 32.0) / 2.0);
                    if arrow_button(ui, false, "Stop using the package", sel_used.is_some())
                        .clicked()
                    {
                        action_unuse = sel_used;
                    }
                });
            });
            ui.vertical(|ui| {
                ui.set_width(col_w);
                ui.label("Available packages:");
                let (c, d) =
                    Self::list_box(ui, "ext_avail", Vec2::new(col_w, list_h), &avail_rows);
                if let Some(i) = c {
                    self.sel = Some(Sel::Available(avail[i].clone()));
                }
                if let Some(i) = d {
                    action_use = Some(avail[i].clone());
                }
            });
        });
        if let Some(n) = action_use {
            self.use_package(&n);
        }
        if let Some(i) = action_unuse {
            self.stop_using(i);
        }

        let sel_inst = self.selected_name().and_then(|n| self.installed(n)).cloned();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let help = sel_inst.as_ref().and_then(|i| i.chm.clone());
            if ui
                .add_enabled(help.is_some(), egui::Button::new("Help"))
                .on_hover_text("Open the help file of the selected package")
                .clicked()
            {
                if let Some(h) = help {
                    if let Err(e) = crate::platform::open_with_os(&h) {
                        cx.requests.push(Request::Message(e));
                    }
                }
            }
            if ui
                .add_enabled(dialogs::HAS_FILESYSTEM, egui::Button::new("Install…"))
                .on_hover_text("Install a package (.ged with its .dat and .chm files)")
                .clicked()
            {
                dialogs::request_open(
                    install_key,
                    &[
                        ("Extension packages", &["ged", "gex"]),
                        ("All files", &["*"]),
                    ],
                    true,
                );
            }
            let removable = sel_inst
                .as_ref()
                .filter(|i| self.user_dir.as_ref() == Some(&i.dir));
            if ui
                .add_enabled(removable.is_some(), egui::Button::new("Uninstall"))
                .on_hover_text(
                    "Remove the selected package from your extension folder \
                     (packages installed with the IDE cannot be removed here)",
                )
                .clicked()
            {
                self.confirm_uninstall = removable.cloned();
            }
        });
        ui.add_space(4.0);
        let info_h = ui.available_height() - 18.0;
        crate::widgets::group_box(ui, "Information", full_w, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("ext_info_scroll")
                .max_height(info_h.max(40.0))
                .auto_shrink([false, false])
                .show(ui, |ui| self.info(ui));
        });
    }
    fn is_dirty(&self, p: &Project) -> bool {
        p.extension_packages != self.work
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        if p.extension_packages == self.work {
            return false;
        }
        p.extension_packages = self.work.clone();
        true
    }
}

#[cfg(test)]
mod tests;
