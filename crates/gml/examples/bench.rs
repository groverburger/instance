//! Micro-benchmarks for the interpreter: `cargo run --release -p gml --example bench`.
use gml::*;

struct World {
    me: VarTable,
}

impl Host for World {
    fn instance_vars(&mut self, id: i32) -> Option<&mut VarTable> {
        (id == 100000).then_some(&mut self.me)
    }
    fn instance_exists(&mut self, id: i32) -> bool {
        id == 100000
    }
    fn select_instances(&mut self, target: i32, out: &mut Vec<i32>) {
        if target == special::ALL || target == 0 || target == 100000 {
            out.push(100000)
        }
    }
    fn get_builtin(&mut self, _: &Ctx, _: i32, _: VariableId, _: i32) -> Result<Value, Error> {
        Ok(Value::Real(0.0))
    }
    fn set_builtin(
        &mut self,
        _: &Ctx,
        _: i32,
        _: VariableId,
        _: i32,
        _: Value,
    ) -> Result<(), Error> {
        Ok(())
    }
    fn call_function(
        &mut self,
        _: &mut Vm,
        _: &Ctx,
        f: FunctionId,
        _: &[Value],
    ) -> Result<Value, Error> {
        Err(Error::new(format!("{} not supported", f.name())))
    }
    fn call_script(&mut self, _: &mut Vm, _: &Ctx, _: i32, _: &[Value]) -> Result<Value, Error> {
        Err(Error::new("no scripts"))
    }
}

fn main() {
    let cases = [
        ("empty repeat", "repeat 1000000 {}"),
        ("repeat const assign", "var n; repeat 1000000 n = 1; return n"),
        ("while local", "var i; i = 0; while i < 1000000 i += 1; return i"),
        ("local for loop", "var i, s; s = 0; for (i = 0; i < 1000000; i += 1) s += i; return s"),
        ("instance var loop", "s = 0; for (i = 0; i < 1000000; i += 1) s += i; return s"),
        ("repeat", "var n; n = 0; repeat 1000000 n += 1; return n"),
        ("array writes", "var i; for (i = 0; i < 1000000; i += 1) a[i mod 1000] = i; return a[999]"),
        ("function calls", "var i, s; s = 0; for (i = 0; i < 1000000; i += 1) s += abs(-i); return s"),
        ("string concat", "var i, s; s = ''; for (i = 0; i < 100000; i += 1) s = 'x' + string(i mod 10); return s"),
    ];
    for (name, src) in cases {
        let mut w = World {
            me: VarTable::new(),
        };
        let mut vm = Vm::new();
        let p = vm.compile(name, src.as_bytes(), &w).unwrap();
        let mut best = f64::MAX;
        let mut r = Value::Real(0.0);
        for _ in 0..7 {
            let t = std::time::Instant::now();
            r = vm.run(&mut w, &p, Ctx::new(100000, 100000), &[]).unwrap();
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        }
        println!("{:20} {:>8.1} ms (best of 7) -> {}", name, best, r);
    }
}
