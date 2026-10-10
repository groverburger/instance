//! Enumerates every piece of GML in a project, with a human-readable location.

use crate::names::event_name;
use instance_format::{action_kind, arg_kind, exe_type, Action, Project};

/// What a piece of code belongs to (used by the variable analyses).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Script(usize),
    Object {
        obj: usize,
        ev_type: i32,
        ev_num: i32,
    },
    Timeline,
    Room,
    /// Instance creation code of a placed instance of `object`.
    Instance {
        object: i32,
    },
    Trigger,
    Constant,
}

/// Whether the text is a program (statements) or a single expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Program,
    Expression,
}

/// One piece of GML.
#[derive(Clone, Debug)]
pub struct Piece {
    pub resource: String,
    pub event: Option<String>,
    pub action: Option<usize>,
    /// Extra location detail (`argument 2`, `Set Variable`).
    pub note: Option<String>,
    pub owner: Owner,
    pub form: Form,
    pub src: String,
}

/// An action list with its location.
pub struct ActionList<'a> {
    pub resource: String,
    pub event: String,
    pub owner: Owner,
    pub actions: &'a [Action],
}

/// Every action list of the project: object events, then time-line moments.
pub fn action_lists(p: &Project) -> Vec<ActionList<'_>> {
    let mut out = Vec::new();
    for (oi, o) in p.objects.iter().enumerate() {
        let Some(o) = o else { continue };
        for e in &o.events {
            out.push(ActionList {
                resource: o.name.clone(),
                event: event_name(p, e.event_type, e.number),
                owner: Owner::Object {
                    obj: oi,
                    ev_type: e.event_type,
                    ev_num: e.number,
                },
                actions: &e.actions,
            });
        }
    }
    for t in p.timelines.iter().flatten() {
        for m in &t.moments {
            out.push(ActionList {
                resource: t.name.clone(),
                event: format!("moment {}", m.step),
                owner: Owner::Timeline,
                actions: &m.actions,
            });
        }
    }
    out
}

/// Whether the runner compiles an action argument of this kind and text as GML
/// (mirrors `instance-runner/src/actions.rs` `compile_arg`).
fn arg_is_expression(kind: i32, text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.parse::<f64>().is_ok() {
        return false;
    }
    match kind {
        arg_kind::STRING | arg_kind::FONT_STRING => false,
        arg_kind::BOTH => t.starts_with('"') || t.starts_with('\''),
        _ => true,
    }
}

fn action_pieces(list: &ActionList, out: &mut Vec<Piece>) {
    for (i, a) in list.actions.iter().enumerate() {
        let piece = |note: Option<String>, form: Form, src: String| Piece {
            resource: list.resource.clone(),
            event: Some(list.event.clone()),
            action: Some(i + 1),
            note,
            owner: list.owner,
            form,
            src,
        };
        match a.exe_type {
            exe_type::CODE if a.kind == action_kind::CODE => {
                out.push(piece(
                    None,
                    Form::Program,
                    a.args.first().cloned().unwrap_or_default(),
                ));
            }
            exe_type::CODE if a.kind == action_kind::VARIABLE => {
                // Compiled the way the runner does: `name = (value);` / `name += (value);`
                let name = a.args.first().map(|s| s.trim()).unwrap_or("");
                let val = a.args.get(1).map(|s| s.as_str()).unwrap_or("0");
                let val = if val.trim().is_empty() { "0" } else { val };
                let op = if a.relative { "+=" } else { "=" };
                out.push(piece(
                    Some("Set Variable".into()),
                    Form::Program,
                    format!("{} {} ({});", name, op, val),
                ));
            }
            exe_type::CODE | exe_type::FUNCTION => {
                let n = (a.argument_count.max(0) as usize).min(a.args.len());
                for (k, text) in a.args.iter().take(n).enumerate() {
                    let kind = a.arg_kinds.get(k).copied().unwrap_or(0);
                    if arg_is_expression(kind, text) {
                        out.push(piece(
                            Some(format!("argument {}", k + 1)),
                            Form::Expression,
                            text.clone(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Every code piece: scripts, user constants, trigger conditions, actions (code,
/// Set Variable, expression arguments) of objects and time lines, room and instance
/// creation code.
pub fn collect(p: &Project) -> Vec<Piece> {
    let mut out = Vec::new();
    for c in &p.constants {
        out.push(Piece {
            resource: format!("constant {}", c.name),
            event: None,
            action: None,
            note: None,
            owner: Owner::Constant,
            form: Form::Expression,
            src: if c.value.trim().is_empty() {
                "0".into()
            } else {
                c.value.clone()
            },
        });
    }
    for (i, s) in p.scripts.iter().enumerate() {
        let Some(s) = s else { continue };
        out.push(Piece {
            resource: s.name.clone(),
            event: None,
            action: None,
            note: None,
            owner: Owner::Script(i),
            form: Form::Program,
            src: s.code.clone(),
        });
    }
    for t in p.triggers.iter().flatten() {
        out.push(Piece {
            resource: format!("trigger {}", t.name),
            event: Some("condition".into()),
            action: None,
            note: None,
            owner: Owner::Trigger,
            form: Form::Program,
            src: t.condition.clone(),
        });
    }
    for list in action_lists(p) {
        action_pieces(&list, &mut out);
    }
    for r in p.rooms.iter().flatten() {
        if !r.creation_code.trim().is_empty() {
            out.push(Piece {
                resource: r.name.clone(),
                event: Some("creation code".into()),
                action: None,
                note: None,
                owner: Owner::Room,
                form: Form::Program,
                src: r.creation_code.clone(),
            });
        }
        for inst in &r.instances {
            if inst.creation_code.trim().is_empty() {
                continue;
            }
            let oname = usize::try_from(inst.object)
                .ok()
                .and_then(|i| p.objects.get(i))
                .and_then(|o| o.as_ref())
                .map(|o| o.name.clone())
                .unwrap_or_else(|| format!("object {}", inst.object));
            out.push(Piece {
                resource: r.name.clone(),
                event: Some(format!("instance {} ({}) creation code", inst.id, oname)),
                action: None,
                note: None,
                owner: Owner::Instance {
                    object: inst.object,
                },
                form: Form::Program,
                src: inst.creation_code.clone(),
            });
        }
    }
    out
}
