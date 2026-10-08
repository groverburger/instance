//! `cargo run --example dumpcode -- GAME.gm81` — prints every script, event,
//! timeline moment and creation code of a project as plain text (one file), for
//! grepping while debugging a game. Drag-and-drop actions are shown as
//! `@action function(args)` lines.

fn actions(out: &mut String, acts: &[instance_format::Action]) {
    for a in acts {
        if a.kind == instance_format::action_kind::CODE {
            out.push_str(a.args.first().map(|s| s.as_str()).unwrap_or(""));
            out.push('\n');
        } else {
            let applies = if a.has_applies_to && a.applies_to != -1 { format!(" applies_to={}", a.applies_to) } else { String::new() };
            out.push_str(&format!(
                "@action lib{}:{} kind{} {}({}){}{}{}\n",
                a.library_id,
                a.action_id,
                a.kind,
                a.function_name,
                a.args.iter().take(a.argument_count.max(0) as usize).cloned().collect::<Vec<_>>().join(", "),
                if a.relative { " relative" } else { "" },
                if a.negate { " NOT" } else { "" },
                applies
            ));
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: dumpcode FILE");
    let (p, _) = instance_format::read_project_file(std::path::Path::new(&path)).expect("read");
    let mut out = String::new();
    for (i, s) in p.scripts.iter().enumerate() {
        if let Some(s) = s {
            out.push_str(&format!("\n=== script {} [{}]\n{}\n", s.name, i, s.code));
        }
    }
    for (i, o) in p.objects.iter().enumerate() {
        if let Some(o) = o {
            out.push_str(&format!(
                "\n##### object {} [{}] parent={} sprite={} solid={} depth={} persistent={}\n",
                o.name, i, o.parent, o.sprite, o.solid, o.depth, o.persistent
            ));
            for e in &o.events {
                out.push_str(&format!("\n=== {} event {}:{}\n", o.name, e.event_type, e.number));
                actions(&mut out, &e.actions);
            }
        }
    }
    for (i, t) in p.timelines.iter().enumerate() {
        if let Some(t) = t {
            for m in &t.moments {
                out.push_str(&format!("\n=== timeline {} [{}] moment {}\n", t.name, i, m.step));
                actions(&mut out, &m.actions);
            }
        }
    }
    for (i, r) in p.rooms.iter().enumerate() {
        if let Some(r) = r {
            out.push_str(&format!(
                "\n=== room {} [{}] {}x{} speed {} instances {} views {}\n{}\n",
                r.name,
                i,
                r.width,
                r.height,
                r.speed,
                r.instances.len(),
                r.views_enabled,
                r.creation_code
            ));
            for (k, v) in r.views.iter().enumerate() {
                if v.visible {
                    out.push_str(&format!("view {}: {:?}\n", k, v));
                }
            }
            for inst in &r.instances {
                if !inst.creation_code.trim().is_empty() {
                    out.push_str(&format!("--- instance {} (object {})\n{}\n", inst.id, inst.object, inst.creation_code));
                }
            }
        }
    }
    for c in &p.constants {
        out.push_str(&format!("const {} = {}\n", c.name, c.value));
    }
    println!("room order: {:?}", p.room_order);
    println!("{}", out);
}
