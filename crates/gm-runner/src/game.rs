//! The [`Game`]: loaded resources plus all runtime state, and the main step
//! (runtime.md §2–§4).

use crate::actions::{self, ActionList};
use crate::audio::Audio;
use crate::draw::Texture;
use crate::events::{ev, ev_other, EventCtx};
use crate::input::{Input, InputEvent};
use crate::instance::Instance;
use crate::particles::Particles;
use crate::render::Gfx;
use crate::res::*;
use crate::room::RoomState;
use gm_format as fmt;
use gml::{Ctx, Error, ErrorKind, Program, Value, Vm};
use std::collections::HashMap;
use std::rc::Rc;

/// A deferred room change / game end request (§3.1).
#[derive(Clone, Debug, PartialEq)]
pub enum Pending {
    Goto(i32),
    Restart,
    End,
    Load(String),
}

/// How errors are presented.
pub trait UiHooks {
    /// Shows an error; returns true to abort the game.
    fn error(&mut self, msg: &str, fatal: bool) -> bool;
    /// Shows a message box.
    fn message(&mut self, msg: &str);
    /// Yes/no question.
    fn question(&mut self, msg: &str) -> bool;
    /// Asks for a string (get_string / get_integer).
    fn get_string(&mut self, msg: &str, default: &str) -> String;
    /// Whether `get_string` shows a real input dialog (else the in-game box is used).
    fn has_input(&self) -> bool {
        false
    }
    /// Message with up to three custom buttons; returns 1..3 (0 if closed).
    fn buttons(&mut self, msg: &str, buttons: &[String]) -> i32 {
        self.message(msg);
        if buttons.iter().any(|b| !b.is_empty()) {
            1
        } else {
            0
        }
    }
    /// Popup menu; None = cancelled.
    fn menu(&mut self, items: &[String]) -> Option<usize> {
        let _ = items;
        None
    }
    /// Colour picker.
    fn color(&mut self, default: i32) -> i32 {
        default
    }
    /// File dialog; empty string = cancelled.
    fn file_dialog(&mut self, save: bool, filter: &str, name: &str) -> String {
        let _ = (save, filter, name);
        String::new()
    }
    /// Directory chooser; empty = cancelled.
    fn directory(&mut self, start: &str) -> String {
        let _ = start;
        String::new()
    }
}

/// Non-interactive UI: prints to stderr and answers with defaults.
pub struct HeadlessUi;

impl UiHooks for HeadlessUi {
    fn error(&mut self, msg: &str, fatal: bool) -> bool {
        eprintln!("{}: {}", if fatal { "FATAL ERROR" } else { "ERROR" }, msg);
        fatal
    }
    fn message(&mut self, msg: &str) {
        eprintln!("[message] {}", msg);
    }
    fn question(&mut self, msg: &str) -> bool {
        eprintln!("[question] {} -> yes", msg);
        true
    }
    fn get_string(&mut self, msg: &str, default: &str) -> String {
        eprintln!("[get_string] {} -> {:?}", msg, default);
        default.to_string()
    }
}

/// Persistent-room snapshot (§3.2).
#[derive(Clone)]
pub struct Snapshot {
    pub state: RoomState,
    pub instances: Vec<Instance>,
}

/// Window properties requested by the game (applied by the frontend).
#[derive(Clone, Debug)]
pub struct WinState {
    pub x: i32,
    pub y: i32,
    /// Client size; 0 = derived from the region and scale.
    pub w: i32,
    pub h: i32,
    pub showborder: bool,
    pub showicons: bool,
    pub stayontop: bool,
    pub sizeable: bool,
    pub color: i32,
    /// Region scale: >0 fixed factor, 0 fill, <0 keep aspect.
    pub region_scale: f64,
    pub center: bool,
    /// Requested mouse position (window_mouse_set), consumed by the frontend.
    pub set_mouse: Option<(f64, f64)>,
    /// Actual client size reported by the frontend.
    pub actual_w: i32,
    pub actual_h: i32,
}

/// One registered highscore entry.
#[derive(Clone, Debug)]
pub struct HighscoreEntry {
    pub name: String,
    pub score: f64,
}

/// The whole game.
pub struct Game {
    // ---------------------------------------------------------- resources
    pub game_id: i32,
    pub settings: fmt::Settings,
    pub game_info: String,
    pub sprites: Vec<Option<SpriteRes>>,
    pub backgrounds: Vec<Option<BackgroundRes>>,
    pub sounds: Vec<Option<SoundRes>>,
    pub paths: Vec<Option<crate::path::PathRes>>,
    pub scripts: Vec<Option<ScriptRes>>,
    pub fonts: Vec<Option<FontRes>>,
    pub timelines: Vec<Option<TimelineRes>>,
    pub objects: Vec<Option<ObjectRes>>,
    pub rooms: Vec<Option<RoomRes>>,
    pub triggers: Vec<Option<TriggerRes>>,
    pub room_order: Vec<i32>,
    /// User constants and resource names (compile-time resolution).
    pub constants: HashMap<String, Value>,
    pub script_names: HashMap<String, i32>,
    /// Effective event tables (own + inherited): object → (type,num) → (owner, list).
    pub effective: Vec<HashMap<(i32, i32), (i32, Rc<ActionList>)>>,
    /// Collision handler per (object, other object): (owner object, list, the
    /// event's target object, i.e. the other object or the ancestor it names).
    pub collision_table: HashMap<(i32, i32), (i32, Rc<ActionList>, i32)>,
    /// Per object: the distinct targets of its effective collision events, ascending.
    pub collision_targets: Vec<Vec<i32>>,
    /// Ancestor chains, self first.
    pub ancestors: Vec<Vec<i32>>,
    /// Objects that have a collision handler at all (fast skip).
    pub has_collision: Vec<bool>,
    /// Per event (type, number): which objects have an effective handler.
    pub event_objs: HashMap<(i32, i32), Rc<Vec<bool>>>,
    /// Per object: the objects it has a collision handler with.
    pub collision_partners: Vec<Vec<i32>>,
    /// Per object: has any mouse event / any outside-boundary event.
    pub has_mouse: Vec<bool>,
    pub has_boundary: Vec<bool>,

