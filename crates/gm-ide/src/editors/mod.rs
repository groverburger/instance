//! Resource forms (specs/ide.md §1.3): each edits a working copy that is committed on
//! OK and discarded on Cancel.

pub mod background;
pub mod constants;
pub mod extensions;
pub mod font;
pub mod gameinfo;
pub mod included;
pub mod object;
pub mod path;
pub mod room;
pub mod script;
pub mod settings;
pub mod sound;
pub mod sprite;
pub mod timeline;
pub mod triggers;

use eframe::egui::{self, Context, TextureId, Ui, Vec2};
use gm_format::{Action, Project, ResourceKind};

use crate::prefs::Prefs;
use crate::project_ops::{now_timestamp, Named};
use crate::widgets::code_editor::ProjectNames;
use crate::widgets::{owner_key, TextureCache};

/// What a form edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResKey {
    Res(ResourceKind, usize),
    GameInfo,
    Settings,
    Extensions,
    Constants,
    Triggers,
    IncludedFiles,
}

/// Requests from forms to the main window.
pub enum Request {
    /// Open (or bring to front) the form of a resource.
    Open(ResKey),
    /// Show a message box.
    Message(String),
}

/// Shared context handed to forms each frame.
pub struct Cx<'a> {
    pub ctx: Context,
    pub project: &'a Project,
    pub tex: &'a mut TextureCache,
    /// Generation of committed project images (bumped on every commit/load).
    pub gen: u64,
    /// IDE-internal action clipboard (works across forms, spec §13.3).
    pub clipboard: &'a mut Vec<Action>,
    pub requests: &'a mut Vec<Request>,
    pub names: &'a ProjectNames,
    pub prefs: &'a Prefs,
    /// Project-wide id allocators for room instances and tiles (spec §15.4).
    pub alloc: &'a mut IdAlloc,
}

/// Instance/tile id allocator shared by all room forms, so that ids handed out by
/// concurrently open forms never collide.
#[derive(Debug, Clone, Copy, Default)]
pub struct IdAlloc {
    pub instance: i32,
    pub tile: i32,
}

impl IdAlloc {
    pub fn from_project(p: &Project) -> Self {
        let mut a = IdAlloc {
            instance: p.last_instance_id,
            tile: p.last_tile_id,
        };
        for r in p.rooms.iter().flatten() {
            for i in &r.instances {
                a.instance = a.instance.max(i.id);
            }
            for t in &r.tiles {
                a.tile = a.tile.max(t.id);
            }
        }
        a
    }
}

impl Cx<'_> {
    pub fn alloc_instance_id(&mut self) -> i32 {
        self.alloc.instance = self.alloc.instance.max(self.project.last_instance_id) + 1;
        self.alloc.instance
    }

    pub fn alloc_tile_id(&mut self) -> i32 {
        self.alloc.tile = self.alloc.tile.max(self.project.last_tile_id) + 1;
        self.alloc.tile
    }

    /// Texture, size and origin of frame `frame` of committed sprite `idx`.
    pub fn sprite_tex(&mut self, idx: i32, frame: usize) -> Option<(TextureId, Vec2, Vec2)> {
        let s = self
            .project
            .sprites
            .get(usize::try_from(idx).ok()?)?
            .as_ref()?;
        let f = s.frames.get(frame.min(s.frames.len().max(1) - 1))?;
        let t = self.tex.get(
            &self.ctx,
            owner_key("spr", idx as usize),
            frame,
            self.gen,
            f,
        )?;
        Some((
            t,
            Vec2::new(f.width as f32, f.height as f32),
            Vec2::new(s.origin_x as f32, s.origin_y as f32),
        ))
    }

    /// Texture and size of committed background `idx`.
    pub fn background_tex(&mut self, idx: i32) -> Option<(TextureId, Vec2)> {
        let b = self
            .project
            .backgrounds
            .get(usize::try_from(idx).ok()?)?
            .as_ref()?;
        let t = self.tex.get(
            &self.ctx,
            owner_key("bck", idx as usize),
            0,
            self.gen,
            &b.image,
        )?;
        Some((t, Vec2::new(b.image.width as f32, b.image.height as f32)))
    }
}

