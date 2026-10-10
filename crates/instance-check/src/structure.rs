//! Structural checks: resource references, instance ids, names (AGENT_GUIDE §2,
//! §3.3, §4.1 checklist, gotchas 9 and 10).

use crate::pieces::action_lists;
use crate::{Diagnostic, Options, Severity};
use instance_format::{arg_kind, event_type, Project, NO_PARENT};
use std::collections::{HashMap, HashSet};

fn d(
    sev: Severity,
    kind: &'static str,
    resource: &str,
    event: Option<String>,
    message: String,
) -> Diagnostic {
    Diagnostic {
        severity: sev,
        kind,
        resource: resource.to_string(),
        event,
        action: None,
        line: None,
        col: None,
        message,
        source_line: None,
    }
}

fn exists<T>(list: &[Option<T>], idx: i32) -> bool {
    usize::try_from(idx)
        .ok()
        .and_then(|i| list.get(i))
        .is_some_and(|x| x.is_some())
}

/// Describes a bad index: "does not exist" / "is a deleted slot".
fn why<T>(list: &[Option<T>], idx: i32) -> &'static str {
    match usize::try_from(idx).ok().and_then(|i| list.get(i)) {
        Some(None) => "a deleted slot",
        _ => "out of range",
    }
}

/// Whether `s` can be written as a GML identifier.
pub fn is_identifier(s: &str) -> bool {
    let mut c = s.bytes();
    match c.next() {
        Some(b) if b.is_ascii_alphabetic() || b == b'_' => {}
        _ => return false,
    }
    c.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn check(p: &Project, opts: &Options, out: &mut Vec<Diagnostic>) {
    use Severity::*;
    // ---------------------------------------------------------------- objects
    for (oi, o) in p.objects.iter().enumerate() {
        let Some(o) = o else { continue };
        let r = o.name.as_str();
        if o.sprite != -1 && !exists(&p.sprites, o.sprite) {
            out.push(d(
                Error,
                "reference",
                r,
                None,
                format!("sprite index {} is {}", o.sprite, why(&p.sprites, o.sprite)),
            ));
        }
        if o.mask_sprite != -1 && !exists(&p.sprites, o.mask_sprite) {
            out.push(d(
                Error,
                "reference",
                r,
                None,
                format!(
                    "mask sprite index {} is {}",
                    o.mask_sprite,
                    why(&p.sprites, o.mask_sprite)
                ),
            ));
        }
        if o.parent != NO_PARENT && o.parent != -1 {
            if !exists(&p.objects, o.parent) {
                out.push(d(
                    Error,
                    "reference",
                    r,
                    None,
                    format!(
                        "parent object index {} is {}",
                        o.parent,
                        why(&p.objects, o.parent)
                    ),
                ));
            } else {
                // cycle?
                let mut seen = HashSet::new();
                let mut cur = oi as i32;
                while cur >= 0 && exists(&p.objects, cur) {
                    if !seen.insert(cur) {
                        out.push(d(
                            Error,
                            "reference",
                            r,
                            None,
                            "the parent chain contains a cycle".into(),
                        ));
                        break;
                    }
                    cur = p.objects[cur as usize].as_ref().unwrap().parent;
                }
            }
        }
        let mut seen_ev = HashSet::new();
        for e in &o.events {
            if !seen_ev.insert((e.event_type, e.number)) {
                out.push(d(
                    Error,
                    "event",
                    r,
                    Some(crate::names::event_name(p, e.event_type, e.number)),
                    "event defined twice".into(),
                ));
            }
            if !(0..=event_type::MAX).contains(&e.event_type) {
                out.push(d(
                    Error,
                    "event",
                    r,
                    None,
                    format!("unknown event type {}", e.event_type),
                ));
            }
            if e.event_type == event_type::COLLISION && !exists(&p.objects, e.number) {
                out.push(d(
                    Error,
                    "reference",
                    r,
                    Some(format!("Collision with object {}", e.number)),
                    format!(
                        "collision event with object index {}, which is {}",
                        e.number,
                        why(&p.objects, e.number)
                    ),
                ));
            }
            if e.event_type == event_type::TRIGGER && !exists(&p.triggers, e.number) {
                out.push(d(
                    Error,
                    "reference",
                    r,
                    Some(format!("Trigger {}", e.number)),
                    format!(
                        "trigger event {} refers to a trigger that is {}",
                        e.number,
                        why(&p.triggers, e.number)
                    ),
                ));
            }
        }
    }

    // ---------------------------------------------------------------- rooms
    let mut ids: HashMap<i32, String> = HashMap::new();
    let mut max_id = 0;
    let mut tile_ids: HashSet<i32> = HashSet::new();
    let mut max_tile = 0;
    for r in p.rooms.iter().flatten() {
        let rn = r.name.as_str();
        for inst in &r.instances {
            let ev = Some(format!("instance {}", inst.id));
            if !exists(&p.objects, inst.object) {
                out.push(d(
                    Error,
                    "reference",
                    rn,
                    ev.clone(),
                    format!(
                        "instance of object index {}, which is {}",
                        inst.object,
                        why(&p.objects, inst.object)
                    ),
                ));
            }
            if inst.id <= instance_format::FIRST_INSTANCE_ID {
                out.push(d(
                    Error,
                    "instance-id",
                    rn,
                    ev.clone(),
                    format!(
                        "instance id {} must be above {}",
                        inst.id,
                        instance_format::FIRST_INSTANCE_ID
                    ),
                ));
            }
            if let Some(other) = ids.get(&inst.id) {
                out.push(d(
                    Error,
                    "instance-id",
                    rn,
                    ev.clone(),
                    format!(
                        "duplicate instance id {} (also used in room {})",
                        inst.id, other
                    ),
                ));
            } else {
                ids.insert(inst.id, rn.to_string());
            }
            max_id = max_id.max(inst.id);
        }
        for t in &r.tiles {
            if !exists(&p.backgrounds, t.background) {
                out.push(d(
                    Error,
                    "reference",
                    rn,
                    Some(format!("tile {}", t.id)),
                    format!(
                        "tile uses background index {}, which is {}",
                        t.background,
                        why(&p.backgrounds, t.background)
                    ),
                ));
            }
            if !tile_ids.insert(t.id) {
                out.push(d(
                    Warning,
                    "tile-id",
                    rn,
                    Some(format!("tile {}", t.id)),
                    format!("duplicate tile id {}", t.id),
                ));
            }
            max_tile = max_tile.max(t.id);
        }
        for (i, b) in r.backgrounds.iter().enumerate() {
            if b.background != -1 && !exists(&p.backgrounds, b.background) {
                out.push(d(
                    Error,
                    "reference",
                    rn,
                    Some(format!("background {}", i)),
                    format!(
                        "background layer uses background index {}, which is {}",
                        b.background,
                        why(&p.backgrounds, b.background)
                    ),
                ));
            }
        }
        for (i, v) in r.views.iter().enumerate() {
            if v.follow_object != -1 && !exists(&p.objects, v.follow_object) {
                out.push(d(
                    Error,
                    "reference",
                    rn,
                    Some(format!("view {}", i)),
                    format!(
                        "view follows object index {}, which is {}",
                        v.follow_object,
                        why(&p.objects, v.follow_object)
                    ),
                ));
            }
        }
    }
    if max_id > p.last_instance_id {
        out.push(d(
            Error,
            "instance-id",
            "project",
            None,
            format!(
                "last_instance_id is {} but instance id {} is used; set it to at least {}",
                p.last_instance_id, max_id, max_id
            ),
        ));
    }
    if max_tile > p.last_tile_id {
        out.push(d(
            Warning,
            "tile-id",
            "project",
            None,
            format!(
                "last_tile_id is {} but tile id {} is used",
                p.last_tile_id, max_tile
            ),
        ));
    }
    if p.rooms.iter().all(|r| r.is_none()) {
        out.push(d(
            Error,
            "room-order",
            "project",
            None,
            "the project has no rooms".into(),
        ));
    } else if p.room_order.is_empty() {
        out.push(d(
            Error,
            "room-order",
            "project",
            None,
            "the room order is empty (the first entry is the start room)".into(),
        ));
    }
    let mut in_order = HashSet::new();
    for (i, &ri) in p.room_order.iter().enumerate() {
        if !exists(&p.rooms, ri) {
            out.push(d(
                Error,
                "room-order",
                "project",
                None,
                format!(
                    "room order entry {} is room index {}, which is {}",
                    i,
                    ri,
                    why(&p.rooms, ri)
                ),
            ));
        } else if !in_order.insert(ri) {
            out.push(d(
                Warning,
                "room-order",
                "project",
                None,
                format!(
                    "room {} appears twice in the room order",
                    p.rooms[ri as usize].as_ref().unwrap().name
                ),
            ));
        }
    }

    // ---------------------------------------------------------------- action arguments
    for list in action_lists(p) {
        for (i, a) in list.actions.iter().enumerate() {
            let mut push = |sev: Severity, kind: &'static str, msg: String| {
                let mut x = d(sev, kind, &list.resource, Some(list.event.clone()), msg);
                x.action = Some(i + 1);
                out.push(x);
            };
            if a.has_applies_to && a.applies_to >= 0 && !exists(&p.objects, a.applies_to) {
                push(
                    Error,
                    "reference",
                    format!(
                        "\"applies to\" object index {} is {}",
                        a.applies_to,
                        why(&p.objects, a.applies_to)
                    ),
                );
            }
            let n = (a.argument_count.max(0) as usize).min(a.args.len());
            for (k, text) in a.args.iter().take(n).enumerate() {
                let kind = a.arg_kinds.get(k).copied().unwrap_or(0);
                let Ok(idx) = text.trim().parse::<i32>() else {
                    continue;
                };
                if idx < 0 {
                    continue;
                }
                let (what, ok) = match kind {
                    arg_kind::SPRITE => ("sprite", exists(&p.sprites, idx)),
                    arg_kind::SOUND => ("sound", exists(&p.sounds, idx)),
                    arg_kind::BACKGROUND => ("background", exists(&p.backgrounds, idx)),
                    arg_kind::PATH => ("path", exists(&p.paths, idx)),
                    arg_kind::SCRIPT => ("script", exists(&p.scripts, idx)),
                    arg_kind::OBJECT => ("object", exists(&p.objects, idx)),
                    arg_kind::ROOM => ("room", exists(&p.rooms, idx)),
                    arg_kind::FONT => ("font", exists(&p.fonts, idx)),
                    arg_kind::TIMELINE => ("time line", exists(&p.timelines, idx)),
                    _ => continue,
                };
                if !ok {
                    push(
                        Error,
                        "reference",
                        format!(
                            "argument {} refers to {} index {}, which does not exist",
                            k + 1,
                            what,
                            idx
                        ),
                    );
                }
            }
            if opts.lite {
                if let Some(def) = instance_format::builtin_action(a.library_id, a.action_id) {
                    if def.pro_only {
                        push(
                            Warning,
                            "lite",
                            format!(
                                "action \"{}\" is not available in the Lite edition",
                                def.palette_title
                            ),
                        );
                    }
                }
            }
        }
    }
    if opts.lite && !p.extension_packages.is_empty() {
        out.push(d(
            Warning,
            "lite",
            "project",
            None,
            format!(
                "extension packages ({}) are believed to be unavailable in the Lite edition",
                p.extension_packages.join(", ")
            ),
        ));
    }

    // ---------------------------------------------------------------- names
    let mut names: HashMap<&str, &'static str> = HashMap::new();
    macro_rules! res {
        ($list:expr, $kind:expr) => {
            for r in $list.iter().flatten() {
                check_resource_name(&r.name, $kind, &mut names, out);
            }
        };
    }
    res!(p.sprites, "sprite");
    res!(p.sounds, "sound");
    res!(p.backgrounds, "background");
    res!(p.paths, "path");
    res!(p.scripts, "script");
    res!(p.fonts, "font");
    res!(p.timelines, "time line");
    res!(p.objects, "object");
    res!(p.rooms, "room");
    let mut consts: HashSet<&str> = HashSet::new();
    for c in &p.constants {
        let r = format!("constant {}", c.name);
        if !is_identifier(&c.name) {
            out.push(d(
                Warning,
                "name-clash",
                &r,
                None,
                format!("constant name \"{}\" is not a valid identifier", c.name),
            ));
        } else if gml::lexer::is_keyword(&c.name) {
            out.push(d(
                Error,
                "name-clash",
                &r,
                None,
                format!("constant name {} is a keyword", c.name),
            ));
        } else if let Some(k) = names.get(c.name.as_str()) {
            out.push(d(Warning, "name-clash", &r, None, format!("{} is also the name of a {}; the resource wins and this constant is never used", c.name, k)));
        } else if gml::builtins::variable_id(&c.name).is_some() {
            out.push(d(
                Warning,
                "name-clash",
                &r,
                None,
                format!(
                    "{} is a built-in variable; the variable wins and this constant is never used",
                    c.name
                ),
            ));
        } else if gml::builtins::constant_value(&c.name).is_some() {
            out.push(d(
                Warning,
                "name-clash",
                &r,
                None,
                format!("{} redefines a built-in constant", c.name),
            ));
        }
        if !consts.insert(&c.name) {
            out.push(d(
                Warning,
                "name-clash",
                &r,
                None,
                format!(
                    "constant {} is defined twice (the first definition wins)",
                    c.name
                ),
            ));
        }
    }
}

