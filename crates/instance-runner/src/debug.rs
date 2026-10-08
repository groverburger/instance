//! Debug mode (specs/ide.md §20.3): pause/step control, watch expressions,
//! "Execute code", the `show_debug_message` log and the inspection lists shown by
//! the Debug Information window.
//!
//! Everything here is plain game-thread logic driven through [`Command`]s, so it
//! is usable (and testable) without any window: the desktop frontend's debug
//! window only sends commands and displays [`Snapshot`]s.

use crate::Game;
use gml::{Ctx, Value, Vm};
use std::collections::VecDeque;
use std::rc::Rc;

/// Most messages kept in the log.
pub const MAX_MESSAGES: usize = 5000;

/// Where a watch or "Execute code" runs: the global context, or an instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    /// An instance id, or an object index (its first instance).
    Target(i32),
    /// An object by name (its first instance).
    Object(String),
}

impl Scope {
    /// Parses user input: empty / `global` → global; a number → instance id or
    /// object index; anything else → object name.
    pub fn parse(s: &str) -> Scope {
        let t = s.trim();
        if t.is_empty() || t.eq_ignore_ascii_case("global") {
            Scope::Global
        } else if let Ok(n) = t.parse::<i32>() {
            Scope::Target(n)
        } else {
            Scope::Object(t.to_string())
        }
    }

    /// Text form (round-trips through [`Scope::parse`]).
    pub fn label(&self) -> String {
        match self {
            Scope::Global => "global".into(),
            Scope::Target(n) => n.to_string(),
            Scope::Object(s) => s.clone(),
        }
    }
}

/// A watch expression and its last value (`Err` = error text).
#[derive(Clone, Debug)]
pub struct Watch {
    pub expr: String,
    pub scope: Scope,
    pub value: Result<String, String>,
    prog: Option<Rc<gml::Program>>,
}

/// Something the debugger front end asks the game to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Run / continue.
    Run,
    Pause,
    /// One step (pauses first when running).
    Step,
    Restart,
    End,
    AddWatch(String, Scope),
    ChangeWatch(usize, String, Scope),
    DeleteWatch(usize),
    ClearWatches,
    /// Run GML code in a scope; the outcome goes to the message log.
    Execute(String, Scope),
    /// Sets `room_speed`.
    SetSpeed(f64),
}

/// Debugger state kept by a game in debug mode.
#[derive(Default)]
pub struct Debugger {
    pub paused: bool,
    /// Single steps requested while paused.
    steps: u32,
    pub watches: Vec<Watch>,
    pub messages: VecDeque<String>,
    /// Total messages ever logged (lets a viewer fetch only new ones).
    pub message_count: u64,
}

impl Debugger {
    pub fn log(&mut self, msg: impl Into<String>) {
        if self.messages.len() >= MAX_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(msg.into());
        self.message_count += 1;
    }

    /// Messages logged after the first `seen` (of [`Debugger::message_count`]).
    pub fn messages_since(&self, seen: u64) -> Vec<String> {
        let new = self.message_count.saturating_sub(seen).min(self.messages.len() as u64) as usize;
        self.messages.iter().skip(self.messages.len() - new).cloned().collect()
    }
}

/// One row of "Show instances".
#[derive(Clone, Debug, PartialEq)]
pub struct InstanceRow {
    pub id: i32,
    pub object: String,
    pub x: f64,
    pub y: f64,
    pub depth: f64,
    pub sprite: String,
}

/// Room / timing information (status bar, "room info").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Info {
    pub fps: f64,
    pub room_speed: f64,
    pub room: String,
    pub room_index: i32,
    pub instances: usize,
    pub step: u64,
    pub paused: bool,
    pub ended: bool,
    /// Id of the instance under the mouse (-4 = none).
    pub under_mouse: i32,
    pub mouse: (f64, f64),
}

/// Which of the (more expensive) lists a viewer wants in snapshots.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wants {
    pub globals: bool,
    pub instances: bool,
    /// Local variables of an instance.
    pub locals: Option<Scope>,
}

/// An instance id and its variables (Err when a scope names no instance).
pub type Locals = Result<(i32, Vec<(String, String)>), String>;