/// A form.
pub trait Editor {
    fn key(&self) -> ResKey;
    fn title(&self) -> String;
    fn default_size(&self) -> Vec2 {
        Vec2::new(520.0, 420.0)
    }
    /// Draw the form contents (below the OK/Cancel bar).
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx);
    /// Whether the working copy differs from the project.
    fn is_dirty(&self, p: &Project) -> bool;
    /// Copy the working copy into the project. Returns true if anything changed.
    fn commit(&mut self, p: &mut Project) -> bool;
    /// The resource was renamed from the tree while the form is open.
    fn on_renamed(&mut self, _name: &str) {}
}

/// Commit a list-resource working copy: stamps `last_changed` when it differs.
pub fn commit_slot<T: Named + PartialEq>(slot: Option<&mut Option<T>>, work: &T) -> bool {
    let Some(slot) = slot else { return false };
    if slot.as_ref() == Some(work) {
        return false;
    }
    let mut w = work.clone();
    w.set_last_changed(now_timestamp());
    *slot = Some(w);
    true
}

/// Whether a list-resource working copy differs from its slot.
pub fn slot_dirty<T: PartialEq>(slot: Option<&Option<T>>, work: &T) -> bool {
    match slot {
        Some(Some(x)) => x != work,
        _ => false,
    }
}

/// Create the form for `key`, or None if the resource does not exist.
pub fn open_editor(key: ResKey, p: &Project, uid: u64) -> Option<Box<dyn Editor>> {
    Some(match key {
        ResKey::Res(kind, i) => match kind {
            ResourceKind::Sprite => Box::new(sprite::SpriteEditor::new(
                i,
                p.sprites.get(i)?.clone()?,
                uid,
            )),
            ResourceKind::Sound => Box::new(sound::SoundEditor::new(i, p.sounds.get(i)?.clone()?)),
            ResourceKind::Background => Box::new(background::BackgroundEditor::new(
                i,
                p.backgrounds.get(i)?.clone()?,
                uid,
            )),
            ResourceKind::Path => Box::new(path::PathEditor::new(i, p.paths.get(i)?.clone()?)),
            ResourceKind::Script => {
                Box::new(script::ScriptEditor::new(i, p.scripts.get(i)?.clone()?))
            }
            ResourceKind::Font => Box::new(font::FontEditor::new(i, p.fonts.get(i)?.clone()?)),
            ResourceKind::Timeline => Box::new(timeline::TimelineEditor::new(
                i,
                p.timelines.get(i)?.clone()?,
            )),
            ResourceKind::Object => {
                Box::new(object::ObjectEditor::new(i, p.objects.get(i)?.clone()?))
            }
            ResourceKind::Room => Box::new(room::RoomEditor::new(i, p.rooms.get(i)?.clone()?, p)),
            ResourceKind::GameInfo => return open_editor(ResKey::GameInfo, p, uid),
            ResourceKind::Settings => return open_editor(ResKey::Settings, p, uid),
            ResourceKind::Extensions => return open_editor(ResKey::Extensions, p, uid),
            ResourceKind::Unknown(_) => return None,
        },
        ResKey::GameInfo => Box::new(gameinfo::GameInfoEditor::new(&p.game_info)),
        ResKey::Settings => Box::new(settings::SettingsEditor::new(p)),
        ResKey::Extensions => Box::new(extensions::ExtensionsEditor::new(p)),
        ResKey::Constants => Box::new(constants::ConstantsEditor::new(p)),
        ResKey::Triggers => Box::new(triggers::TriggersEditor::new(p)),
        ResKey::IncludedFiles => Box::new(included::IncludedEditor::new(p)),
    })
}

/// Name field shared by resource forms.
pub fn name_field(ui: &mut Ui, name: &mut String) {
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.add(egui::TextEdit::singleline(name).desired_width(180.0));
    });
}
