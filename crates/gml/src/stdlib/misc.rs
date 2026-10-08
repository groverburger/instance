//! Variables by name (403_11) and code execution (408_06 / 409_06).

use super::{n, Args, NResult, NativeFn};
use crate::builtins::{self, VarScope};
use crate::error::Error;
use crate::host::{Ctx, Host};
use crate::value::Value;
use crate::vars::MAX_INDEX;
use crate::vm::Vm;

fn index(a: &Args, i: usize) -> Result<usize, Error> {
    let v = a.int(i)?;
    if v < 0 || v as usize >= MAX_INDEX {
        return Err(Error::new(format!(
            "Array index out of range in {}.",
            a.func.name()
        )));
    }
    Ok(v as usize)
}

fn name<'a>(a: &Args<'a>) -> Result<&'a str, Error> {
    std::str::from_utf8(a.str(0)?).map_err(|_| a.invalid())
}

#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Global,
    Local,
}

fn builtin_for(name: &str, scope: Scope) -> Option<builtins::VariableId> {
    let id = builtins::variable_id(name)?;
    match (id.info().scope, scope) {
        (VarScope::Global, Scope::Global) | (VarScope::Instance, Scope::Local) => Some(id),
        _ => None,
    }
}

fn exists(vm: &mut Vm, h: &mut dyn Host, c: &Ctx, a: Args, scope: Scope) -> NResult {
    let nm = name(&a)?;
    if builtin_for(nm, scope).is_some() {
        return Ok(Value::Real(1.0));
    }
    let Some(id) = vm.lookup_var(nm) else {
        return Ok(Value::Real(0.0));
    };
    let found = match scope {
        Scope::Global => vm.globals.contains(id),
        Scope::Local => h
            .instance_vars(c.self_id)
            .map(|t| t.contains(id))
            .unwrap_or(false),
    };
    Ok(Value::from(found))
}

fn get(
    vm: &mut Vm,
    h: &mut dyn Host,
    c: &Ctx,
    a: Args,
    scope: Scope,
    i: usize,
    j: usize,
) -> NResult {
    let nm = name(&a)?;
    if let Some(b) = builtin_for(nm, scope) {
        let idx = if i == 0 {
            j as i32
        } else {
            return Err(a.invalid());
        };
        return h.get_builtin(c, c.self_id, b, idx);
    }
    let id = vm.intern(nm);
    let v = match scope {
        Scope::Global => vm.globals.get_elem(id, i, j).cloned(),
        Scope::Local => h
            .instance_vars(c.self_id)
            .and_then(|t| t.get_elem(id, i, j).cloned()),
    };
    match v {
        Some(v) => Ok(v),
        None if vm.settings.uninitialized_as_zero => Ok(Value::Real(0.0)),
        None => Err(Error::new(format!("Unknown variable {}", nm))),
    }
}

#[allow(clippy::too_many_arguments)]
fn set(
    vm: &mut Vm,
    h: &mut dyn Host,
    c: &Ctx,
    a: Args,
    scope: Scope,
    i: usize,
    j: usize,
    v: Value,
) -> NResult {
    let nm = name(&a)?;
    if let Some(b) = builtin_for(nm, scope) {
        if b.info().readonly {
            return Err(Error::new(format!(
                "Cannot assign to the read-only variable {}.",
                nm
            )));
        }
        let idx = if i == 0 {
            j as i32
        } else {
            return Err(a.invalid());
        };
        h.set_builtin(c, c.self_id, b, idx, v)?;
        return Ok(Value::Real(0.0));
    }
    let id = vm.intern(nm);
    let r = match scope {
        Scope::Global => vm.globals.set_elem(id, i, j, v),
        Scope::Local => match h.instance_vars(c.self_id) {
            Some(t) => t.set_elem(id, i, j, v),
            None => Err("The current instance does not exist.".into()),
        },
    };
    r.map_err(Error::new)?;
    Ok(Value::Real(0.0))
}