    // ---------------------------------------------------------- runtime
    pub instances: Vec<Instance>,
    pub inst_index: HashMap<i32, usize, gml::IdBuildHasher>,
    pub next_instance_id: i32,
    pub next_tile_id: i32,
    pub room: i32,
    pub rs: RoomState,
    pub snapshots: HashMap<i32, Snapshot>,
    pub pending: Option<Pending>,
    /// Nesting depth of running action lists (events, timeline moments).
    pub list_depth: u32,
    /// Action lists at this depth or shallower stop after their current action
    /// (a room change was requested while they ran; 0 = none).
    pub abort_at: u32,
    pub transition_kind: i32,
    pub transition_steps: i32,
    pub transition_color: i32,
    pub score: f64,
    pub lives: f64,
    pub health: f64,
    pub show_score: bool,
    pub show_lives: bool,
    pub show_health: bool,
    pub caption_score: String,
    pub caption_lives: String,
    pub caption_health: String,
    pub cursor_sprite: i32,
    pub error_occurred: bool,
    pub error_last: String,
    pub ev: EventCtx,
    pub input: Input,
    pub gfx: Gfx,
    pub audio: Audio,
    pub particles: Particles,
    pub highscores: Vec<HighscoreEntry>,
    pub automatic_draw: bool,
    pub ended: bool,
    pub aborted: bool,
    pub headless: bool,
    pub ui: Box<dyn UiHooks>,
    pub frame: u64,
    pub fps: f64,
    pub start_time: web_time::Instant,
    /// Scripted input: (step number, event) applied at the start of that step.
    pub scripted_input: Vec<(u64, InputEvent)>,
    /// The window's region size (decided at room start).
    pub region_w: u32,
    pub region_h: u32,
    pub window_caption: String,
    pub fullscreen: bool,
    pub working_directory: String,
    pub program_directory: String,
    pub temp_directory: String,
    /// Open text files, binary files, ini.
    pub files: crate::builtins::files::FileState,
    pub transitions: HashMap<i32, i32>,
    /// Timeline-moment pseudo event in progress.
    pub in_timeline: bool,
    /// Steps per second achieved (for fps), counters.
    pub fps_counter: u32,
    pub fps_time: web_time::Instant,
    /// Sleep requests made during the step (ms), honoured by the window loop.
    pub sleep_ms: f64,
    /// Requested by window functions: (w,h) to resize, centre, etc.
    pub window_dirty: bool,
    pub window_visible: bool,
    pub cursor: i32,
    pub mp: crate::builtins::motion::MpState,
    /// Debug mode state (`--debug`, specs/ide.md §20.3); `None` normally.
    pub debug: Option<Box<crate::debug::Debugger>>,
    pub save_counter: u32,
    pub close_requested: bool,
    /// `sleep` time not yet waited (browser build: the frontend delays the next step).
    pub sleep_debt_ms: f64,
    /// Accumulated seconds spent drawing (diagnostics).
    pub draw_time: f64,
    pub pending_lives_event: bool,
    pub pending_health_event: bool,
    pub win: WinState,
    pub application_title: String,
    pub hs_style: crate::builtins::ui::HighscoreStyle,
    pub hs_loaded: bool,
    pub included_files: Vec<fmt::IncludedFile>,
    /// Command-line parameters (index 0 = program).
    pub parameters: Vec<String>,
    /// Non-blocking input source (frontend), used by io_handle / keyboard_wait.
    pub pump: Option<Box<dyn FnMut() -> Vec<InputEvent>>>,
    /// Multiplayer state (`mplay_*`).
    pub net: gm_net::Mplay,
    /// Extension packages and the DLL bridge.
    pub ext: crate::ext::Extensions,
    /// Timing instrumentation (`GM_PROFILE`).
    pub prof: Option<Box<crate::profile::Profiler>>,
    /// Calls of stubbed or unsupported built-in functions (diagnostics).
    pub unsupported_calls: std::collections::BTreeMap<&'static str, u64>,
    /// Collision broad phase (see spatial.rs).
    pub spatial: std::cell::RefCell<crate::spatial::SpatialIndex>,
    /// Per-object instance lists (see spatial.rs).
    pub objlists: std::cell::RefCell<crate::spatial::ObjLists>,
}