fn check_resource_name<'a>(
    name: &'a str,
    kind: &'static str,
    names: &mut HashMap<&'a str, &'static str>,
    out: &mut Vec<Diagnostic>,
) {
    use Severity::*;
    let e = |sev, msg: String| d(sev, "name-clash", name, None, msg);
    if !is_identifier(name) {
        out.push(e(
            Warning,
            format!(
                "{} name \"{}\" is not a valid identifier, so code cannot refer to it",
                kind, name
            ),
        ));
        return;
    }
    if let Some(prev) = names.get(name) {
        out.push(e(
            Error,
            format!("{} name {} is also used by a {}", kind, name, prev),
        ));
        return;
    }
    names.insert(name, kind);
    if gml::lexer::is_keyword(name) {
        out.push(e(Error, format!("{} name {} is a GML keyword", kind, name)));
    } else if gml::builtins::function_id(name).is_some() {
        let msg = if kind == "script" {
            format!("script {} replaces the built-in function {}", name, name)
        } else {
            format!("{} name {} is the name of a built-in function", kind, name)
        };
        out.push(e(Error, msg));
    } else if gml::builtins::variable_id(name).is_some() {
        out.push(e(
            Error,
            format!(
                "{} name {} is a built-in variable, so code cannot refer to the {}",
                kind, name, kind
            ),
        ));
    } else if gml::builtins::constant_value(name).is_some() {
        out.push(e(
            Error,
            format!(
                "{} name {} replaces the built-in constant {}",
                kind, name, name
            ),
        ));
    }
}
