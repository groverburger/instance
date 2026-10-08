//! A mock game world implementing `gml::Host` for the integration tests.
#![allow(dead_code)]

use gml::builtins::{functions as f, variables as v};
use gml::*;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Inst {
    pub id: i32,
    pub object: i32,
    pub vars: VarTable,
    pub builtins: HashMap<(u16, i32), Value>,
    pub alive: bool,
}

#[derive(Default)]
pub struct Mock {
    pub insts: Vec<Inst>,
    /// object -> parent object
    pub parents: HashMap<i32, i32>,
    pub objects: HashMap<String, i32>,
    pub constants: HashMap<String, Value>,
    pub script_names: HashMap<String, i32>,
    pub scripts: HashMap<i32, Rc<Program>>,
    pub global_builtins: HashMap<(u16, i32), Value>,
    pub log: Vec<String>,
    pub next_id: i32,
    /// When `Some`, errors reported through `Host::soft_error` are recorded here
    /// and execution continues (as gm-runner does); when `None` they abort.
    pub soft_errors: Option<Vec<String>>,
}

impl Mock {
    pub fn new() -> Self {
        let mut m = Mock {
            next_id: 100000,
            ..Default::default()
        };
        m.objects.insert("obj_a".into(), 0);
        m.objects.insert("obj_b".into(), 1);
        m.objects.insert("obj_child".into(), 2);
        m.parents.insert(2, 1);
        m
    }

    pub fn add(&mut self, object: i32) -> i32 {
        let id = self.next_id;
        self.next_id += 1;
        self.insts.push(Inst {
            id,
            object,
            vars: VarTable::new(),
            builtins: HashMap::new(),
            alive: true,
        });
        id
    }

    pub fn inst(&mut self, id: i32) -> Option<&mut Inst> {
        self.insts.iter_mut().find(|i| i.id == id)
    }

    fn is_a(&self, mut obj: i32, target: i32) -> bool {
        loop {
            if obj == target {
                return true;
            }
            match self.parents.get(&obj) {
                Some(&p) => obj = p,
                None => return false,
            }
        }
    }

    /// Registers and compiles scripts (names first, so scripts can call each other).
    pub fn compile_scripts(&mut self, vm: &mut Vm, sources: &[(&str, &str)]) {
        for (name, _) in sources {
            let idx = self.script_names.len() as i32;
            self.script_names.insert(name.to_string(), idx);
        }
        for (name, src) in sources {
            let p = vm
                .compile(name, src.as_bytes(), self)
                .unwrap_or_else(|e| panic!("{}: {}", name, e));
            let idx = self.script_names[*name];
            self.scripts.insert(idx, p);
        }
    }

    pub fn var(&mut self, vm: &mut Vm, id: i32, name: &str) -> Option<Value> {
        let vid = vm.intern(name);
        self.inst(id)?.vars.get_value(vid).cloned()
    }
}

impl Host for Mock {
    fn resolve_constant(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.constants.get(name) {
            return Some(v.clone());
        }
        if let Some(&o) = self.objects.get(name) {
            return Some(Value::Real(o as f64));
        }
        self.script_names.get(name).map(|&i| Value::Real(i as f64))
    }

    fn resolve_script(&self, name: &str) -> Option<i32> {
        self.script_names.get(name).copied()
    }

    fn instance_vars(&mut self, id: i32) -> Option<&mut VarTable> {
        self.inst(id).map(|i| &mut i.vars)
    }

    fn instance_exists(&mut self, id: i32) -> bool {
        self.insts.iter().any(|i| i.id == id && i.alive)
    }

    fn select_instances(&mut self, target: i32, out: &mut Vec<i32>) {
        for i in &self.insts {
            if !i.alive {
                continue;
            }
            let ok = if target == special::ALL {
                true
            } else if target >= special::FIRST_INSTANCE_ID {
                i.id == target
            } else {
                self.is_a(i.object, target)
            };
            if ok {
                out.push(i.id);
            }
        }
    }

    fn get_builtin(
        &mut self,
        _ctx: &Ctx,
        inst: i32,
        var: VariableId,
        index: i32,
    ) -> Result<Value, Error> {
        if var.info().scope == builtins::VarScope::Global {
            if var == v::INSTANCE_COUNT {
                return Ok(Value::Real(
                    self.insts.iter().filter(|i| i.alive).count() as f64
                ));
            }
            return Ok(self
                .global_builtins
                .get(&(var.0, index))
                .cloned()
                .unwrap_or_default());
        }
        let Some(i) = self.inst(inst) else {
            return Err(Error::new(format!("no instance {}", inst)));
        };
        if var == v::ID {
            return Ok(Value::Real(i.id as f64));
        }
        if var == v::OBJECT_INDEX {
            return Ok(Value::Real(i.object as f64));
        }
        Ok(i.builtins.get(&(var.0, index)).cloned().unwrap_or_default())
    }

