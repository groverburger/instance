//! Test-harness hooks of the headless runner: GML injected from the command
//! line (`--exec`, `--exec-file`), watched expressions (`--watch`) and the
//! start room (`--exec-room`).
//!
//! Injected code and watches run in the *harness context*: `self` (and `other`)
//! is the first live instance of the current room in instance (creation) order,
//! or the global pseudo-instance when the room has no instances (then only
//! `global.*`, `with`, functions and scripts are usable; plain instance
//! variables are errors). Code that changes a particular object should say so:
//! `with (obj_player) { x = 100; }`, `global.cash = 1e5`.

use crate::game::Game;
use gml::{Ctx, Error, ErrorKind, Program, Value, Vm};
use std::rc::Rc;

/// Code to run at the start of a step.
#[derive(Clone, Debug)]
pub struct Exec {
    /// Runs before step `step`'s events (0 = before the first step, after the
    /// first room's Create / Game Start / Room Start events).
    pub step: u64,
    /// For messages: where the code came from (`--exec` or a file name).
    pub origin: String,
    pub code: String,
}

/// An expression printed every `every` steps.
pub struct Watch {
    pub every: u64,
    pub expr: String,
    prog: Option<Rc<Program>>,
    /// The last evaluation failed (errors are printed once per failure streak).
    failing: bool,
}

impl Watch {
    pub fn new(every: u64, expr: &str) -> Self {
        Watch { every: every.max(1), expr: expr.trim().to_string(), prog: None, failing: false }
    }
}

/// Parses `STEP:TEXT` (`--exec`, `--exec-file`) or `K:EXPR` (`--watch`).
pub fn split_step(arg: &str) -> Option<(u64, &str)> {
    let (n, rest) = arg.split_once(':')?;
    Some((n.trim().parse().ok()?, rest))
}

/// A watched value: reals in full precision (shortest form that reads back
/// exactly, `62.4`, `100`), strings quoted.
pub fn show_value(v: &Value) -> String {
    match v {
        Value::Real(r) => {
            if r.fract() == 0.0 && r.abs() < 1e15 {
                format!("{}", *r as i64)
            } else {
                format!("{}", r)
            }
        }
        Value::Str(s) => format!("\"{}\"", s),
    }
}

/// An error as GM 8.1 shows it: the message, with the code excerpt and a caret
/// when the location is known.
pub fn error_text(e: &Error) -> String {
    match &e.location {
        Some(l) => gml::code_error_text(l.line, l.col, l.source_line.as_deref().unwrap_or(""), &e.message),
        None => e.message.clone(),
    }
}

impl Game {
    /// The harness context (see the module docs).
    pub fn harness_ctx(&self) -> Ctx {
        let id = self.instances.iter().find(|i| i.alive()).map(|i| i.id).unwrap_or(gml::special::GLOBAL);
        Ctx::new(id, id)
    }

    /// Runs GML code in the harness context, as `execute_string` does
    /// (compile errors come back as errors too). A `game_end` / abort sets
    /// [`Game::ended`].
    pub fn harness_exec(&mut self, vm: &mut Vm, code: &str) -> Result<Value, Error> {
        let ctx = self.harness_ctx();
        let saved = self.ev;
        self.ev = crate::events::EventCtx::default();
        let r = vm.execute_string(self, code.as_bytes(), ctx, &[]);
        self.ev = saved;
        // events triggered by assignments (lives/health), destroyed instances
        let r = r.and_then(|v| self.flush_edge_events(vm).map(|_| v));
        self.purge_destroyed();
        if let Err(e) = &r {
            if e.kind == ErrorKind::Halt {
                self.ended = true;
            }
        }
        r
    }

    /// Runs the `--exec` items due before the coming step, printing errors
    /// (`ERROR in --exec ...`) on stderr; they never stop the run.
    pub fn harness_run_execs(&mut self, vm: &mut Vm, execs: &[Exec]) {
        let step = self.frame;
        for x in execs.iter().filter(|x| x.step == step) {
            if self.ended {
                return;
            }
            if let Err(e) = self.harness_exec(vm, &x.code) {
                if e.kind != ErrorKind::Halt && e.kind != ErrorKind::AbortEvent {
                    eprintln!("ERROR in {} at step {}:\n{}", x.origin, step, error_text(&e));
                }
            }
        }
    }

    /// Evaluates an expression in the harness context without touching the
    /// game's error state (`error_occurred`, `error_last`).
    pub fn harness_eval(&mut self, vm: &mut Vm, prog: &Rc<Program>) -> Result<Value, Error> {
        let ctx = self.harness_ctx();
        let (occ, last) = (self.error_occurred, self.error_last.clone());
        let saved = self.ev;
        self.ev = crate::events::EventCtx::default();
        let r = vm.run(self, prog, ctx, &[]);
        self.ev = saved;
        self.error_occurred = occ;
        self.error_last = last;
        r
    }

    /// Prints the watches due after the step just run (`self.frame` steps
    /// done): `watch step N: EXPR = value` on stderr. A failing watch prints
    /// its error once, and again only after it has succeeded in between.
    pub fn harness_watch(&mut self, vm: &mut Vm, watches: &mut [Watch]) {
        let step = self.frame;
        for w in watches.iter_mut() {
            if !step.is_multiple_of(w.every) {
                continue;
            }
            if w.prog.is_none() {
                match vm.compile_expression("watch", w.expr.as_bytes(), &*self) {
                    Ok(p) => w.prog = Some(p),
                    Err(e) => {
                        if !w.failing {
                            eprintln!("watch step {}: {} = ERROR: {} (not printed again)", step, w.expr, e);
                            w.failing = true;
                        }
                        continue;
                    }
                }
            }
            let p = w.prog.clone().unwrap();
            match self.harness_eval(vm, &p) {
                Ok(v) => {
                    w.failing = false;
                    eprintln!("watch step {}: {} = {}", step, w.expr, show_value(&v));
                }
                Err(e) => {
                    if !w.failing {
                        w.failing = true;
                        eprintln!(
                            "watch step {}: {} = ERROR: {} (not printed again until it succeeds)",
                            step,
                            w.expr,
                            e.message.replace('\n', " ")
                        );
                    }
                }
            }
        }
    }

    /// The index of the room called `name` (case-insensitive), or a room number.
    pub fn room_by_name(&self, name: &str) -> Option<i32> {
        let n = name.trim();
        if let Some(i) = self.rooms.iter().position(|r| r.as_ref().is_some_and(|r| r.data.name.eq_ignore_ascii_case(n))) {
            return Some(i as i32);
        }
        let k: i32 = n.parse().ok()?;
        self.rooms.get(k.max(0) as usize).and_then(|r| r.as_ref()).map(|_| k)
    }
}
