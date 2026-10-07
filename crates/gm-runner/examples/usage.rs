//! Dev tool: print every code fragment and action call in a project (for coverage planning).
use gm_format::*;
fn main() {
    let path = std::env::args().nth(1).expect("path");
    let (p, _) = read_project_file(&path).unwrap();
    let dump = |where_: &str, acts: &[Action]| {
        for a in acts {
            println!("ACTION {} lib{} id{} kind{} fn={} applies={} rel={} not={} args={:?}", where_, a.library_id, a.action_id, a.kind, a.function_name, a.applies_to, a.relative, a.negate, &a.args[..a.argument_count.max(0) as usize]);
        }
    };
    for s in p.scripts.iter().flatten() { println!("SCRIPT {}\n{}", s.name, s.code); }
    for (i, o) in p.objects.iter().enumerate() { if let Some(o) = o {
        println!("OBJECT {} {} spr={} solid={} vis={} depth={} parent={} mask={} persistent={}", i, o.name, o.sprite, o.solid, o.visible, o.depth, o.parent, o.mask_sprite, o.persistent);
        for e in &o.events { dump(&format!("{}:{}:{}", o.name, e.event_type, e.number), &e.actions); }
    }}
    for t in p.timelines.iter().flatten() { for m in &t.moments { dump(&format!("TL {} {}", t.name, m.step), &m.actions); } }
    for r in p.rooms.iter().flatten() {
        println!("ROOM {} {}x{} speed={} views={} cc={:?}", r.name, r.width, r.height, r.speed, r.views_enabled, r.creation_code);
        for b in &r.backgrounds { if b.visible { println!("  BG {:?}", b); } }
        for v in &r.views { if v.visible { println!("  VIEW {:?}", v); } }
        for i in &r.instances { if !i.creation_code.is_empty() { println!("  INSTCC {} {:?}", i.id, i.creation_code); } }
    }
    println!("SETTINGS {:?}", (p.settings.scaling, p.settings.color_outside_room, p.settings.display_errors, p.settings.treat_uninitialized_as_zero, p.settings.let_esc_end_game));
    for s in p.sounds.iter().flatten() { println!("SOUND {} kind={} type={} file={} data={}", s.name, s.kind, s.file_type, s.file_name, s.data.as_ref().map(|d| d.len()).unwrap_or(0)); }
    for s in p.sprites.iter().flatten() { println!("SPRITE {} o=({},{}) shape={} sep={} bbmode={} bb=({},{},{},{}) tol={}", s.name, s.origin_x, s.origin_y, s.collision_shape, s.separate_collision_masks, s.bbox_mode, s.bbox_left, s.bbox_top, s.bbox_right, s.bbox_bottom, s.alpha_tolerance); }
    for f in p.fonts.iter().flatten() { println!("FONT {:?}", (&f.name, &f.font_name, f.size, f.bold, f.italic, f.range_start, f.range_end, f.charset, f.antialias)); }
    for pa in p.paths.iter().flatten() { println!("PATH {} kind={} closed={} prec={} pts={:?}", pa.name, pa.connection_kind, pa.closed, pa.precision, pa.points); }
}
