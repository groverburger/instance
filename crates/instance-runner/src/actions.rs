//! Drag-and-drop action lists: compilation into a small tree (blocks, questions,
//! else, repeat, exit) and execution (actions.json; file-formats.md §5.9;
//! gml-language.md §11).

use crate::game::Game;
use instance_format::{action_kind, arg_kind, exe_type, Action};
use gml::{Ctx, Error, ErrorKind, FunctionId, Program, Value, Vm};
use std::rc::Rc;

/// Signature of a runner-implemented `action_*` function. The last argument is
/// the action's "relative" flag.
pub type ActionFn = fn(&mut Game, &mut Vm, &Ctx, &[Value], bool) -> Result<Value, Error>;

/// An argument of an action.
pub enum Arg {
    Val(Value),
    Expr(Rc<Program>),
    Bad(String),
}

/// What an action does.
pub enum ActKind {
    Code(Rc<Program>),
    Func(ActionFn),
    Builtin(FunctionId),
    /// Code that failed to compile or an unknown function: raises this error when run.
    Error(String),
    Nop,
}

/// One executable action.
pub struct Act {
    pub idx: i32,
    pub kind: ActKind,
    pub applies: i32,
    pub relative: bool,
    pub negate: bool,
    pub question: bool,
    pub args: Vec<Arg>,
}

/// A node of the structured action list.
pub enum Node {
    Act(Box<Act>),
    If {
        cond: Box<Act>,
        then: Box<Node>,
        els: Option<Box<Node>>,
    },
    Repeat {
        idx: i32,
        count: Arg,
        body: Box<Node>,
    },
    Block(Vec<Node>),
    /// Another (shared) list, run inline (used when code is appended at run time).
    Shared(Rc<ActionList>),
    Exit,
    Nop,
}

/// A compiled event / moment action list.
pub struct ActionList {
    pub nodes: Vec<Node>,
}

/// Control flow out of a node.
#[derive(PartialEq, Eq)]
enum Flow {
    Next,
    Exit,
}

/// Truth value of a GML value for questions (`>= 0.5`; strings false).
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Real(r) => *r >= 0.5,
        Value::Str(_) => false,
    }
}

fn compile_arg(vm: &mut Vm, g: &Game, kind: i32, text: &str, where_: &str) -> Arg {
    let expr = |vm: &mut Vm, text: &str| -> Arg {
        let t = if text.trim().is_empty() { "0" } else { text };
        match vm.compile_expression(where_, t.as_bytes(), g) {
            Ok(p) => Arg::Expr(p),
            Err(e) => Arg::Bad(format!("COMPILATION ERROR in argument of action ({}): {}", where_, e)),
        }
    };
    match kind {
        arg_kind::EXPRESSION => {
            // fast path for plain numbers
            if let Ok(v) = text.trim().parse::<f64>() {
                Arg::Val(Value::Real(v))
            } else {
                expr(vm, text)
            }
        }
        arg_kind::STRING | arg_kind::FONT_STRING => Arg::Val(Value::Str(instance_format::text::encode_cp1252(text).into())),
        arg_kind::BOTH => {
            let t = text.trim_start();
            if t.starts_with('"') || t.starts_with('\'') {
                expr(vm, text)
            } else {
                Arg::Val(Value::Str(instance_format::text::encode_cp1252(text).into()))
            }
        }
        _ => match text.trim().parse::<f64>() {
            Ok(v) => Arg::Val(Value::Real(v)),
            Err(_) => expr(vm, text),
        },
    }
}

fn compile_act(vm: &mut Vm, g: &Game, a: &Action, idx: i32, where_: &str, errors: &mut Vec<String>) -> Act {
    let nargs = (a.argument_count.max(0) as usize).min(a.args.len());
    let kind_of = |i: usize| a.arg_kinds.get(i).copied().unwrap_or(0);
    let mut act = Act {
        idx,
        kind: ActKind::Nop,
        applies: if a.has_applies_to { a.applies_to } else { gml::special::SELF },
        relative: a.relative && a.may_be_relative,
        negate: a.negate,
        question: a.is_question,
        args: Vec::new(),
    };
    let w = format!("action {} of {}", idx + 1, where_);
    match a.exe_type {
        exe_type::CODE if a.kind == action_kind::CODE => {
            let code = a.args.first().map(|s| s.as_str()).unwrap_or("");
            match vm.compile(&w, code.as_bytes(), g) {
                Ok(p) => act.kind = ActKind::Code(p),
                Err(e) => {
                    let m = format!("COMPILATION ERROR in code action ({})\n{}", w, e);
                    errors.push(m.clone());
                    act.kind = ActKind::Error(m);
                }
            }
        }
        exe_type::CODE if a.kind == action_kind::VARIABLE => {
            let name = a.args.first().map(|s| s.trim()).unwrap_or("");
            let val = a.args.get(1).map(|s| s.as_str()).unwrap_or("0");
            let op = if a.relative { "+=" } else { "=" };
            let src = format!("{} {} ({});", name, op, if val.trim().is_empty() { "0" } else { val });
            match vm.compile(&w, src.as_bytes(), g) {
                Ok(p) => act.kind = ActKind::Code(p),
                Err(e) => {
                    let m = format!("COMPILATION ERROR in Set Variable action ({})\n{}", w, e);
                    errors.push(m.clone());
                    act.kind = ActKind::Error(m);
                }
            }
            act.relative = false;
        }
        exe_type::CODE => {
            // a library action with a GML body (user libraries)
            match vm.compile(&w, a.code.as_bytes(), g) {
                Ok(p) => act.kind = ActKind::Code(p),
                Err(e) => act.kind = ActKind::Error(e.to_string()),
            }
            for i in 0..nargs {
                act.args.push(compile_arg(vm, g, kind_of(i), &a.args[i], &w));
            }
        }
        exe_type::FUNCTION => {
            let name = a.function_name.as_str();
            if let Some(f) = crate::action_lib::lookup(name) {
                act.kind = ActKind::Func(f);
            } else if let Some(fid) = gml::builtins::function_id(name) {
                act.kind = ActKind::Builtin(fid);
            } else if let Some(&s) = g.script_names.get(name) {
                // user library function implemented as a script
                act.kind = ActKind::Error(format!("script:{}", s));
            } else {
                let m = format!("Unknown action function {} ({})", name, w);
                errors.push(m.clone());
                act.kind = ActKind::Error(m);
            }
            for i in 0..nargs {
                act.args.push(compile_arg(vm, g, kind_of(i), &a.args[i], &w));
            }
        }
        _ => act.kind = ActKind::Nop,
    }
    act
}