impl Game {
    /// Loads a project into a ready-to-start game (resources compiled, no room yet).
    pub fn new(project: fmt::Project, vm: &mut Vm, headless: bool) -> Result<Game, String> {
        let p = project;
        vm.settings.uninitialized_as_zero = p.settings.treat_uninitialized_as_zero;
        vm.settings.check_script_args = p.settings.argument_errors;
        let mut g = Game {
            game_id: p.game_id,
            settings: p.settings.clone(),
            game_info: p.game_info.rtf.clone(),
            sprites: p.sprites.iter().map(|s| s.as_ref().map(SpriteRes::from_format)).collect(),
            backgrounds: p
                .backgrounds
                .iter()
                .map(|b| b.as_ref().map(BackgroundRes::from_format))
                .collect(),
            sounds: p
                .sounds
                .iter()
                .map(|s| {
                    s.as_ref().map(|s| SoundRes {
                        name: s.name.clone(),
                        kind: s.kind,
                        file_type: s.file_type.clone(),
                        file_name: s.file_name.clone(),
                        data: s.data.as_ref().map(|d| std::sync::Arc::from(d.as_slice())),
                        effects: s.effects,
                        volume: s.volume,
                        pan: s.pan,
                        preload: s.preload,
                    })
                })
                .collect(),
            paths: p.paths.iter().map(|x| x.as_ref().map(crate::path::PathRes::from_format)).collect(),
            scripts: p
                .scripts
                .iter()
                .map(|s| {
                    s.as_ref().map(|s| ScriptRes {
                        name: s.name.clone(),
                        code: s.code.clone(),
                        prog: None,
                        error: None,
                    })
                })
                .collect(),
            fonts: p
                .fonts
                .iter()
                .map(|f| {
                    f.as_ref().map(|f| FontRes {
                        name: f.name.clone(),
                        font_name: f.font_name.clone(),
                        size: f.size,
                        bold: f.bold,
                        italic: f.italic,
                        first: f.range_start as i32,
                        last: f.range_end,
                        sprite: None,
                    })
                })
                .collect(),
            timelines: p
                .timelines
                .iter()
                .map(|t| {
                    t.as_ref().map(|t| TimelineRes {
                        name: t.name.clone(),
                        moments: Default::default(),
                    })
                })
                .collect(),
            objects: p
                .objects
                .iter()
                .map(|o| {
                    o.as_ref().map(|o| ObjectRes {
                        name: o.name.clone(),
                        sprite: o.sprite,
                        solid: o.solid,
                        visible: o.visible,
                        depth: o.depth,
                        persistent: o.persistent,
                        parent: o.parent,
                        mask: o.mask_sprite,
                        events: HashMap::new(),
                    })
                })
                .collect(),
            rooms: p
                .rooms
                .iter()
                .map(|r| {
                    r.as_ref().map(|r| RoomRes {
                        data: r.clone(),
                        creation: None,
                        inst_creation: HashMap::new(),
                    })
                })
                .collect(),
            triggers: p
                .triggers
                .iter()
                .map(|t| {
                    t.as_ref().map(|t| TriggerRes {
                        name: t.name.clone(),
                        condition: None,
                        moment: t.moment,
                    })
                })
                .collect(),
            room_order: p.room_order.clone(),
            constants: HashMap::new(),
            script_names: HashMap::new(),
            effective: Vec::new(),
            collision_table: HashMap::new(),
            ancestors: Vec::new(),
            has_collision: Vec::new(),
            event_objs: HashMap::new(),
            collision_partners: Vec::new(),
            collision_targets: Vec::new(),
            has_mouse: Vec::new(),
            has_boundary: Vec::new(),
            instances: Vec::new(),
            inst_index: HashMap::default(),
            next_instance_id: p.last_instance_id.max(gml::special::FIRST_INSTANCE_ID) + 1,
            next_tile_id: p.last_tile_id.max(fmt::FIRST_TILE_ID) + 1,
            room: -1,
            rs: RoomState::default(),
            snapshots: HashMap::new(),
            pending: None,
            list_depth: 0,
            abort_at: 0,
            transition_kind: 0,
            transition_steps: 80,
            transition_color: 0,
            score: 0.0,
            lives: -1.0,
            health: 100.0,
            show_score: true,
            show_lives: false,
            show_health: false,
            caption_score: "Score: ".into(),
            caption_lives: "Lives: ".into(),
            caption_health: "Health: ".into(),
            cursor_sprite: -1,
            error_occurred: false,
            error_last: String::new(),
            ev: EventCtx::default(),
            input: Input::default(),
            gfx: Gfx::new(p.settings.interpolate_pixels),
            audio: Audio::new(headless),
            particles: Particles::default(),
            highscores: Vec::new(),
            automatic_draw: true,
            ended: false,
            aborted: false,
            headless,
            ui: Box::new(HeadlessUi),
            frame: 0,
            fps: 0.0,
            start_time: web_time::Instant::now(),
            scripted_input: Vec::new(),
            region_w: 640,
            region_h: 480,
            window_caption: String::new(),
            fullscreen: p.settings.start_fullscreen,
            working_directory: String::new(),
            program_directory: String::new(),
            temp_directory: crate::vfs::temp_dir().to_string_lossy().trim_end_matches('\\').to_string(),
            files: Default::default(),
            transitions: HashMap::new(),
            in_timeline: false,
            fps_counter: 0,
            fps_time: web_time::Instant::now(),
            sleep_ms: 0.0,
            window_dirty: true,
            window_visible: true,
            cursor: 0,
            mp: Default::default(),
            debug: None,
            save_counter: 0,
            close_requested: false,
            sleep_debt_ms: 0.0,
            draw_time: 0.0,
            pending_lives_event: false,
            pending_health_event: false,
            win: WinState {
                x: -1,
                y: -1,
                w: 0,
                h: 0,
                showborder: !p.settings.dont_draw_border,
                showicons: !p.settings.dont_show_buttons,
                stayontop: p.settings.always_on_top,
                sizeable: p.settings.allow_window_resize,
                color: p.settings.color_outside_room as i32,
                region_scale: if p.settings.scaling > 0 { p.settings.scaling as f64 / 100.0 } else if p.settings.scaling == 0 { 0.0 } else { -1.0 },
                center: true,
                set_mouse: None,
                actual_w: 0,
                actual_h: 0,
            },
            application_title: String::new(),
            hs_style: Default::default(),
            hs_loaded: false,
            included_files: p.included_files.clone(),
            parameters: std::env::args().collect(),
            pump: None,
            net: crate::net::new_state(p.game_id),
            ext: Default::default(),
            prof: crate::profile::Profiler::from_env(),
            unsupported_calls: Default::default(),
            spatial: Default::default(),
            objlists: Default::default(),
        };
        // resource names → constants
        macro_rules! names {
            ($v:expr) => {
                for (i, r) in $v.iter().enumerate() {
                    if let Some(r) = r {
                        g.constants.entry(r.name.clone()).or_insert(Value::Real(i as f64));
                    }
                }
            };
        }
        names!(g.sprites);
        names!(g.sounds);
        names!(g.backgrounds);
        names!(g.paths);
        names!(g.fonts);
        names!(g.timelines);
        names!(g.objects);
        for (i, r) in g.rooms.iter().enumerate() {
            if let Some(r) = r {
                g.constants.entry(r.data.name.clone()).or_insert(Value::Real(i as f64));
            }
        }
        for (i, s) in g.scripts.iter().enumerate() {
            if let Some(s) = s {
                g.constants.entry(s.name.clone()).or_insert(Value::Real(i as f64));
                g.script_names.insert(s.name.clone(), i as i32);
            }
        }
        // extension packages: functions and constants (extensions.md §2.3)
        crate::ext::load(&mut g, vm, &p.extension_packages)?;
        // user constants (in order, may refer to earlier ones)
        for c in &p.constants {
            let v = match vm.compile_expression(&format!("constant {}", c.name), c.value.as_bytes(), &g) {
                Ok(prog) => {
                    let mut tmp = crate::host::ConstHost;
                    vm.run(&mut tmp, &prog, Ctx::new(gml::special::NOONE, gml::special::NOONE), &[])
                        .unwrap_or(Value::Real(0.0))
                }
                Err(e) => {
                    eprintln!("constant {}: {}", c.name, e);
                    Value::Real(0.0)
                }
            };
            // a resource name wins over a user constant of the same name (VERIFIED
            // black-box, bb_clash clash.const_vs_object)
            g.constants.entry(c.name.clone()).or_insert(v);
        }
        g.compile_all(vm, &p);
        crate::ext::compile(&mut g, vm);
        g.build_event_tables();
        Ok(g)
    }