/// Everything a debug window shows, taken between steps.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub info: Info,
    /// (expression, scope label, value or error)
    pub watches: Vec<(String, String, Result<String, String>)>,
    pub globals: Vec<(String, String)>,
    pub instances: Vec<InstanceRow>,
    /// Instance id and its variables (Err when the scope names no instance).
    pub locals: Option<Locals>,
}

/// Value as shown in the debugger (strings quoted).
pub fn show_value(v: &Value) -> String {
    match v {
        Value::Real(_) => v.to_string(),
        Value::Str(s) => format!("\"{}\"", s),
    }
}

fn show_var(v: &gml::Var) -> String {
    match v {
        gml::Var::Scalar(x) => show_value(x),
        gml::Var::Array(a) => {
            let rows = a.rows();
            let items: Vec<String> = rows.first().map(|r| r.iter().take(8).map(show_value).collect()).unwrap_or_default();
            let n: usize = rows.iter().map(|r| r.len()).sum();
            let more = if n > items.len() { ", …" } else { "" };
            if rows.len() > 1 {
                format!("array {}x… [{}{}]", rows.len(), items.join(", "), more)
            } else {
                format!("array [{}{}]", items.join(", "), more)
            }
        }
    }
}

impl Game {
    /// Turns debug mode on (`debug_mode` reads true, messages are logged).
    pub fn enable_debug(&mut self) {
        if self.debug.is_none() {
            self.debug = Some(Box::default());
        }
    }

    pub fn debug_mode(&self) -> bool {
        self.debug.is_some()
    }