/// Compiles a flat action list into a structured [`ActionList`].
pub fn compile_list(vm: &mut Vm, g: &Game, actions: &[Action], where_: &str, errors: &mut Vec<String>) -> ActionList {
    let mut pos = 0;
    let mut nodes = Vec::new();
    while pos < actions.len() {
        let (n, next) = parse_node(vm, g, actions, pos, where_, errors);
        nodes.push(n);
        pos = next;
    }
    ActionList { nodes }
}

fn parse_node(
    vm: &mut Vm,
    g: &Game,
    acts: &[Action],
    pos: usize,
    where_: &str,
    errors: &mut Vec<String>,
) -> (Node, usize) {
    if pos >= acts.len() {
        return (Node::Nop, pos);
    }
    let a = &acts[pos];
    match a.kind {
        action_kind::BEGIN_GROUP => {
            let mut p = pos + 1;
            let mut body = Vec::new();
            while p < acts.len() && acts[p].kind != action_kind::END_GROUP {
                let (n, next) = parse_node(vm, g, acts, p, where_, errors);
                body.push(n);
                p = next;
            }
            (Node::Block(body), (p + 1).min(acts.len().max(p + 1)))
        }
        action_kind::END_GROUP | action_kind::ELSE => (Node::Nop, pos + 1),
        action_kind::EXIT => (Node::Exit, pos + 1),
        action_kind::REPEAT => {
            let count = compile_arg(
                vm,
                g,
                arg_kind::EXPRESSION,
                a.args.first().map(|s| s.as_str()).unwrap_or("1"),
                where_,
            );
            let (body, next) = parse_node(vm, g, acts, pos + 1, where_, errors);
            (
                Node::Repeat {
                    idx: pos as i32,
                    count,
                    body: Box::new(body),
                },
                next,
            )
        }
        _ => {
            let act = compile_act(vm, g, a, pos as i32, where_, errors);
            if act.question {
                let (then, mut next) = parse_node(vm, g, acts, pos + 1, where_, errors);
                let mut els = None;
                if next < acts.len() && acts[next].kind == action_kind::ELSE {
                    let (e, n2) = parse_node(vm, g, acts, next + 1, where_, errors);
                    els = Some(Box::new(e));
                    next = n2;
                }
                (
                    Node::If {
                        cond: Box::new(act),
                        then: Box::new(then),
                        els,
                    },
                    next,
                )
            } else if matches!(act.kind, ActKind::Nop) {
                (Node::Nop, pos + 1)
            } else {
                (Node::Act(Box::new(act)), pos + 1)
            }
        }
    }
}

/// Runs an action list for instance `self_id`.
pub fn execute(g: &mut Game, vm: &mut Vm, list: &ActionList, self_id: i32, other: i32) -> Result<(), Error> {
    g.list_depth += 1;
    let mut r = Ok(());
    for n in &list.nodes {
        match exec_node(g, vm, n, self_id, other) {
            Ok(Flow::Next) => {}
            Ok(Flow::Exit) => break,
            // GM 8.1 abandons the rest of the event silently (bb_lang args.arr_index_16)
            Err(e) if e.kind == ErrorKind::AbortEvent => break,
            Err(e) => {
                r = Err(e);
                break;
            }
        }
    }
    g.list_depth -= 1;
    // lists started from now on at this depth were not running at the request
    g.abort_at = g.abort_at.min(g.list_depth);
    r
}

/// Whether a room change requested while this list was running means its
/// remaining actions are skipped (runtime.md §3.2a rule 1).
#[inline]
fn aborted(g: &Game) -> bool {
    g.abort_at >= g.list_depth
}

