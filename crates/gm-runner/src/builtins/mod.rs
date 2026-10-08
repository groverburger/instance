//! Built-in world functions, registered by name per manual chapter.

use crate::game::Game;
use gml::{Ctx, Error, FunctionId, Value, Vm};
use std::sync::OnceLock;

pub mod d3d;
pub mod draw;
pub mod files;
pub mod game;
pub mod input;
pub mod motion;
pub mod part;
pub mod res;
pub mod sound;
pub mod stubs;
pub mod ui;

/// A runner-implemented built-in function.
pub type BuiltinFn = fn(&mut Game, &mut Vm, &Ctx, &A) -> Result<Value, Error>;

/// Registration list.
pub type Reg = Vec<(&'static str, BuiltinFn)>;

/// Typed argument access producing GML errors.
pub struct A<'a> {
    pub v: &'a [Value],
    pub f: FunctionId,
}

impl<'a> A<'a> {
    pub fn len(&self) -> usize {
        self.v.len()
    }
    pub fn is_empty(&self) -> bool {
        self.v.is_empty()
    }
    pub fn val(&self, i: usize) -> Value {
        self.v.get(i).cloned().unwrap_or(Value::Real(0.0))
    }
    pub fn err(&self, msg: &str) -> Error {
        Error::new(format!("{}: {}", self.f.name(), msg))
    }
    pub fn type_err(&self) -> Error {
        Error::new(format!("Wrong type of arguments to function {}.", self.f.name()))
    }
    pub fn r(&self, i: usize) -> Result<f64, Error> {
        match self.v.get(i) {
            Some(Value::Real(x)) => Ok(*x),
            None => Ok(0.0),
            Some(Value::Str(_)) => Err(self.type_err()),
        }
    }
    pub fn i(&self, i: usize) -> Result<i32, Error> {
        Ok(gml::to_int(self.r(i)?).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    }
    pub fn b(&self, i: usize) -> Result<bool, Error> {
        Ok(self.r(i)? >= 0.5)
    }
    /// String argument as raw bytes.
    pub fn sb(&self, i: usize) -> Result<Vec<u8>, Error> {
        match self.v.get(i) {
            Some(Value::Str(s)) => Ok(s.as_bytes().to_vec()),
            None => Ok(Vec::new()),
            Some(Value::Real(_)) => Err(self.type_err()),
        }
    }
    /// String argument decoded as Windows-1252.
    pub fn s(&self, i: usize) -> Result<String, Error> {
        Ok(gm_format::text::decode_cp1252(&self.sb(i)?))
    }
    /// Any value as display text (reals formatted like string()).
    pub fn text(&self, i: usize) -> Vec<u8> {
        match self.v.get(i) {
            Some(Value::Str(s)) => s.as_bytes().to_vec(),
            Some(Value::Real(r)) => gml::real_to_string(*r).into_bytes(),
            None => Vec::new(),
        }
    }
}

/// Helpers for return values.
pub fn r(x: f64) -> Result<Value, Error> {
    Ok(Value::Real(x))
}
pub fn bo(x: bool) -> Result<Value, Error> {
    Ok(Value::Real(if x { 1.0 } else { 0.0 }))
}
pub fn st(x: &str) -> Result<Value, Error> {
    Ok(Value::Str(gm_format::text::encode_cp1252(x).into()))
}
pub fn zero() -> Result<Value, Error> {
    Ok(Value::Real(0.0))
}

struct Table {
    fns: Vec<Option<BuiltinFn>>,
    stubbed: Vec<bool>,
    /// Functions that may change collision geometry of existing instances.
    geometry: Vec<bool>,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let mut list: Reg = Vec::new();
        game::register(&mut list);
        motion::register(&mut list);
        input::register(&mut list);
        draw::register(&mut list);
        d3d::register(&mut list);
        sound::register(&mut list);
        ui::register(&mut list);
        res::register(&mut list);
        files::register(&mut list);
        part::register(&mut list);
        crate::net::register(&mut list);
        crate::ext::register_builtins(&mut list);
        let mut stub_list: Reg = Vec::new();
        stubs::register(&mut stub_list);
        let n = gml::builtins::FUNCTIONS.len();
        let mut fns = vec![None; n];
        let mut stubbed = vec![false; n];
        for (name, f) in list {
            let id = gml::builtins::function_id(name).unwrap_or_else(|| panic!("{} is not in builtins.json", name));
            fns[id.0 as usize] = Some(f);
        }
        for (name, f) in stub_list {
            let id = gml::builtins::function_id(name).unwrap_or_else(|| panic!("{} is not in builtins.json", name));
            if fns[id.0 as usize].is_none() {
                fns[id.0 as usize] = Some(f);
                stubbed[id.0 as usize] = true;
            }
        }
        let geometry = (0..n)
            .map(|i| {
                let name = gml::builtins::FUNCTIONS[i].name;
                name.starts_with("sprite_")
                    && !name.starts_with("sprite_get")
                    && !name.starts_with("sprite_exists")
                    && !name.starts_with("sprite_save")
            })
            .collect();
        Table { fns, stubbed, geometry }
    })
}

/// Dispatches a world function.
pub fn dispatch(g: &mut Game, vm: &mut Vm, ctx: &Ctx, f: FunctionId, args: &[Value]) -> Result<Value, Error> {
    let t = table();
    if t.stubbed[f.0 as usize] || t.fns[f.0 as usize].is_none() {
        *g.unsupported_calls.entry(f.name()).or_insert(0) += 1;
    }
    match t.fns[f.0 as usize] {
        Some(func) => {
            let a = A { v: args, f };
            if t.geometry[f.0 as usize] {
                // edits sprites (masks/origins): instance boxes may change
                g.spatial_invalidate();
            }
            if g.prof.is_some() {
                let t0 = std::time::Instant::now();
                let r = func(g, vm, ctx, &a);
                crate::profile::Profiler::add(&mut g.prof.as_mut().unwrap().funcs, f.0, t0);
                return r;
            }
            func(g, vm, ctx, &a)
        }
        None => Err(Error::new(format!("Function {} is not supported by this runner.", f.name()))),
    }
}

/// Coverage: (implemented by the runner, stubbed, native in gml, total).
pub fn coverage() -> (usize, usize, usize, usize) {
    let t = table();
    let n = gml::builtins::FUNCTIONS.len();
    let mut native = 0;
    let mut implemented = 0;
    let mut stubbed = 0;
    for i in 0..n {
        let id = FunctionId(i as u16);
        if gml::is_native(id) {
            native += 1;
        } else if t.fns[i].is_some() {
            if t.stubbed[i] {
                stubbed += 1;
            } else {
                implemented += 1;
            }
        }
    }
    (implemented, stubbed, native, n)
}

/// Names of functions that are neither native nor implemented.
pub fn missing() -> Vec<&'static str> {
    let t = table();
    (0..gml::builtins::FUNCTIONS.len())
        .filter(|&i| !gml::is_native(FunctionId(i as u16)) && t.fns[i].is_none())
        .map(|i| gml::builtins::FUNCTIONS[i].name)
        .collect()
}

/// Resolves a GM "object" argument against the context (self/other/all/obj/id).
pub fn targets(g: &Game, ctx: &Ctx, obj: i32) -> Vec<i32> {
    g.select_ctx(ctx, obj)
}