    fn set_builtin(
        &mut self,
        _ctx: &Ctx,
        inst: i32,
        var: VariableId,
        index: i32,
        value: Value,
    ) -> Result<(), Error> {
        if var.info().scope == builtins::VarScope::Global {
            if var == v::ROOM_CAPTION {
                if !value.is_str() {
                    return Err(Error::new("Wrong type of value assigned"));
                }
            } else if !value.is_real() {
                return Err(Error::new("Wrong type of value assigned"));
            }
            self.global_builtins.insert((var.0, index), value);
            return Ok(());
        }
        if !value.is_real() {
            return Err(Error::new("Wrong type of value assigned"));
        }
        let Some(i) = self.inst(inst) else {
            return Err(Error::new("no instance"));
        };
        i.builtins.insert((var.0, index), value);
        Ok(())
    }

    fn call_function(
        &mut self,
        vm: &mut Vm,
        ctx: &Ctx,
        func: FunctionId,
        args: &[Value],
    ) -> Result<Value, Error> {
        match func {
            f::SHOW_DEBUG_MESSAGE => {
                self.log.push(args[0].to_string());
                Ok(Value::Real(0.0))
            }
            f::INSTANCE_CREATE => {
                let obj = args[2].real_or_zero() as i32;
                let id = self.add(obj);
                let i = self.inst(id).unwrap();
                i.builtins.insert((v::X.0, 0), args[0].clone());
                i.builtins.insert((v::Y.0, 0), args[1].clone());
                // Run a "create event" for objects that have one (re-entrancy test).
                if let Some(p) = self
                    .scripts
                    .get(&self.script_names.get("create_event").copied().unwrap_or(-1))
                    .cloned()
                {
                    vm.run(self, &p, Ctx::new(id, ctx.self_id), &[])?;
                }
                Ok(Value::Real(id as f64))
            }
            f::INSTANCE_DESTROY => {
                if let Some(i) = self.inst(ctx.self_id) {
                    i.alive = false;
                }
                Ok(Value::Real(0.0))
            }
            f::INSTANCE_NUMBER => {
                let mut v = Vec::new();
                self.select_instances(args[0].real_or_zero() as i32, &mut v);
                Ok(Value::Real(v.len() as f64))
            }
            f::GAME_END => Err(Error::halt()),
            _ => Err(Error::new(format!("unsupported function {}", func.name()))),
        }
    }

    fn soft_error(&mut self, e: Error) -> Result<(), Error> {
        match &mut self.soft_errors {
            Some(v) => {
                v.push(e.message);
                Ok(())
            }
            None => Err(e),
        }
    }

    fn call_script(
        &mut self,
        vm: &mut Vm,
        ctx: &Ctx,
        script: i32,
        args: &[Value],
    ) -> Result<Value, Error> {
        let p = self
            .scripts
            .get(&script)
            .cloned()
            .ok_or_else(|| Error::new(format!("Unknown script {}", script)))?;
        vm.run_script(self, &p, *ctx, args)
    }
}

/// Runs `src` as a code action of a fresh instance 100000 of `obj_a`.
pub fn run(src: &str) -> Result<Value, Error> {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    run_in(&mut vm, &mut h, src)
}

pub fn run_in(vm: &mut Vm, h: &mut Mock, src: &str) -> Result<Value, Error> {
    if h.insts.is_empty() {
        h.add(0);
    }
    let prog = vm.compile("test", src.as_bytes(), h).map_err(Error::from)?;
    let me = h.insts[0].id;
    vm.run(h, &prog, Ctx::new(me, me), &[])
}

pub fn ok(src: &str) -> Value {
    run(src).unwrap_or_else(|e| panic!("{}\n=> {}", src, e))
}

pub fn real(src: &str) -> f64 {
    match ok(src) {
        Value::Real(x) => x,
        other => panic!("{} => {:?}", src, other),
    }
}

pub fn string(src: &str) -> String {
    match ok(src) {
        Value::Str(s) => s.to_string_lossy(),
        other => panic!("{} => {:?}", src, other),
    }
}

pub fn err(src: &str) -> Error {
    match run(src) {
        Ok(v) => panic!("expected error from {}, got {:?}", src, v),
        Err(e) => e,
    }
}

pub fn compile_err(src: &str) -> CompileError {
    let (mut vm, h) = (Vm::new(), Mock::new());
    match vm.compile("test", src.as_bytes(), &h) {
        Ok(_) => panic!("expected compile error from {}", src),
        Err(e) => e,
    }
}
