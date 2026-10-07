//! Second pass: GML translation, compatibility layer and Studio-only event emulation.

use crate::compat;
use crate::gml::{self, Translator, Usage};
use crate::loader::{Kind, Loaded};
use crate::Warning;
use gm_format::model::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// First line of instance creation code that carries the Studio per-instance
/// transform (scale/rotation/colour); the next line is that transform.
pub const TRANSFORM_MARK: &str = "// gmx2gm81: instance transform\n";

/// Conversion options.
#[derive(Debug, Clone)]
pub struct Options {
    /// GMS2-style backslash escapes in strings (off: GMS1 has none).
    pub string_escapes: bool,
    /// Override "treat uninitialised variables as 0" (None: take the Studio setting).
    pub uninit_as_zero: Option<bool>,
    /// Inject compatibility scripts/constants.
    pub compat: bool,
    /// Emulate Studio's order "Create event, then instance creation code".
    pub studio_create_order: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            string_escapes: false,
            uninit_as_zero: None,
            compat: true,
            studio_create_order: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeKind {
    Program,
    Expr,
}

/// Visit every code piece of the project (and of pending Studio-only events).
fn visit_code(
    p: &mut Project,
    extra: &mut [(usize, i32, i32, Vec<Action>)],
    f: &mut dyn FnMut(&str, &mut String, CodeKind),
) {
    fn actions(owner: &str, list: &mut [Action], f: &mut dyn FnMut(&str, &mut String, CodeKind)) {
        for (i, a) in list.iter_mut().enumerate() {
            let ctx = format!("{owner}, action {}", i + 1);
            if a.kind == action_kind::CODE {
                if let Some(c) = a.args.get_mut(0) {
                    f(&ctx, c, CodeKind::Program);
                }
            } else if a.kind == action_kind::VARIABLE {
                if let Some(c) = a.args.get_mut(1) {
                    f(&ctx, c, CodeKind::Expr);
                }
            } else {
                let n = (a.argument_count.max(0) as usize).min(a.args.len());
                for k in 0..n {
                    let kind = a.arg_kinds.get(k).copied().unwrap_or(0);
                    if kind == arg_kind::EXPRESSION {
                        f(&ctx, &mut a.args[k], CodeKind::Expr);
                    }
                }
            }
        }
    }
    for s in p.scripts.iter_mut().flatten() {
        let ctx = format!("script {}", s.name);
        f(&ctx, &mut s.code, CodeKind::Program);
    }
    let names: Vec<String> = p.objects.iter().map(|o| o.as_ref().map(|o| o.name.clone()).unwrap_or_default()).collect();
    for o in p.objects.iter_mut().flatten() {
        for e in &mut o.events {
            let owner = format!("object {}, event {}/{}", o.name, e.event_type, e.number);
            actions(&owner, &mut e.actions, f);
        }
    }
    for (oi, t, n, acts) in extra.iter_mut() {
        let owner = format!("object {}, event {t}/{n}", names.get(*oi).cloned().unwrap_or_default());
        actions(&owner, acts, f);
    }
    for tl in p.timelines.iter_mut().flatten() {
        for m in &mut tl.moments {
            let owner = format!("timeline {}, step {}", tl.name, m.step);
            actions(&owner, &mut m.actions, f);
        }
    }
    for r in p.rooms.iter_mut().flatten() {
        let ctx = format!("room {} creation code", r.name);
        f(&ctx, &mut r.creation_code, CodeKind::Program);
        for inst in &mut r.instances {
            let ctx = format!("room {} instance {}", r.name, inst.id);
            f(&ctx, &mut inst.creation_code, CodeKind::Program);
        }
    }
}

fn code_action(code: &str) -> Action {
    let mut a = Action {
        library_id: 1,
        action_id: 603,
        kind: action_kind::CODE,
        may_be_relative: false,
        is_question: false,
        has_applies_to: true,
        exe_type: exe_type::CODE,
        function_name: String::new(),
        code: String::new(),
        argument_count: 1,
        arg_kinds: vec![arg_kind::STRING, 0, 0, 0, 0, 0, 0, 0],
        applies_to: APPLIES_TO_SELF,
        relative: false,
        args: vec![code.to_string()],
        negate: false,
    };
    a.args.resize(8, "0".into());
    a
}

fn ancestors(p: &Project, mut o: i32) -> Vec<i32> {
    let mut v = Vec::new();
    let mut guard = 0;
    while o >= 0 && guard < 1000 {
        guard += 1;
        let Some(Some(obj)) = p.objects.get(o as usize) else { break };
        if obj.parent < 0 {
            break;
        }
        o = obj.parent;
        v.push(o);
    }
    v
}

fn set_event(o: &mut Object, t: i32, n: i32, actions: Vec<Action>) {
    if let Some(e) = o.events.iter_mut().find(|e| e.event_type == t && e.number == n) {
        e.actions = actions;
    } else {
        o.events.push(Event {
            event_type: t,
            number: n,
            actions,
        });
        o.events.sort_by_key(|e| e.event_type);
    }
}

/// Run the conversion pass over loaded data.
pub fn convert(mut l: Loaded, opts: &Options) -> (Project, Vec<Warning>) {
    let mut p = std::mem::take(&mut l.project);
    let mut warnings = std::mem::take(&mut l.warnings);
    let mut extra = std::mem::take(&mut l.extra_events);
    let warn = |w: &mut Vec<Warning>, c: &str, m: String| {
        w.push(Warning {
            context: c.to_string(),
            message: m,
        })
    };

    // 1. Directives (#macro, #region) and enums.
    let mut macros: Vec<(String, String)> = Vec::new();
    let mut tr = Translator {
        string_escapes: opts.string_escapes,
        ..Default::default()
    };
    visit_code(&mut p, &mut extra, &mut |_, code, kind| {
        if kind == CodeKind::Program && (code.contains("#macro") || code.contains("#region") || code.contains("#endregion")) {
            let (c, m) = gml::strip_directives(code);
            *code = c;
            macros.extend(m);
        }
        if kind == CodeKind::Program && code.contains("enum") {
            if let Ok(stmts) = gml::parse_program(code, opts.string_escapes) {
                gml::collect_enums(&stmts, code, &mut tr.enums);
            }
        }
    });
    for (n, v) in macros {
        p.constants.push(Constant { name: n, value: v });
    }

    // 2. Usage pass (untranslated) to decide renames.
    let mut usage = Usage::default();
    visit_code(&mut p, &mut extra, &mut |_, code, kind| {
        let t = match kind {
            CodeKind::Program => tr.translate(code),
            CodeKind::Expr => tr.translate_expr(code),
        };
        usage.calls.extend(t.usage.calls);
        usage.idents.extend(t.usage.idents);
        usage.assigned.extend(t.usage.assigned);
    });
    let b = compat::builtins();
    let mut user_names: HashSet<String> = HashSet::new();
    for m in l.names.values() {
        user_names.extend(m.keys().cloned());
    }
    for s in p.scripts.iter().flatten() {
        user_names.insert(s.name.clone());
    }
    for c in &p.constants {
        user_names.insert(c.name.clone());
    }
    let defined = |n: &str| user_names.contains(n) || usage.assigned.contains(n);
    if opts.compat {
        for (n, v) in compat::VAR_RENAMES {
            if usage.idents.contains(*n) && !defined(n) && !b.contains(n) {
                tr.var_renames.insert(n.to_string(), v.to_string());
            }
        }
        let gui = extra.iter().any(|e| e.1 == event_type::DRAW && matches!(e.2, 64 | 74 | 75 | 77));
        if gui && usage.idents.contains("view_current") {
            tr.var_renames.insert("view_current".into(), "__gmx_view_current()".into());
        }
        for (from, to) in compat::WRAPS {
            if usage.calls.contains(*from) && !user_names.contains(*from) && !user_names.contains(*to) {
                tr.call_renames.insert(from.to_string(), to.to_string());
            }
        }
        for c in &usage.calls {
            if b.functions.contains(c) || user_names.contains(c) {
                continue;
            }
            if let Some(r) = compat::colour_rename(c) {
                tr.call_renames.insert(c.clone(), r);
            }
        }
    }

    // 3. Translate.
    let mut usage = Usage::default();
    let mut errors = 0;
    let mut changed = 0;
    let mut notes: Vec<(String, String)> = Vec::new();
    visit_code(&mut p, &mut extra, &mut |ctx, code, kind| {
        if code.trim().is_empty() {
            return;
        }
        let t = match kind {
            CodeKind::Program => tr.translate(code),
            CodeKind::Expr => tr.translate_expr(code),
        };
        if let Some(e) = &t.error {
            errors += 1;
            notes.push((ctx.to_string(), format!("GML not translated ({e}); kept as is")));
        }
        for n in &t.notes {
            notes.push((ctx.to_string(), n.clone()));
        }
        if t.changed {
            changed += 1;
            *code = t.code;
        }
        usage.calls.extend(t.usage.calls);
        usage.idents.extend(t.usage.idents);
        usage.assigned.extend(t.usage.assigned);
    });
    for (c, m) in notes {
        warn(&mut warnings, &c, m);
    }
    if changed > 0 {
        warn(&mut warnings, "gml", format!("{changed} code piece(s) rewritten from Studio syntax"));
    }
    let _ = errors;

    // 4. Missing functions → compat scripts or stubs.
    let mut need: BTreeMap<String, ()> = BTreeMap::new();
    let mut stubs: Vec<String> = Vec::new();
    let mut compat_used: Vec<String> = Vec::new();
    for c in &usage.calls {
        if b.functions.contains(c) || user_names.contains(c) || gml::parser::is_keyword(c) {
            continue;
        }
        if opts.compat && compat::script(c).is_some() {
            need.insert(c.clone(), ());
            compat_used.push(c.clone());
        } else {
            stubs.push(c.clone());
        }
    }
    let has_gui = extra.iter().any(|e| e.1 == event_type::DRAW && matches!(e.2, 64 | 74 | 75 | 77));
    if has_gui {
        need.insert("display_get_gui_width".into(), ());
        need.insert("display_get_gui_height".into(), ());
    }
    for v in tr.var_renames.values() {
        if let Some(f) = v.strip_suffix("()") {
            if compat::script(f).is_some() {
                need.insert(f.to_string(), ());
            }
        }
    }
    let script_base = p.scripts.len();
    for s in compat::with_deps(&need) {
        if b.functions.contains(s.name) || user_names.contains(s.name) {
            continue;
        }
        p.scripts.push(Some(Script {
            name: s.name.to_string(),
            last_changed: 0.0,
            code: format!("{}{}", compat::HEADER, s.code),
        }));
    }
    if !compat_used.is_empty() {
        warn(&mut warnings, "compat", format!("Studio functions emulated by injected scripts: {}", compat_used.join(", ")));
    }
    for s in &stubs {
        p.scripts.push(Some(Script {
            name: s.clone(),
            last_changed: 0.0,
            code: format!("// Added by gmx2gm81: unsupported function '{s}' (not in GM 8.1); does nothing.\nreturn 0;"),
        }));
    }
    if !stubs.is_empty() {
        warn(&mut warnings, "unknown functions", format!("{} function(s) not available in GM 8.1, stubbed to return 0: {}", stubs.len(), stubs.join(", ")));
    }
    if !tr.call_renames.is_empty() {
        let v: Vec<String> = tr.call_renames.iter().map(|(a, b)| format!("{a}→{b}")).collect();
        warn(&mut warnings, "compat", format!("renamed: {}", v.join(", ")));
    }
    if !tr.var_renames.is_empty() {
        let v: Vec<&String> = tr.var_renames.keys().collect();
        warn(&mut warnings, "compat", format!("Studio variables replaced: {v:?}"));
    }
    let _ = script_base;

    // 5. Unknown identifiers.
    let mut all_known: HashSet<String> = user_names.clone();
    all_known.extend(p.constants.iter().map(|c| c.name.clone()));
    let mut added_consts = Vec::new();
    let mut unknown = Vec::new();
    for id in &usage.idents {
        if all_known.contains(id)
            || b.contains(id)
            || usage.assigned.contains(id)
            || tr.enums.contains_key(id)
            || tr.var_renames.contains_key(id)
            || id.starts_with("__gmx_")
        {
            continue;
        }
        if let Some(&iid) = l.instance_names.get(id) {
            p.constants.push(Constant {
                name: id.clone(),
                value: iid.to_string(),
            });
            continue;
        }
        if let Some(&si) = l.names.get(&Kind::Shader).and_then(|m| m.get(id)) {
            p.constants.push(Constant {
                name: id.clone(),
                value: si.to_string(),
            });
            continue;
        }
        if opts.compat {
            if let Some(v) = compat::constant(id) {
                p.constants.push(Constant {
                    name: id.clone(),
                    value: v.to_string(),
                });
                added_consts.push(id.clone());
                continue;
            }
        }
        unknown.push(id.clone());
    }
    if !added_consts.is_empty() {
        warn(&mut warnings, "compat", format!("Studio constants added: {}", added_consts.join(", ")));
    }
    if !unknown.is_empty() {
        warn(
            &mut warnings,
            "unknown identifiers",
            format!("never assigned and not GM 8.1 built-ins (Studio built-ins or typos?): {}", unknown.join(", ")),
        );
    }

    // 6. Free user events for emulation.
    let mut used_user: BTreeSet<i32> = BTreeSet::new();
    for o in p.objects.iter().flatten() {
        for e in &o.events {
            if e.event_type == event_type::OTHER && (10..26).contains(&e.number) {
                used_user.insert(e.number - 10);
            }
        }
    }
    for id in &usage.idents {
        if let Some(n) = id.strip_prefix("ev_user").and_then(|n| n.parse::<i32>().ok()) {
            used_user.insert(n);
        }
    }
    let mut free_user = (0..16).rev().filter(|n| !used_user.contains(n));

    // 7. Studio-only events.
    emulate_events(&mut p, &mut extra, &mut free_user, &mut warnings);

    // 8. Instance creation code order.
    finish_creation_code(&mut p, opts, &mut free_user, &mut warnings);

    if let Some(v) = opts.uninit_as_zero {
        p.settings.treat_uninitialized_as_zero = v;
    }
    p.resource_tree = crate::loader::build_tree(&l, &p);
    (p, warnings)
}

fn emulate_events(
    p: &mut Project,
    extra: &mut Vec<(usize, i32, i32, Vec<Action>)>,
    free_user: &mut dyn Iterator<Item = i32>,
    warnings: &mut Vec<Warning>,
) {
    let mut gui: BTreeMap<usize, Vec<Action>> = BTreeMap::new();
    let mut begin: BTreeMap<usize, Vec<Action>> = BTreeMap::new();
    let mut end: BTreeMap<usize, Vec<Action>> = BTreeMap::new();
    let mut dropped: Vec<String> = Vec::new();
    for (oi, t, n, acts) in extra.drain(..) {
        let oname = p.objects[oi].as_ref().map(|o| o.name.clone()).unwrap_or_default();
        match (t, n) {
            (event_type::DRAW, 64 | 74 | 75 | 77) => gui.entry(oi).or_default().extend(acts),
            (event_type::DRAW, 72 | 76) => begin.entry(oi).or_default().extend(acts),
            (event_type::DRAW, 73) => end.entry(oi).or_default().extend(acts),
            _ => dropped.push(format!("{oname} {t}/{n}")),
        }
    }
    if !dropped.is_empty() {
        warnings.push(Warning {
            context: "events".into(),
            message: format!("Studio-only events dropped (async/resize/...): {}", dropped.join(", ")),
        });
    }
    // Draw Begin / Draw End: merged into the Draw event (parents first).
    if !begin.is_empty() || !end.is_empty() {
        let mut objs: Vec<usize> = begin.keys().chain(end.keys()).copied().collect::<BTreeSet<_>>().into_iter().collect();
        objs.sort_by_key(|&o| ancestors(p, o as i32).len());
        for oi in objs {
            let has_anc_draw = ancestors(p, oi as i32).iter().any(|&a| {
                p.objects[a as usize].as_ref().is_some_and(|o| o.event(event_type::DRAW, 0).is_some())
            });
            let o = p.objects[oi].as_mut().unwrap();
            let own = o.event(event_type::DRAW, 0).map(|e| e.actions.clone());
            let mid = match own {
                Some(a) => a,
                None if has_anc_draw => vec![code_action("// Draw Begin/End merged into Draw by gmx2gm81\nevent_inherited();")],
                None => vec![code_action("// Draw Begin/End merged into Draw by gmx2gm81: default drawing\nif (sprite_index >= 0) draw_sprite_ext(sprite_index, image_index, x, y, image_xscale, image_yscale, image_angle, image_blend, image_alpha);")],
            };
            let mut all = begin.remove(&oi).unwrap_or_default();
            all.extend(mid);
            all.extend(end.remove(&oi).unwrap_or_default());
            set_event(o, event_type::DRAW, 0, all);
        }
        warnings.push(Warning {
            context: "events".into(),
            message: "Draw Begin/End events merged into the Draw event (ordering across instances differs)".into(),
        });
    }
    if gui.is_empty() {
        return;
    }
    let Some(g) = free_user.next() else {
        warnings.push(Warning {
            context: "events".into(),
            message: "Draw GUI events dropped: no free user event to emulate them".into(),
        });
        return;
    };
    // Objects whose GUI handler is not reached through an ancestor's `with`.
    let gui_objs: Vec<usize> = gui.keys().copied().collect();
    let mut roots: Vec<usize> = gui_objs
        .iter()
        .copied()
        .filter(|&o| !ancestors(p, o as i32).iter().any(|a| gui.contains_key(&(*a as usize))))
        .collect();
    roots.sort_by_key(|&o| std::cmp::Reverse(p.objects[o].as_ref().map_or(0, |x| x.depth)));
    let names: Vec<String> = gui_objs.iter().map(|&o| p.objects[o].as_ref().unwrap().name.clone()).collect();
    for (oi, acts) in gui {
        let o = p.objects[oi].as_mut().unwrap();
        set_event(o, event_type::OTHER, 10 + g, acts);
    }
    let mut code = format!(
        "// Draw GUI emulation added by gmx2gm81: runs the Studio Draw GUI events (moved to\n\
         // user event {g}) after everything else, in GUI coordinates.\n\
         var __i, __last;\n\
         if (view_enabled) {{\n    __last = -1;\n    for (__i = 0; __i < 8; __i += 1) if (view_visible[__i]) __last = __i;\n    if (view_current != __last) exit;\n}}\n\
         d3d_set_projection_ortho(0, 0, display_get_gui_width(), display_get_gui_height(), 0);\n"
    );
    code.push_str("global.__gmx_in_gui = true;\n");
    for o in &roots {
        let n = &p.objects[*o].as_ref().unwrap().name;
        code.push_str(&format!("with ({n}) if (visible) event_perform(ev_other, ev_user{g});\n"));
    }
    code.push_str("global.__gmx_in_gui = false;\n");
    let ctrl = Object {
        name: "__gmx_gui_controller".into(),
        depth: -31_000,
        visible: true,
        events: vec![Event {
            event_type: event_type::DRAW,
            number: 0,
            actions: vec![code_action(&code)],
        }],
        ..Object::default()
    };
    p.objects.push(Some(ctrl));
    let ci = (p.objects.len() - 1) as i32;
    for r in p.rooms.iter_mut().flatten() {
        p.last_instance_id += 1;
        r.instances.push(Instance {
            x: 0,
            y: 0,
            object: ci,
            id: p.last_instance_id,
            creation_code: String::new(),
            locked: false,
        });
    }
    warnings.push(Warning {
        context: "events".into(),
        message: format!(
            "Draw GUI events of {} emulated via user event {g} and object __gmx_gui_controller",
            names.join(", ")
        ),
    });
}

fn finish_creation_code(
    p: &mut Project,
    opts: &Options,
    free_user: &mut dyn Iterator<Item = i32>,
    warnings: &mut Vec<Warning>,
) {
    // Split off the transform prefix.
    let mut with_code: BTreeSet<i32> = BTreeSet::new();
    let mut split: HashMap<(usize, usize), (String, String)> = HashMap::new();
    for (ri, r) in p.rooms.iter().enumerate() {
        let Some(r) = r else { continue };
        for (ii, inst) in r.instances.iter().enumerate() {
            let (pre, code) = match inst.creation_code.strip_prefix(TRANSFORM_MARK) {
                Some(rest) => match rest.split_once('\n') {
                    Some((a, b)) => (a.to_string(), b.to_string()),
                    None => (rest.to_string(), String::new()),
                },
                None => (String::new(), inst.creation_code.clone()),
            };
            if !code.trim().is_empty() {
                with_code.insert(inst.object);
            }
            split.insert((ri, ii), (pre, code));
        }
    }
    let mut user = None;
    if opts.studio_create_order && !with_code.is_empty() {
        // Objects whose Create event must be moved: those with creation code and all
        // their ancestors.
        let mut moved: BTreeSet<i32> = BTreeSet::new();
        for &o in &with_code {
            moved.insert(o);
            moved.extend(ancestors(p, o));
        }
        let any_create = moved.iter().any(|&o| {
            p.objects[o as usize].as_ref().is_some_and(|x| x.event(event_type::CREATE, 0).is_some())
        });
        if any_create {
            match free_user.next() {
                Some(u) => {
                    user = Some(u);
                    for &o in &moved {
                        let Some(obj) = p.objects[o as usize].as_mut() else { continue };
                        let Some(ev) = obj.events.iter().position(|e| e.event_type == event_type::CREATE && e.number == 0) else {
                            continue;
                        };
                        let acts = std::mem::take(&mut obj.events[ev].actions);
                        obj.events[ev].actions = vec![code_action(&format!(
                            "// gmx2gm81: Studio runs the Create event before the instance creation code;\n\
                             // the Create actions moved to user event {u} and run from the creation code.\n\
                             if (variable_local_exists(\"__gmx_cc_done\")) {{ if (__gmx_cc_done) {{ __gmx_cc_done = false; exit; }} }}\n\
                             event_perform(ev_other, ev_user{u});"
                        ))];
                        set_event(obj, event_type::OTHER, 10 + u, acts);
                    }
                    warnings.push(Warning {
                        context: "creation code".into(),
                        message: format!(
                            "Create events of {} object(s) moved to user event {u} to run before instance creation code (Studio order)",
                            moved.len()
                        ),
                    });
                }
                None => warnings.push(Warning {
                    context: "creation code".into(),
                    message: "no free user event: instance creation code runs before Create (GM 8.1 order)".into(),
                }),
            }
        }
    }
    for (ri, r) in p.rooms.iter_mut().enumerate() {
        let Some(r) = r else { continue };
        for (ii, inst) in r.instances.iter_mut().enumerate() {
            let Some((pre, code)) = split.remove(&(ri, ii)) else { continue };
            let mut out = String::new();
            if !pre.is_empty() {
                out.push_str(&pre);
                out.push('\n');
            }
            if let (Some(u), false) = (user, code.trim().is_empty()) {
                out.push_str(&format!("event_perform(ev_other, ev_user{u}); __gmx_cc_done = true;\n"));
            }
            out.push_str(&code);
            inst.creation_code = out;
        }
    }
}