fn exec_node(g: &mut Game, vm: &mut Vm, n: &Node, self_id: i32, other: i32) -> Result<Flow, Error> {
    match n {
        Node::Nop => Ok(Flow::Next),
        Node::Exit => Ok(Flow::Exit),
        Node::Block(b) => {
            for c in b {
                if exec_node(g, vm, c, self_id, other)? == Flow::Exit {
                    return Ok(Flow::Exit);
                }
            }
            Ok(Flow::Next)
        }
        Node::Act(a) => {
            let r = run_act(g, vm, a, self_id, other);
            r?;
            Ok(if aborted(g) { Flow::Exit } else { Flow::Next })
        }
        Node::Shared(l) => {
            for c in &l.nodes {
                if exec_node(g, vm, c, self_id, other)? == Flow::Exit {
                    return Ok(Flow::Exit);
                }
            }
            Ok(Flow::Next)
        }
        Node::If { cond, then, els } => {
            let r = run_act(g, vm, cond, self_id, other)?;
            if aborted(g) {
                return Ok(Flow::Exit);
            }
            if r {
                exec_node(g, vm, then, self_id, other)
            } else if let Some(e) = els {
                exec_node(g, vm, e, self_id, other)
            } else {
                Ok(Flow::Next)
            }
        }
        Node::Repeat { idx, count, body } => {
            g.ev.action = *idx;
            let c = match eval_arg(g, vm, count, Ctx::new(self_id, other)) {
                Ok(v) => match v {
                    Value::Real(r) => gml::to_int(r),
                    _ => 0,
                },
                Err(e) => {
                    g.report_error(e, *idx)?;
                    0
                }
            };
            for _ in 0..c.max(0) {
                if exec_node(g, vm, body, self_id, other)? == Flow::Exit {
                    return Ok(Flow::Exit);
                }
            }
            Ok(Flow::Next)
        }
    }
}

fn eval_arg(g: &mut Game, vm: &mut Vm, a: &Arg, ctx: Ctx) -> Result<Value, Error> {
    match a {
        Arg::Val(v) => Ok(v.clone()),
        Arg::Expr(p) => {
            let p = p.clone();
            vm.run(g, &p, ctx, &[])
        }
        Arg::Bad(m) => Err(Error::new(m.clone())),
    }
}

/// Runs one action on its applies-to targets. For questions returns the result
/// (true only if true for every target; false with no target).
fn run_act(g: &mut Game, vm: &mut Vm, a: &Act, self_id: i32, other: i32) -> Result<bool, Error> {
    let targets: Vec<(i32, i32)> = match a.applies {
        gml::special::SELF => vec![(self_id, other)],
        gml::special::OTHER => {
            if g.inst(other).is_some() {
                vec![(other, self_id)]
            } else {
                vec![]
            }
        }
        obj => {
            let mut v = Vec::new();
            g.select(obj, &mut v);
            v.into_iter().map(|t| (t, self_id)).collect()
        }
    };
    let mut result = !targets.is_empty();
    for (s, o) in targets {
        g.ev.action = a.idx;
        let ctx = Ctx {
            self_id: s,
            other_id: o,
            argument_relative: a.relative,
        };
        let r = (|| -> Result<Value, Error> {
            let mut args = Vec::with_capacity(a.args.len());
            for x in &a.args {
                args.push(eval_arg(g, vm, x, Ctx::new(s, o))?);
            }
            match &a.kind {
                // code actions leave the global argument array alone (GM 8.1: they see
                // whatever it holds, bb_lang args.code_action_*)
                ActKind::Code(p) => {
                    let p = p.clone();
                    vm.run_code(g, &p, ctx)
                }
                ActKind::Func(f) => f(g, vm, &ctx, &args, a.relative),
                ActKind::Builtin(fid) => vm.call_function(g, &ctx, *fid, &args),
                ActKind::Error(m) => {
                    if let Some(s) = m.strip_prefix("script:") {
                        let idx: i32 = s.parse().unwrap_or(-1);
                        let pr = g.scripts.get(idx as usize).and_then(|x| x.as_ref()).and_then(|x| x.prog.clone());
                        match pr {
                            Some(p) => vm.run(g, &p, ctx, &args),
                            None => Err(Error::new("Script does not exist")),
                        }
                    } else {
                        Err(Error::new(m.clone()))
                    }
                }
                ActKind::Nop => Ok(Value::Real(0.0)),
            }
        })();
        if g.pending_lives_event || g.pending_health_event {
            g.flush_edge_events(vm)?;
        }
        match r {
            Ok(v) => {
                if a.question {
                    let t = truthy(&v) != a.negate;
                    result &= t;
                }
            }
            Err(e) => {
                if e.kind == ErrorKind::Halt || e.kind == ErrorKind::AbortEvent {
                    return Err(e);
                }
                // the current action number, which a nested event may have changed
                // (see Game::run_handler)
                let no = if g.ev.action >= 0 { g.ev.action } else { a.idx };
                g.report_error(e, no)?;
                if a.question {
                    result = false;
                }
            }
        }
    }
    Ok(result)
}