    /// `show_debug_message`: printed on the console, and logged in debug mode.
    pub fn debug_message(&mut self, msg: &str) {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&msg.into());
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("[debug] {}", msg);
        if let Some(d) = &mut self.debug {
            d.log(msg);
        }
    }

    /// Whether the frontend should run a step now (false while paused, unless a
    /// single step was requested or the window is being closed).
    pub fn debug_should_step(&mut self) -> bool {
        match &mut self.debug {
            None => true,
            Some(d) if !d.paused || self.close_requested || self.ended => true,
            Some(d) if d.steps > 0 => {
                d.steps -= 1;
                true
            }
            Some(_) => false,
        }
    }

    pub fn debug_paused(&self) -> bool {
        self.debug.as_ref().is_some_and(|d| d.paused)
    }

    /// The instance a scope designates (`None` = global context).
    fn scope_instance(&self, s: &Scope) -> Result<Option<i32>, String> {
        match s {
            Scope::Global => Ok(None),
            Scope::Target(t) => self.first_of(*t).map(Some).ok_or_else(|| format!("no instance {}", t)),
            Scope::Object(name) => {
                let obj = self
                    .objects
                    .iter()
                    .position(|o| o.as_ref().is_some_and(|o| o.name.eq_ignore_ascii_case(name)))
                    .ok_or_else(|| format!("unknown object {}", name))?;
                self.first_of(obj as i32).map(Some).ok_or_else(|| format!("no instance of {}", name))
            }
        }
    }

    fn scope_ctx(&self, s: &Scope) -> Result<Ctx, String> {
        let id = self.scope_instance(s)?.unwrap_or(gml::special::GLOBAL);
        Ok(Ctx::new(id, id))
    }

    /// Evaluates an expression in a scope.
    pub fn debug_eval(&mut self, vm: &mut Vm, expr: &str, scope: &Scope) -> Result<String, String> {
        let prog = vm.compile_expression("watch", expr.as_bytes(), &*self).map_err(|e| e.to_string())?;
        self.debug_run(vm, &prog, scope)
    }

    fn debug_run(&mut self, vm: &mut Vm, prog: &Rc<gml::Program>, scope: &Scope) -> Result<String, String> {
        let ctx = self.scope_ctx(scope)?;
        // errors here belong to the debugger, not to the game
        let (occ, last) = (self.error_occurred, self.error_last.clone());
        let r = vm.run(self, prog, ctx, &[]).map(|v| show_value(&v)).map_err(|e| e.to_string());
        self.error_occurred = occ;
        self.error_last = last;
        r
    }

    /// Runs GML code ("Execute code"); returns its `return` value.
    pub fn debug_execute(&mut self, vm: &mut Vm, code: &str, scope: &Scope) -> Result<String, String> {
        let ctx = self.scope_ctx(scope)?;
        let r = vm.execute_string(self, code.as_bytes(), ctx, &[]).map(|v| show_value(&v)).map_err(|e| e.to_string());
        // instances created/destroyed by the code
        self.purge_destroyed();
        r
    }

    /// Re-evaluates all watches (after each step).
    pub fn debug_update_watches(&mut self, vm: &mut Vm) {
        let Some(d) = &mut self.debug else { return };
        let mut ws = std::mem::take(&mut d.watches);
        for w in &mut ws {
            if w.prog.is_none() {
                match vm.compile_expression("watch", w.expr.as_bytes(), &*self) {
                    Ok(p) => w.prog = Some(p),
                    Err(e) => {
                        w.value = Err(e.to_string());
                        continue;
                    }
                }
            }
            let p = w.prog.clone().unwrap();
            w.value = self.debug_run(vm, &p, &w.scope);
        }
        if let Some(d) = &mut self.debug {
            // (a command run by a watch cannot have added watches: keep ours)
            d.watches = ws;
        }
    }

    /// Carries out a debugger command.
    pub fn debug_command(&mut self, vm: &mut Vm, cmd: Command) {
        self.enable_debug();
        let watch = |expr: String, scope: Scope| Watch { expr, scope, value: Ok(String::new()), prog: None };
        let mut reeval = false;
        let d = self.debug.as_mut().unwrap();
        match cmd {
            Command::Run => d.paused = false,
            Command::Pause => d.paused = true,
            Command::Step => {
                if d.paused {
                    d.steps += 1;
                } else {
                    d.paused = true;
                }
            }
            Command::AddWatch(e, s) => {
                if !e.trim().is_empty() {
                    d.watches.push(watch(e, s));
                    reeval = true;
                }
            }
            Command::ChangeWatch(i, e, s) => {
                if let Some(w) = d.watches.get_mut(i) {
                    *w = watch(e, s);
                    reeval = true;
                }
            }
            Command::DeleteWatch(i) => {
                if i < d.watches.len() {
                    d.watches.remove(i);
                }
            }
            Command::ClearWatches => d.watches.clear(),
            Command::SetSpeed(s) => {
                if s >= 1.0 {
                    self.rs.speed = s;
                }
            }
            Command::Execute(code, scope) => {
                let r = self.debug_execute(vm, &code, &scope);
                let d = self.debug.as_mut().unwrap();
                match r {
                    Ok(v) => d.log(format!("> {}  [{}] = {}", code.trim(), scope.label(), v)),
                    Err(e) => d.log(format!("> {}  [{}] error: {}", code.trim(), scope.label(), e)),
                }
                reeval = true;
            }
            Command::Restart => {
                self.pending = Some(crate::Pending::Restart);
                if let Err(e) = self.process_pending(vm) {
                    if let Some(d) = &mut self.debug {
                        d.log(format!("error restarting: {}", e));
                    }
                }
                reeval = true;
            }
            Command::End => {
                if !self.ended {
                    if let Err(e) = self.game_end(vm) {
                        eprintln!("instance-runner: {}", e);
                    }
                    self.ended = true;
                }
            }
        }
        if reeval {
            self.debug_update_watches(vm);
        }
    }

    pub fn debug_info(&mut self) -> Info {
        let (rx, ry) = self.debug_mouse_room();
        let under = self.instance_at_point(rx, ry, crate::collision::Sel::All).unwrap_or(gml::special::NOONE);
        Info {
            fps: self.fps,
            room_speed: self.rs.speed,
            room: self.rooms.get(self.room.max(0) as usize).and_then(|r| r.as_ref()).map(|r| r.data.name.clone()).unwrap_or_default(),
            room_index: self.room,
            instances: self.instances.iter().filter(|i| i.alive()).count(),
            step: self.frame,
            paused: self.debug_paused(),
            ended: self.ended,
            under_mouse: under,
            mouse: (rx, ry),
        }
    }

    /// `mouse_x`, `mouse_y` (room coordinates).
    fn debug_mouse_room(&mut self) -> (f64, f64) {
        let ctx = Ctx::new(gml::special::GLOBAL, gml::special::GLOBAL);
        let get = |g: &mut Game, n: &str| {
            gml::builtins::variable_id(n)
                .and_then(|v| gml::Host::get_builtin(g, &ctx, gml::special::GLOBAL, v, 0).ok())
                .map(|v| v.real_or_zero())
                .unwrap_or(0.0)
        };
        (get(self, "mouse_x"), get(self, "mouse_y"))
    }

    /// "Show global variables": user globals, then the main built-in ones.
    pub fn debug_globals(&mut self, vm: &Vm) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> =
            vm.globals().iter().map(|(id, v)| (format!("global.{}", vm.var_name(id)), show_var(v))).collect();
        out.sort();
        let ctx = Ctx::new(gml::special::GLOBAL, gml::special::GLOBAL);
        for n in [
            "room", "room_speed", "room_width", "room_height", "room_caption", "score", "lives", "health", "fps",
            "instance_count", "mouse_x", "mouse_y", "keyboard_key", "keyboard_lastchar", "error_occurred",
            "error_last", "working_directory",
        ] {
            if let Some(v) = gml::builtins::variable_id(n) {
                let s = gml::Host::get_builtin(self, &ctx, gml::special::GLOBAL, v, 0).map(|x| show_value(&x)).unwrap_or_else(|e| e.message);
                out.push((n.to_string(), s));
            }
        }
        out
    }

    /// "Show local variables": built-in and user variables of an instance.
    pub fn debug_locals(&mut self, vm: &Vm, scope: &Scope) -> Locals {
        let id = self.scope_instance(scope)?.ok_or("choose an instance or object")?;
        let ctx = Ctx::new(id, id);
        let mut out = Vec::new();
        for (k, info) in gml::builtins::VARIABLES.iter().enumerate() {
            if info.scope != gml::builtins::VarScope::Instance || info.array.is_some() {
                continue;
            }
            let var = gml::builtins::VariableId(k as u16);
            if let Ok(v) = gml::Host::get_builtin(self, &ctx, id, var, 0) {
                out.push((info.name.to_string(), show_value(&v)));
            }
        }
        if let Some(i) = self.inst(id) {
            for (k, a) in i.alarm.iter().enumerate() {
                out.push((format!("alarm[{}]", k), Value::Real(*a).to_string()));
            }
            let mut user: Vec<(String, String)> = i.vars.iter().map(|(vid, v)| (vm.var_name(vid).to_string(), show_var(v))).collect();
            user.sort();
            out.extend(user);
        }
        Ok((id, out))
    }

    /// "Show instances".
    pub fn debug_instances(&self) -> Vec<InstanceRow> {
        self.instances
            .iter()
            .filter(|i| i.alive())
            .map(|i| InstanceRow {
                id: i.id,
                object: self.object(i.object_index).map(|o| o.name.clone()).unwrap_or_else(|| i.object_index.to_string()),
                x: i.x,
                y: i.y,
                depth: i.depth,
                sprite: self.sprite(i.sprite_index).map(|s| s.name.clone()).unwrap_or_else(|| i.sprite_index.to_string()),
            })
            .collect()
    }

    /// Everything a debug window shows.
    pub fn debug_snapshot(&mut self, vm: &Vm, wants: &Wants) -> Snapshot {
        let watches = self
            .debug
            .as_ref()
            .map(|d| d.watches.iter().map(|w| (w.expr.clone(), w.scope.label(), w.value.clone())).collect())
            .unwrap_or_default();
        Snapshot {
            info: self.debug_info(),
            watches,
            globals: if wants.globals { self.debug_globals(vm) } else { Vec::new() },
            instances: if wants.instances { self.debug_instances() } else { Vec::new() },
            locals: wants.locals.as_ref().map(|s| self.debug_locals(vm, s)),
        }
    }
}
