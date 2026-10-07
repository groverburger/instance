//! Gameplay checks on the GameMaker example projects (skipped when absent).
//! The example files are used as opaque inputs and are not part of the repo.

use gm_runner::{Game, InputEvent};
use gml::Vm;
use std::path::PathBuf;

fn example(name: &str) -> Option<PathBuf> {
    let dir = std::env::var("GM_EXAMPLES").unwrap_or_else(|_| r"C:\Users\Grob\Documents\gamemaker\GameMaker\Examples".into());
    let p = PathBuf::from(dir).join(name);
    p.exists().then_some(p)
}

fn load(name: &str) -> Option<(Game, Vm)> {
    let p = example(name)?;
    let mut vm = Vm::new();
    let mut g = gm_runner::load_game(&p, &mut vm, true).expect("load");
    g.start(&mut vm).expect("start");
    Some((g, vm))
}

fn obj(g: &Game, name: &str) -> i32 {
    g.objects.iter().position(|o| o.as_ref().map(|o| o.name == name).unwrap_or(false)).unwrap() as i32
}

fn steps(g: &mut Game, vm: &mut Vm, n: usize) {
    for _ in 0..n {
        g.step(vm).expect("step");
    }
}

#[test]
fn tutorial1_click_fruit_scores() {
    let Some((mut g, mut vm)) = load("tutorial1.gmk") else { return };
    steps(&mut g, &mut vm, 5);
    let apple = obj(&g, "obj_apple");
    let i = g.instances.iter().find(|i| i.object_index == apple).unwrap();
    let (x, y) = (i.x + 16.0 + i.hspeed, i.y + 16.0 + i.vspeed);
    let f = g.frame;
    g.scripted_input = vec![(f, InputEvent::MouseMove(x, y)), (f, InputEvent::MouseDown(1)), (f + 1, InputEvent::MouseUp(1))];
    steps(&mut g, &mut vm, 3);
    assert_eq!(g.score, 50.0);
    // fruit bounces off the walls: everything stays inside the room
    steps(&mut g, &mut vm, 300);
    for i in &g.instances {
        assert!(i.x >= -32.0 && i.x <= 672.0 && i.y >= -32.0 && i.y <= 512.0);
    }
}

#[test]
fn treasure_explorer_moves_with_keys() {
    let Some((mut g, mut vm)) = load("treasure.gm81") else { return };
    assert_eq!(g.lives, 3.0);
    let ex = obj(&g, "explorer");
    let start = g.instances.iter().find(|i| i.object_index == ex).map(|i| (i.x, i.y)).unwrap();
    let f = g.frame;
    g.scripted_input = Vec::new();
    for (k, key) in [37u8, 38, 39, 40].iter().enumerate() {
        let t = f + k as u64 * 10;
        g.scripted_input.push((t, InputEvent::KeyDown(*key)));
        g.scripted_input.push((t + 8, InputEvent::KeyUp(*key)));
    }
    steps(&mut g, &mut vm, 42);
    let now = g.instances.iter().find(|i| i.object_index == ex).map(|i| (i.x, i.y)).unwrap();
    assert!(now != start, "explorer did not move: {:?}", now);
    // never inside a wall
    let wall = obj(&g, "wall");
    let id = g.instances.iter().find(|i| i.object_index == ex).unwrap().id;
    let walls: Vec<i32> = g.instances.iter().filter(|i| i.object_index == wall).map(|i| i.id).collect();
    assert!(walls.iter().all(|&w| !g.instances_collide(id, w)));
}

#[test]
fn street_racing_start_and_steer() {
    let Some((mut g, mut vm)) = load("street racing.gm81") else { return };
    assert_eq!(g.room, 0);
    let f = g.frame;
    g.scripted_input = vec![(f + 2, InputEvent::KeyDown(32)), (f + 4, InputEvent::KeyUp(32))];
    steps(&mut g, &mut vm, 6);
    assert_eq!(g.room, 1, "any key goes to the game room");
    assert_eq!(g.lives, 3.0);
    let car = obj(&g, "car_racing");
    let x0 = g.instances.iter().find(|i| i.object_index == car).unwrap().x;
    let f = g.frame;
    g.scripted_input = vec![(f, InputEvent::KeyDown(39)), (f + 10, InputEvent::KeyUp(39))];
    steps(&mut g, &mut vm, 12);
    let x1 = g.instances.iter().find(|i| i.object_index == car).unwrap().x;
    assert!(x1 > x0, "{} -> {}", x0, x1);
    steps(&mut g, &mut vm, 400);
    assert!(g.score > 300.0);
}