pub(super) fn register(t: &mut Vec<(&'static str, NativeFn)>) {
    t.extend([
        n("variable_global_exists", |vm, h, c, a| {
            exists(vm, h, c, a, Scope::Global)
        }),
        n("variable_global_get", |vm, h, c, a| {
            get(vm, h, c, a, Scope::Global, 0, 0)
        }),
        n("variable_global_array_get", |vm, h, c, a| {
            let j = index(&a, 1)?;
            get(vm, h, c, a, Scope::Global, 0, j)
        }),
        n("variable_global_array2_get", |vm, h, c, a| {
            let (i, j) = (index(&a, 1)?, index(&a, 2)?);
            get(vm, h, c, a, Scope::Global, i, j)
        }),
        n("variable_global_set", |vm, h, c, a| {
            set(vm, h, c, a, Scope::Global, 0, 0, a.val(1).clone())
        }),
        n("variable_global_array_set", |vm, h, c, a| {
            let j = index(&a, 1)?;
            set(vm, h, c, a, Scope::Global, 0, j, a.val(2).clone())
        }),
        n("variable_global_array2_set", |vm, h, c, a| {
            let (i, j) = (index(&a, 1)?, index(&a, 2)?);
            set(vm, h, c, a, Scope::Global, i, j, a.val(3).clone())
        }),
        n("variable_local_exists", |vm, h, c, a| {
            exists(vm, h, c, a, Scope::Local)
        }),
        n("variable_local_get", |vm, h, c, a| {
            get(vm, h, c, a, Scope::Local, 0, 0)
        }),
        n("variable_local_array_get", |vm, h, c, a| {
            let j = index(&a, 1)?;
            get(vm, h, c, a, Scope::Local, 0, j)
        }),
        n("variable_local_array2_get", |vm, h, c, a| {
            let (i, j) = (index(&a, 1)?, index(&a, 2)?);
            get(vm, h, c, a, Scope::Local, i, j)
        }),
        n("variable_local_set", |vm, h, c, a| {
            set(vm, h, c, a, Scope::Local, 0, 0, a.val(1).clone())
        }),
        n("variable_local_array_set", |vm, h, c, a| {
            let j = index(&a, 1)?;
            set(vm, h, c, a, Scope::Local, 0, j, a.val(2).clone())
        }),
        n("variable_local_array2_set", |vm, h, c, a| {
            let (i, j) = (index(&a, 1)?, index(&a, 2)?);
            set(vm, h, c, a, Scope::Local, i, j, a.val(3).clone())
        }),
        n("script_execute", |vm, h, c, a| {
            let idx = a.int(0)?;
            let ctx = Ctx {
                argument_relative: false,
                ..*c
            };
            vm.via_script_execute = true;
            let r = h.call_script(
                vm,
                &ctx,
                idx.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                &a.vals[1..],
            );
            vm.via_script_execute = false;
            r
        }),
        n("execute_string", |vm, h, c, a| {
            let src = a.str(0)?;
            let ctx = Ctx {
                argument_relative: false,
                ..*c
            };
            let r = vm.execute_string(h, src, ctx, &a.vals[1..]);
            isolated(h, r)
        }),
        n("execute_file", |vm, h, c, a| {
            let path = String::from_utf8_lossy(a.str(0)?).into_owned();
            let src = h.read_file(&path)?;
            let prog = match vm.compile(&path, &src, &*h) {
                Ok(p) => p,
                Err(e) => return isolated(h, Err(Error::from(e))),
            };
            let ctx = Ctx {
                argument_relative: false,
                ..*c
            };
            let r = vm.run(h, &prog, ctx, &a.vals[1..]);
            isolated(h, r)
        }),
    ]);
}

/// Errors inside `execute_string` / `execute_file` code do not abandon the caller:
/// they are reported and the call returns 0 (VERIFIED black-box, bb_lang
/// type.readonly_id, var.shadow_x).
fn isolated(h: &mut dyn Host, r: Result<Value, Error>) -> Result<Value, Error> {
    match r {
        Err(e) if e.kind == crate::error::ErrorKind::Runtime => {
            h.soft_error(e)?;
            Ok(Value::Real(0.0))
        }
        r => r,
    }
}