    /// Compiles every script, event, timeline, trigger and creation code.
    fn compile_all(&mut self, vm: &mut Vm, p: &fmt::Project) {
        let mut errors: Vec<String> = Vec::new();
        // scripts
        let mut progs = Vec::new();
        for (i, s) in self.scripts.iter().enumerate() {
            if let Some(s) = s {
                match vm.compile(&s.name, s.code.as_bytes(), &*self) {
                    Ok(pr) => progs.push((i, Some(pr), None)),
                    Err(e) => {
                        let m = format!("COMPILATION ERROR in Script: {}\n{}", s.name, e);
                        errors.push(m.clone());
                        progs.push((i, None, Some(m)));
                    }
                }
            }
        }
        for (i, pr, e) in progs {
            let s = self.scripts[i].as_mut().unwrap();
            s.prog = pr;
            s.error = e;
        }
        // objects
        let mut obj_events = Vec::new();
        for (oi, o) in p.objects.iter().enumerate() {
            if let Some(o) = o {
                let mut map = HashMap::new();
                for e in &o.events {
                    if e.actions.is_empty() {
                        continue;
                    }
                    let where_ = format!(
                        "{} event for object {}",
                        crate::events::event_name(e.event_type, e.number, self),
                        o.name
                    );
                    let list = actions::compile_list(vm, self, &e.actions, &where_, &mut errors);
                    map.insert((e.event_type, e.number), Rc::new(list));
                }
                obj_events.push((oi, map));
            }
        }
        for (oi, map) in obj_events {
            self.objects[oi].as_mut().unwrap().events = map;
        }
        // timelines
        let mut tls = Vec::new();
        for (ti, t) in p.timelines.iter().enumerate() {
            if let Some(t) = t {
                let mut moments = std::collections::BTreeMap::new();
                for m in &t.moments {
                    let where_ = format!("moment {} of time line {}", m.step, t.name);
                    let list = actions::compile_list(vm, self, &m.actions, &where_, &mut errors);
                    moments.insert(m.step, Rc::new(list));
                }
                tls.push((ti, moments));
            }
        }
        for (ti, m) in tls {
            self.timelines[ti].as_mut().unwrap().moments = m;
        }
        // triggers
        let mut trs = Vec::new();
        for (i, t) in p.triggers.iter().enumerate() {
            if let Some(t) = t {
                // The condition is a piece of code that returns the result, not an
                // expression (runtime.md §5 trigger row, verified in GM 8.1).
                let pr = vm.compile(&format!("trigger {}", t.name), t.condition.as_bytes(), &*self);
                match pr {
                    Ok(pr) => trs.push((i, Some(pr))),
                    Err(e) => {
                        errors.push(format!("COMPILATION ERROR in condition of trigger {}\n{}", t.name, e));
                        trs.push((i, None));
                    }
                }
            }
        }
        for (i, pr) in trs {
            self.triggers[i].as_mut().unwrap().condition = pr;
        }
        // rooms
        let mut rcs = Vec::new();
        for (ri, r) in self.rooms.iter().enumerate() {
            if let Some(r) = r {
                let mut cc = None;
                if !r.data.creation_code.trim().is_empty() {
                    match vm.compile(
                        &format!("creation code of room {}", r.data.name),
                        r.data.creation_code.as_bytes(),
                        &*self,
                    ) {
                        Ok(pr) => cc = Some(pr),
                        Err(e) => errors.push(format!(
                            "COMPILATION ERROR in creation code of room {}\n{}",
                            r.data.name, e
                        )),
                    }
                }
                let mut ics = HashMap::new();
                for inst in &r.data.instances {
                    if !inst.creation_code.trim().is_empty() {
                        match vm.compile(
                            &format!("creation code of instance {}", inst.id),
                            inst.creation_code.as_bytes(),
                            &*self,
                        ) {
                            Ok(pr) => {
                                ics.insert(inst.id, pr);
                            }
                            Err(e) => errors.push(format!(
                                "COMPILATION ERROR in creation code of instance {}\n{}",
                                inst.id, e
                            )),
                        }
                    }
                }
                rcs.push((ri, cc, ics));
            }
        }
        for (ri, cc, ics) in rcs {
            let r = self.rooms[ri].as_mut().unwrap();
            r.creation = cc;
            r.inst_creation = ics;
        }
        for e in errors {
            self.ui_error(&e, false);
        }
    }

    /// Builds ancestor chains and the effective event tables (§6.1, §6.3).
    pub fn build_event_tables(&mut self) {
        let n = self.objects.len();
        // parents (and the object list) may have changed
        self.objlists.get_mut().invalidate();
        self.ancestors = (0..n)
            .map(|i| {
                let mut chain = vec![];
                let mut cur = i as i32;
                while cur >= 0 && (cur as usize) < n && self.objects[cur as usize].is_some() {
                    if chain.contains(&cur) {
                        break;
                    }
                    chain.push(cur);
                    cur = self.objects[cur as usize].as_ref().unwrap().parent;
                }
                chain
            })
            .collect();
        self.effective = (0..n)
            .map(|i| {
                let mut m: HashMap<(i32, i32), (i32, Rc<ActionList>)> = HashMap::new();
                for &a in self.ancestors[i].iter() {
                    if let Some(o) = &self.objects[a as usize] {
                        for (k, l) in &o.events {
                            m.entry(*k).or_insert((a, l.clone()));
                        }
                    }
                }
                m
            })
            .collect();
        let mut eo: HashMap<(i32, i32), Vec<bool>> = HashMap::new();
        for (i, m) in self.effective.iter().enumerate() {
            for k in m.keys() {
                eo.entry(*k).or_insert_with(|| vec![false; n])[i] = true;
            }
        }
        self.event_objs = eo.into_iter().map(|(k, v)| (k, Rc::new(v))).collect();
        self.has_mouse = self.effective.iter().map(|m| m.keys().any(|(t, _)| *t == ev::MOUSE)).collect();
        self.has_boundary = self
            .effective
            .iter()
            .map(|m| m.keys().any(|(t, n)| *t == ev::OTHER && (*n == 0 || *n == 1 || (40..58).contains(n))))
            .collect();
        self.collision_table.clear();
        self.collision_partners = vec![Vec::new(); n];
        self.collision_targets = vec![Vec::new(); n];
        self.has_collision = vec![false; n];
        for a in 0..n {
            if self.objects[a].is_none() {
                continue;
            }
            let any = self.effective[a].keys().any(|k| k.0 == ev::COLLISION);
            self.has_collision[a] = any;
            if !any {
                continue;
            }
            for b in 0..n {
                if self.objects[b].is_none() {
                    continue;
                }
                // walk A's chain; first object with a collision event targeting B or
                // an ancestor of B; choose the target nearest to B.
                'outer: for &owner in &self.ancestors[a] {
                    let o = self.objects[owner as usize].as_ref().unwrap();
                    for &target in &self.ancestors[b] {
                        if let Some(l) = o.events.get(&(ev::COLLISION, target)) {
                            self.collision_table.insert((a as i32, b as i32), (owner, l.clone(), target));
                            self.collision_partners[a].push(b as i32);
                            if !self.collision_targets[a].contains(&target) {
                                self.collision_targets[a].push(target);
                            }
                            break 'outer;
                        }
                    }
                }
            }
            self.collision_targets[a].sort_unstable();
        }
    }

    // ------------------------------------------------------------------ helpers

    pub fn object(&self, i: i32) -> Option<&ObjectRes> {
        if i < 0 {
            return None;
        }
        self.objects.get(i as usize).and_then(|o| o.as_ref())
    }

    pub fn sprite(&self, i: i32) -> Option<&SpriteRes> {
        if i < 0 {
            return None;
        }
        self.sprites.get(i as usize).and_then(|o| o.as_ref())
    }

    pub fn background(&self, i: i32) -> Option<&BackgroundRes> {
        if i < 0 {
            return None;
        }
        self.backgrounds.get(i as usize).and_then(|o| o.as_ref())
    }

    /// Whether object `obj` is `anc` or a descendant of it.
    pub fn is_a(&self, obj: i32, anc: i32) -> bool {
        if obj < 0 {
            return false;
        }
        self.ancestors
            .get(obj as usize)
            .map(|c| c.contains(&anc))
            .unwrap_or(false)
    }

    #[inline]
    pub fn inst(&self, id: i32) -> Option<&Instance> {
        self.inst_index.get(&id).map(|&i| &self.instances[i])
    }

    /// Mutable access to an instance. Marks it for re-binning in the collision
    /// index, so every change of position/sprite/mask/scale/angle/activity must
    /// go through here (or call `spatial_invalidate`).
    #[inline]
    pub fn inst_mut(&mut self, id: i32) -> Option<&mut Instance> {
        match self.inst_index.get(&id) {
            Some(&i) => {
                self.spatial.get_mut().mark(i);
                Some(&mut self.instances[i])
            }
            None => None,
        }
    }

    /// Mutable access to the instance at list position `k` (marks it, see `inst_mut`).
    #[inline]
    pub fn inst_at_mut(&mut self, k: usize) -> &mut Instance {
        self.spatial.get_mut().mark(k);
        &mut self.instances[k]
    }

    /// Forces a rebuild of the collision index (bulk changes, sprite edits).
    pub fn spatial_invalidate(&mut self) {
        self.spatial.get_mut().invalidate();
    }

    /// Appends an instance record to the list.
    pub fn push_instance(&mut self, inst: Instance) {
        let k = self.instances.len();
        self.inst_index.insert(inst.id, k);
        self.instances.push(inst);
        self.objlists.borrow_mut().appended(self, k);
    }

    /// Instance or error.
    pub fn inst_req(&mut self, id: i32) -> Result<&mut Instance, Error> {
        self.inst_mut(id)
            .ok_or_else(|| Error::new(format!("Unknown instance id {}", id)))
    }

    /// Ids of the live instances designated by `target` (object incl. descendants,
    /// instance id, all), in instance order.
    pub fn select(&self, target: i32, out: &mut Vec<i32>) {
        if target == gml::special::ALL {
            out.extend(self.instances.iter().filter(|i| i.alive()).map(|i| i.id));
        } else if target >= gml::special::FIRST_INSTANCE_ID {
            if let Some(i) = self.inst(target) {
                if i.alive() {
                    out.push(i.id);
                }
            }
        } else if target >= 0 {
            self.objlists.borrow_mut().for_each(self, target, |k| {
                let i = &self.instances[k];
                if i.alive() {
                    out.push(i.id);
                }
                true
            });
        }
    }

    /// The first live instance designated by `target` (object, id or all).
    pub fn first_of(&self, target: i32) -> Option<i32> {
        if target == gml::special::ALL {
            return self.instances.iter().find(|i| i.alive()).map(|i| i.id);
        }
        if target >= gml::special::FIRST_INSTANCE_ID {
            return self.inst(target).filter(|i| i.alive()).map(|i| i.id);
        }
        if target < 0 {
            return None;
        }
        let mut r = None;
        self.objlists.borrow_mut().for_each(self, target, |k| {
            let i = &self.instances[k];
            if i.alive() {
                r = Some(i.id);
                false
            } else {
                true
            }
        });
        r
    }

    /// Number of live instances designated by `target` (object, id or all).
    pub fn count_of(&self, target: i32) -> usize {
        if target == gml::special::ALL {
            return self.instances.iter().filter(|i| i.alive()).count();
        }
        if target >= gml::special::FIRST_INSTANCE_ID {
            return self.inst(target).map(|i| i.alive() as usize).unwrap_or(0);
        }
        if target < 0 {
            return 0;
        }
        let mut n = 0;
        self.objlists.borrow_mut().for_each(self, target, |k| {
            n += self.instances[k].alive() as usize;
            true
        });
        n
    }

    /// Resolves an "object" argument that may also be self/other/all/an id.
    pub fn select_ctx(&self, ctx: &Ctx, target: i32) -> Vec<i32> {
        let mut v = Vec::new();
        match target {
            gml::special::SELF => {
                if self.inst(ctx.self_id).map(|i| i.alive()).unwrap_or(false) {
                    v.push(ctx.self_id)
                }
            }
            gml::special::OTHER => {
                if self.inst(ctx.other_id).map(|i| i.alive()).unwrap_or(false) {
                    v.push(ctx.other_id)
                }
            }
            t => self.select(t, &mut v),
        }
        v
    }

    pub fn alloc_id(&mut self) -> i32 {
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        id
    }

    /// Adds a new instance record with the object's defaults (no events run).
    pub fn add_instance(&mut self, id: i32, obj: i32, x: f64, y: f64) -> i32 {
        let mut inst = Instance::new(id, obj, x, y);
        if let Some(o) = self.object(obj) {
            inst.sprite_index = o.sprite;
            inst.mask_index = o.mask;
            inst.visible = o.visible;
            inst.solid = o.solid;
            inst.persistent = o.persistent;
            inst.depth = o.depth as f64;
        }
        self.push_instance(inst);
        id
    }

    /// `instance_create`: create and run the Create event (with `other` = caller).
    pub fn create_instance(&mut self, vm: &mut Vm, x: f64, y: f64, obj: i32, caller: i32) -> Result<i32, Error> {
        if self.object(obj).is_none() {
            return Err(Error::new(format!("Creating instance for non-existing object: {}", obj)));
        }
        let id = self.alloc_id();
        self.add_instance(id, obj, x, y);
        let other = if caller >= gml::special::FIRST_INSTANCE_ID { caller } else { id };
        self.run_event(vm, id, ev::CREATE, 0, other)?;
        Ok(id)
    }

    /// Destroys an instance: Destroy event then mark (§5.6).
    pub fn destroy_instance(&mut self, vm: &mut Vm, id: i32, other: i32) -> Result<(), Error> {
        match self.inst(id) {
            Some(i) if !i.destroyed => {}
            _ => return Ok(()),
        }
        let r = self.run_event(vm, id, ev::DESTROY, 0, other);
        if let Some(i) = self.inst_mut(id) {
            i.destroyed = true;
        }
        r
    }

    /// `instance_change` (§5.7).
    pub fn change_instance(&mut self, vm: &mut Vm, id: i32, obj: i32, perf: bool) -> Result<(), Error> {
        if self.object(obj).is_none() {
            return Err(Error::new(format!("Changing instance to non-existing object: {}", obj)));
        }
        if perf {
            self.run_event(vm, id, ev::DESTROY, 0, id)?;
        }
        let (spr, mask, solid, vis, pers, depth) = {
            let o = self.object(obj).unwrap();
            (o.sprite, o.mask, o.solid, o.visible, o.persistent, o.depth)
        };
        let old = self.inst(id).map(|i| i.object_index).unwrap_or(-1);
        if let Some(i) = self.inst_mut(id) {
            i.object_index = obj;
            i.sprite_index = spr;
            i.mask_index = mask;
            i.solid = solid;
            i.visible = vis;
            i.persistent = pers;
            i.depth = depth as f64;
        }
        if let Some(&k) = self.inst_index.get(&id) {
            self.objlists.borrow_mut().changed(self, k, old);
        }
        if perf {
            self.run_event(vm, id, ev::CREATE, 0, id)?;
        }
        Ok(())
    }

    /// Removes destroyed instances from the list.
    pub fn purge_destroyed(&mut self) {
        if self.instances.iter().any(|i| i.destroyed) {
            self.instances.retain(|i| !i.destroyed);
            self.reindex();
        }
    }

    pub fn reindex(&mut self) {
        self.spatial.get_mut().invalidate();
        self.objlists.get_mut().invalidate();
        self.inst_index.clear();
        for (k, i) in self.instances.iter().enumerate() {
            self.inst_index.insert(i.id, k);
        }
    }

    // ------------------------------------------------------------------ errors

    /// Shows an error through the UI. Returns true if the game should abort.
    pub fn ui_error(&mut self, msg: &str, fatal: bool) -> bool {
        let fatal = fatal || self.settings.abort_on_all_errors;
        // (while loading, the game folder is not known yet: no log file)
        if self.settings.write_errors_to_log && !self.working_directory.is_empty() {
            use std::io::Write;
            if let Ok(mut f) =
                crate::vfs::File::open(&std::path::Path::new(&self.working_directory).join("game_errors.log"), crate::vfs::Mode::Append)
            {
                let _ = writeln!(f, "{}", msg);
            }
        }
        if !self.settings.display_errors && !fatal {
            if self.headless {
                eprintln!("ERROR (not displayed): {}", msg);
            }
            return false;
        }
        self.ui.error(msg, fatal) || fatal
    }

    /// Reports a runtime error raised while running an action (§19). Returns
    /// `Err(halt)` if the game must stop.
    pub fn report_error(&mut self, e: Error, action_no: i32) -> Result<(), Error> {
        if e.kind == ErrorKind::Halt {
            return Err(e);
        }
        if e.kind == ErrorKind::AbortEvent {
            // an event abandoned without a message (GM 8.1 crash-like failure)
            return Ok(());
        }
        self.error_occurred = true;
        // GM 8.1's message layout (VERIFIED black-box, bb_lang *.msg): a header,
        // then "In script <name>:" for errors inside scripts and the code excerpt
        // with a caret; errors of built-in functions show only their message.
        let body = match &e.location {
            Some(l) => {
                let script = if self.script_names.contains_key(&*l.program) {
                    format!("In script {}:\n", l.program)
                } else {
                    String::new()
                };
                let line = l.source_line.clone().unwrap_or_default();
                format!("{}{}", script, gml::code_error_text(l.line, l.col, &line, &e.message))
            }
            None => e.message.clone(),
        };
        let obj_name = self
            .object(self.ev.object)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "<none>".into());
        let header: Vec<String> = if self.in_timeline || self.ev.object < 0 {
            vec![format!("ERROR in action number {}:", action_no + 1)]
        } else {
            vec![
                "ERROR in".into(),
                format!("action number {}", action_no + 1),
                format!("of {} Event", crate::events::event_name(self.ev.etype, self.ev.number, self)),
                format!("for object {}:", obj_name),
                String::new(),
            ]
        };
        // error_last: header lines end with CR LF, the body uses CR (as GM 8.1)
        // a compile error in execute_string has no header (bb_lang syn.*.msg)
        self.error_last = if body.starts_with("COMPILATION ERROR") {
            body.replace('\n', "\r")
        } else {
            format!("{}\r\n{}", header.join("\r\n"), body.replace('\n', "\r"))
        };
        let msg = format!("{}\n{}", header.join("\n"), body);
        let abort = self.ui_error(&msg, e.kind == ErrorKind::Fatal);
        if abort {
            self.aborted = true;
            self.ended = true;
            return Err(Error::halt());
        }
        Ok(())
    }

    // ------------------------------------------------------------------ input

    /// Applies an input event (from the window or the scripted queue).
    pub fn input_event(&mut self, vm: &mut Vm, e: InputEvent) {
        if let InputEvent::Close = e {
            self.close_requested = true;
            return;
        }
        let _ = vm;
        self.input.apply(&e);
    }

    // ------------------------------------------------------------------ main loop

    /// Starts the game: first room with Game Start (§2 step 7).
    pub fn start(&mut self, vm: &mut Vm) -> Result<(), Error> {
        vm.randomize();
        self.audio.preload(&self.sounds);
        let first = match self.room_order.first() {
            Some(&r) => r,
            None => return Err(Error::fatal("The game has no rooms.")),
        };
        crate::ext::run_inits(self, vm)?;
        self.goto_room(vm, first, true)?;
        // a room change requested while entering the room is performed at once
        self.process_pending(vm)
    }

    /// Runs one complete step (§4.2), including drawing.
    pub fn step(&mut self, vm: &mut Vm) -> Result<(), Error> {
        if self.ended {
            return Ok(());
        }
        // scripted input
        if !self.scripted_input.is_empty() {
            let f = self.frame;
            let mut i = 0;
            while i < self.scripted_input.len() {
                if self.scripted_input[i].0 <= f {
                    let (_, e) = self.scripted_input.remove(i);
                    self.input_event(vm, e);
                } else {
                    i += 1;
                }
            }
        }
        crate::net::pump(self);
        let r = self.step_inner(vm);
        self.frame += 1;
        // fps
        self.fps_counter += 1;
        let el = self.fps_time.elapsed().as_secs_f64();
        if el >= 1.0 {
            self.fps = (self.fps_counter as f64 / el).round();
            self.fps_counter = 0;
            self.fps_time = web_time::Instant::now();
        }
        match r {
            Err(e) if e.kind == ErrorKind::Halt => {
                self.ended = true;
                Ok(())
            }
            other => other,
        }
    }

    fn step_inner(&mut self, vm: &mut Vm) -> Result<(), Error> {
        macro_rules! phase {
            ($g:expr, $name:expr, $e:expr) => {{
                if $g.prof.is_some() {
                    let t0 = web_time::Instant::now();
                    let r = $e;
                    let el = t0.elapsed().as_secs_f64();
                    *$g.prof.as_mut().unwrap().phases.entry($name).or_insert(0.0) += el;
                    r?
                } else {
                    $e?
                }
            }};
        }
        'phases: {
            // 2. special keys and close button
            phase!(self, "special_keys", self.special_keys(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 3. previous positions
            for i in self.instances.iter_mut() {
                if i.alive() {
                    i.xprevious = i.x;
                    i.yprevious = i.y;
                    i.path_positionprevious = i.path_position;
                }
            }
            // 5. begin step
            self.run_triggers(vm, 1)?;
            if self.pending.is_some() {
                break 'phases;
            }
            phase!(self, "begin step", self.broadcast(vm, ev::STEP, crate::events::step::BEGIN));
            if self.pending.is_some() {
                break 'phases;
            }
            // 5b. timelines. VERIFIED black-box (bb_motion mo.tl.*): moments run
            // after Begin Step (moment 0 of a timeline started in Create runs in
            // the first step, after the Begin Step events). The order relative to
            // alarms / keyboard / mouse / Step is not verified.
            phase!(self, "timelines", self.advance_timelines(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 6. alarms
            phase!(self, "alarms", self.run_alarms(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 7. keyboard
            phase!(self, "keyboard", self.keyboard_events(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 8. mouse
            phase!(self, "mouse", self.mouse_events(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 9. step
            self.run_triggers(vm, 0)?;
            if self.pending.is_some() {
                break 'phases;
            }
            phase!(self, "step", self.broadcast(vm, ev::STEP, crate::events::step::NORMAL));
            if self.pending.is_some() {
                break 'phases;
            }
            // 10. motion
            phase!(self, "motion", self.motion_update(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 11. boundary events
            phase!(self, "boundary", self.boundary_events(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 12. collisions
            phase!(self, "collision events", self.collision_events(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 13. end step
            self.run_triggers(vm, 2)?;
            if self.pending.is_some() {
                break 'phases;
            }
            phase!(self, "end step", self.broadcast(vm, ev::STEP, crate::events::step::END));
            if self.pending.is_some() {
                break 'phases;
            }
            // 14. remove destroyed
            self.purge_destroyed();
            // 15. particles
            self.update_particles(vm);
            // 16. views
            self.update_views();
            // 17. draw
            if self.automatic_draw {
                let t0 = web_time::Instant::now();
                phase!(self, "draw", self.draw_frame(vm));
                self.draw_time += t0.elapsed().as_secs_f64();
            }
            if self.pending.is_some() {
                break 'phases;
            }
            // 18. animation
            phase!(self, "animation", self.advance_animation(vm));
            if self.pending.is_some() {
                break 'phases;
            }
            // 19. background scrolling
            for b in self.rs.backgrounds.iter_mut() {
                b.x += b.hspeed;
                b.y += b.vspeed;
            }
        }
        self.purge_destroyed();
        // 21. pending room change / end / restart / load. A room change has
        // abandoned the rest of the step above and is performed at once
        // (runtime.md §3.2a rule 1), so the new room's Create / Room Start
        // events still see this step's input edges.
        let r = self.process_pending(vm);
        // 20. clear input edges
        self.input.end_step();
        r
    }

    /// Performs a pending room change / end / restart (§3).
    pub fn process_pending(&mut self, vm: &mut Vm) -> Result<(), Error> {
        while let Some(p) = self.pending.take() {
            self.abort_at = 0;
            match p {
                Pending::End => {
                    self.game_end(vm)?;
                    return Ok(());
                }
                Pending::Restart => {
                    // Game End for everyone, then clean slate
                    let ids: Vec<i32> = self.instances.iter().filter(|i| i.alive()).map(|i| i.id).collect();
                    for id in ids {
                        if self.has_handler_inst(id, ev::OTHER, ev_other::GAME_END) {
                            self.run_event(vm, id, ev::OTHER, ev_other::GAME_END, id)?;
                        }
                    }
                    self.instances.clear();
                    self.reindex();
                    self.snapshots.clear();
                    // VERIFIED black-box (bb_restart rs.p2.*): global variables are
                    // cleared, score/lives reset, health keeps its value.
                    vm.clear_globals();
                    self.score = 0.0;
                    self.lives = -1.0;
                    self.audio.stop_all();
                    self.room = -1;
                    self.pending = None;
                    let first = *self.room_order.first().unwrap_or(&0);
                    self.goto_room(vm, first, true)?;
                }
                Pending::Goto(r) => {
                    self.goto_room(vm, r, false)?;
                }
                Pending::Load(f) => {
                    if let Err(e) = crate::save::load_game(self, vm, &f) {
                        self.report_error(Error::new(e), 0)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// game_end(): Game End events, then stop (§3.3).
    pub fn game_end(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let ids: Vec<i32> = self.instances.iter().filter(|i| i.alive()).map(|i| i.id).collect();
        for id in ids {
            if self.has_handler_inst(id, ev::OTHER, ev_other::GAME_END) {
                self.run_event(vm, id, ev::OTHER, ev_other::GAME_END, id)?;
            }
        }
        crate::ext::run_finals(self, vm);
        self.net.end();
        self.audio.stop_all();
        self.ended = true;
        Ok(())
    }

    fn special_keys(&mut self, vm: &mut Vm) -> Result<(), Error> {
        use crate::input::vk;
        if std::mem::take(&mut self.close_requested) {
            if self.settings.treat_close_as_esc {
                self.input.apply(&InputEvent::KeyDown(vk::ESCAPE));
            } else {
                let ids: Vec<i32> = self.instances.iter().filter(|i| i.alive()).map(|i| i.id).collect();
                let mut any = false;
                for id in ids {
                    if self.has_handler_inst(id, ev::OTHER, ev_other::CLOSE_BUTTON) {
                        any = true;
                        self.run_event(vm, id, ev::OTHER, ev_other::CLOSE_BUTTON, id)?;
                    }
                }
                if !any {
                    self.pending = Some(Pending::End);
                    return Ok(());
                }
            }
        }
        if self.settings.let_esc_end_game && self.input.pressed[vk::ESCAPE as usize] {
            self.pending = Some(Pending::End);
            return Ok(());
        }
        if self.settings.let_f4_fullscreen && self.input.pressed[vk::F4 as usize] {
            self.fullscreen = !self.fullscreen;
            self.window_dirty = true;
        }
        if self.settings.let_f1_game_info && self.input.pressed[vk::F1 as usize] {
            let info = self.game_info.clone();
            self.ui.message(&crate::builtins::ui::rtf_to_text(&info));
            self.input.clear_all();
        }
        if self.settings.let_f9_screenshot && self.input.pressed[vk::F9 as usize] {
            let mut n = 0;
            loop {
                let p = std::path::Path::new(&self.working_directory).join(format!("screenshot{:03}.png", n));
                if !crate::vfs::exists(&p) {
                    let _ = self.gfx.read_back().save_png(&p, true);
                    break;
                }
                n += 1;
                if n > 999 {
                    break;
                }
            }
        }
        if self.settings.let_f5_save_f6_load {
            if self.input.pressed[vk::F5 as usize] {
                let f = std::path::Path::new(&self.working_directory).join("_save.gmd");
                if let Err(e) = crate::save::save_game(self, vm, &f.to_string_lossy()) {
                    self.report_error(Error::new(e), 0)?;
                }
            }
            if self.input.pressed[vk::F6 as usize] {
                let f = std::path::Path::new(&self.working_directory).join("_save.gmd");
                self.pending = Some(Pending::Load(f.to_string_lossy().into_owned()));
            }
        }
        Ok(())
    }

    /// The window caption (§12.1).
    pub fn caption(&self) -> String {
        let mut s = self.rs.caption.clone();
        let mut parts = Vec::new();
        if self.show_score {
            parts.push(format!("{}{}", self.caption_score, gml::real_to_string(self.score)));
        }
        if self.show_lives {
            parts.push(format!("{}{}", self.caption_lives, gml::real_to_string(self.lives)));
        }
        if self.show_health {
            parts.push(format!("{}{}", self.caption_health, gml::real_to_string(self.health)));
        }
        for p in parts {
            // single space between parts (runtime.md §3.2a rule 4)
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&p);
        }
        s
    }

    /// The current frame as RGBA (back buffer).
    pub fn frame_image(&mut self) -> Texture {
        self.gfx.read_back()
    }

    /// Requests a room change. A pending game end / restart / load is not
    /// overridden by a later room change (see OPEN_QUESTIONS [IMPL-RUN]).
    pub fn request_goto(&mut self, r: i32) {
        match self.pending {
            Some(Pending::End) | Some(Pending::Restart) | Some(Pending::Load(_)) => {}
            _ => {
                self.pending = Some(Pending::Goto(r));
                // finish the current action, skip the rest of every running event
                // (runtime.md §3.2a rule 1)
                self.abort_at = self.list_depth;
            }
        }
    }

    /// Dispatches No More Lives / No More Health raised by assignments (§5.4).
    pub fn flush_edge_events(&mut self, vm: &mut Vm) -> Result<(), Error> {
        if std::mem::take(&mut self.pending_lives_event) {
            self.broadcast_now(vm, ev::OTHER, ev_other::NO_MORE_LIVES)?;
        }
        if std::mem::take(&mut self.pending_health_event) {
            self.broadcast_now(vm, ev::OTHER, ev_other::NO_MORE_HEALTH)?;
        }
        Ok(())
    }

    /// Runs a compiled program as `self` with context; errors are returned.
    pub fn run_program(&mut self, vm: &mut Vm, prog: &Rc<Program>, ctx: Ctx, args: &[Value]) -> Result<Value, Error> {
        let p = prog.clone();
        vm.run(self, &p, ctx, args)
    }
}
