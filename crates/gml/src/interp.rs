//! Tree-walking interpreter over the resolved execution tree.
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

use crate::builtins::{VarScope, VariableId};
use crate::compile::{ArgVar, Ex, FieldName, Idx, Place, PlaceKind, Pos, St};
use crate::error::{Error, ErrorKind, Location};
use crate::host::{special, Ctx, Host};
use crate::stdlib::{self as natives, Args};
use crate::value::{binary, to_int, unary, BinOp, Value};
use crate::vars::{Var, VarId, MAX_INDEX};
use crate::vm::Vm;
use crate::Program;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    Normal,
    Break,
    Continue,
    Exit,
    Return,
}

pub(crate) struct Frame<'p> {
    pub prog: &'p Program,
    pub locals: Vec<Option<Var>>,
    pub args: [Value; 16],
    pub argc: usize,
    pub ctx: Ctx,
    pub ret: Value,
}

type R<T> = Result<T, Box<Error>>;

enum ReadOwner {
    Inst(i32),
    Global,
    Local,
}

impl Vm {
    pub(crate) fn run_program(
        &mut self,
        h: &mut dyn Host,
        prog: &Program,
        ctx: Ctx,
        args: &[Value],
    ) -> Result<Value, Error> {
        if self.depth >= self.settings.max_call_depth {
            return Err(Error::new(
                "Too many nested script calls (recursion too deep).",
            ));
        }
        self.depth += 1;
        let mut locals = self.locals_pool.pop().unwrap_or_default();
        locals.resize_with(prog.local_names.len(), || None);
        let mut f = Frame {
            prog,
            locals,
            args: Default::default(),
            argc: args.len().min(16),
            ctx,
            ret: Value::Real(0.0),
        };
        for (k, a) in args.iter().take(16).enumerate() {
            f.args[k] = a.clone();
        }
        let r = self.exec_block(h, &mut f, &prog.body);
        self.depth -= 1;
        let mut locals = std::mem::take(&mut f.locals);
        locals.clear();
        self.locals_pool.push(locals);
        match r {
            Ok(Flow::Return) => Ok(std::mem::take(&mut f.ret)),
            Ok(_) => Ok(Value::Real(0.0)),
            Err(e) => Err(*e),
        }
    }

    // ------------------------------------------------------------ errors

    #[cold]
    fn loc(&self, f: &Frame, pos: Pos, mut e: Error) -> Box<Error> {
        if e.location.is_none() && e.kind != ErrorKind::Halt {
            e.location = Some(Location {
                program: f.prog.name.clone(),
                line: pos.line,
                col: pos.col,
            });
        }
        Box::new(e)
    }

    #[cold]
    fn fail<T>(&self, f: &Frame, pos: Pos, msg: impl Into<String>) -> R<T> {
        Err(self.loc(f, pos, Error::new(msg)))
    }

    #[cold]
    fn uninit(&self, f: &Frame, var: VarId, i: usize, j: usize, pos: Pos) -> R<Value> {
        if self.settings.uninitialized_as_zero {
            return Ok(Value::Real(0.0));
        }
        let name = self.interner.name(var);
        let msg = if i == 0 && j == 0 {
            format!("Unknown variable {}", name)
        } else if i == 0 {
            format!("Unknown variable {}[{}]", name, j)
        } else {
            format!("Unknown variable {}[{},{}]", name, i, j)
        };
        self.fail(f, pos, msg)
    }

    #[inline(always)]
    fn bin(&self, f: &Frame, pos: Pos, op: BinOp, l: &Value, r: &Value) -> R<Value> {
        if let (Value::Real(a), Value::Real(b)) = (l, r) {
            let (a, b) = (*a, *b);
            let v = match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div if b != 0.0 => a / b,
                BinOp::Mod if b != 0.0 => a % b,
                BinOp::Lt => (a < b) as u8 as f64,
                BinOp::Le => (a <= b) as u8 as f64,
                BinOp::Gt => (a > b) as u8 as f64,
                BinOp::Ge => (a >= b) as u8 as f64,
                BinOp::Eq => (a == b) as u8 as f64,
                BinOp::Ne => (a != b) as u8 as f64,
                _ => return self.bin_slow(f, pos, op, l, r),
            };
            return Ok(Value::Real(v));
        }
        self.bin_slow(f, pos, op, l, r)
    }

    fn bin_slow(&self, f: &Frame, pos: Pos, op: BinOp, l: &Value, r: &Value) -> R<Value> {
        match binary(op, l, r) {
            Ok(v) => Ok(v),
            Err(m) => self.fail(f, pos, m),
        }
    }

    // ------------------------------------------------------------ statements

    pub(crate) fn exec_block(&mut self, h: &mut dyn Host, f: &mut Frame, body: &[St]) -> R<Flow> {
        for s in body {
            let fl = self.exec(h, f, s)?;
            if fl != Flow::Normal {
                return Ok(fl);
            }
        }
        Ok(Flow::Normal)
    }

    fn exec(&mut self, h: &mut dyn Host, f: &mut Frame, s: &St) -> R<Flow> {
        match s {
            St::Nop => {}
            St::Block(b) => return self.exec_block(h, f, b),
            St::AssignLocal(slot, op, e, pos) => self.assign_local(h, f, *slot, *op, e, *pos)?,
            St::Assign(p, op, e) => self.assign(h, f, p, *op, e)?,
            St::Expr(e) => {
                self.eval(h, f, e)?;
            }
            St::If(c, t, e) => {
                if self.eval(h, f, c)?.is_true() {
                    return self.exec(h, f, t);
                } else if let Some(e) = e {
                    return self.exec(h, f, e);
                }
            }
            St::Repeat(c, body, pos) => return self.exec_repeat(h, f, c, body, *pos),
            St::While(c, body) => return self.exec_while(h, f, c, body),
            St::Do(body, c) => return self.exec_do(h, f, body, c),
            St::For(init, c, step, body) => return self.exec_for(h, f, init, c, step, body),
            St::Switch(sw) => return self.exec_switch(h, f, sw),
            St::With(e, body, pos) => return self.exec_with(h, f, e, body, *pos),
            St::GlobalVar(vars) => {
                for &v in vars {
                    self.declare_globalvar(v);
                }
            }
            St::Break => return Ok(Flow::Break),
            St::Continue => return Ok(Flow::Continue),
            St::Exit => return Ok(Flow::Exit),
            St::Return(e) => {
                f.ret = self.eval(h, f, e)?;
                return Ok(Flow::Return);
            }
        }
        Ok(Flow::Normal)
    }

    #[inline]
    fn assign_local(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        slot: u16,
        op: Option<BinOp>,
        e: &Ex,
        pos: Pos,
    ) -> R<()> {
        let v = match op {
            None => self.eval(h, f, e)?,
            Some(op) => {
                let r = self.eval(h, f, e)?;
                // Fast path: real op= real on a scalar local, updated in place.
                if let (Some(Var::Scalar(Value::Real(cur))), Value::Real(rv)) =
                    (&mut f.locals[slot as usize], &r)
                {
                    let done = match op {
                        BinOp::Add => Some(*cur + *rv),
                        BinOp::Sub => Some(*cur - *rv),
                        BinOp::Mul => Some(*cur * *rv),
                        _ => None,
                    };
                    if let Some(x) = done {
                        *cur = x;
                        return Ok(());
                    }
                }
                let cur = self.read_local(f, slot, 0, 0, pos)?;
                self.bin(f, pos, op, &cur, &r)?
            }
        };
        match &mut f.locals[slot as usize] {
            Some(Var::Scalar(s)) => *s = v,
            Some(var) => {
                let _ = var.set(0, 0, v);
            }
            none => *none = Some(Var::Scalar(v)),
        }
        Ok(())
    }

    #[inline(never)]
    fn exec_repeat(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        c: &Ex,
        body: &St,
        pos: Pos,
    ) -> R<Flow> {
        let n = match self.eval(h, f, c)? {
            Value::Real(x) => to_int(x),
            Value::Str(_) => return self.fail(f, pos, "Repeat count must be a number."),
        };
        let mut k = 0;
        while k < n {
            match self.exec(h, f, body)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => break,
                other => return Ok(other),
            }
            k += 1;
        }
        Ok(Flow::Normal)
    }

    #[inline(never)]
    fn exec_while(&mut self, h: &mut dyn Host, f: &mut Frame, c: &Ex, body: &St) -> R<Flow> {
        while self.eval(h, f, c)?.is_true() {
            match self.exec(h, f, body)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => break,
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    #[inline(never)]
    fn exec_do(&mut self, h: &mut dyn Host, f: &mut Frame, body: &St, c: &Ex) -> R<Flow> {
        loop {
            match self.exec(h, f, body)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => break,
                other => return Ok(other),
            }
            if self.eval(h, f, c)?.is_true() {
                break;
            }
        }
        Ok(Flow::Normal)
    }

    #[inline(never)]
    fn exec_for(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        init: &St,
        c: &Ex,
        step: &St,
        body: &St,
    ) -> R<Flow> {
        match self.exec(h, f, init)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
        while self.eval(h, f, c)?.is_true() {
            match self.exec(h, f, body)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => break,
                other => return Ok(other),
            }
            match self.exec(h, f, step)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    #[inline(never)]
    fn exec_switch(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        sw: &crate::compile::Switch,
    ) -> R<Flow> {
        let v = self.eval(h, f, &sw.value)?;
        let mut start = None;
        for (ce, target) in &sw.cases {
            let cv = self.eval(h, f, ce)?;
            if cv == v {
                start = Some(*target);
                break;
            }
        }
        if let Some(st) = start.or(sw.default) {
            for s in &sw.body[st..] {
                match self.exec(h, f, s)? {
                    Flow::Normal => {}
                    Flow::Break => break,
                    other => return Ok(other),
                }
            }
        }
        Ok(Flow::Normal)
    }

    #[inline(never)]
    fn exec_with(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        e: &Ex,
        body: &St,
        pos: Pos,
    ) -> R<Flow> {
        let ov = self.eval(h, f, e)?;
        let o = self.owner_id(f, &ov, pos)?;
        let mut targets = self.select_pool.pop().unwrap_or_default();
        let mut check = true;
        match o {
            special::SELF => {
                targets.push(f.ctx.self_id);
                check = false;
            }
            special::OTHER => {
                if f.ctx.other_id != special::NOONE {
                    targets.push(f.ctx.other_id);
                }
                check = false;
            }
            special::ALL => h.select_instances(special::ALL, &mut targets),
            x if x >= 0 => h.select_instances(x, &mut targets),
            _ => {}
        }
        let saved = f.ctx;
        let mut result = Ok(Flow::Normal);
        for k in 0..targets.len() {
            let t = targets[k];
            if check && k > 0 && !h.instance_exists(t) {
                continue;
            }
            f.ctx.other_id = saved.self_id;
            f.ctx.self_id = t;
            match self.exec(h, f, body) {
                Ok(Flow::Normal) | Ok(Flow::Continue) => {}
                Ok(Flow::Break) => break,
                other => {
                    result = other;
                    break;
                }
            }
        }
        f.ctx = saved;
        targets.clear();
        self.select_pool.push(targets);
        result
    }

    // ------------------------------------------------------------ expressions

    pub(crate) fn eval(&mut self, h: &mut dyn Host, f: &Frame, e: &Ex) -> R<Value> {
        match e {
            Ex::Const(v) => Ok(v.clone()),
            Ex::Local(s, pos) => self.read_local(f, *s, 0, 0, *pos),
            Ex::User(v, pos) => self.read_user(h, f, *v, 0, 0, *pos),
            Ex::Builtin(b, pos) => h
                .get_builtin(&f.ctx, f.ctx.self_id, *b, 0)
                .map_err(|e| self.loc(f, *pos, e)),
            Ex::Arg(n) => Ok(f.args[*n as usize].clone()),
            Ex::Place(p) => self.read_place(h, f, p),
            Ex::Unary(op, x, pos) => {
                let v = self.eval(h, f, x)?;
                match unary(*op, &v) {
                    Ok(v) => Ok(v),
                    Err(m) => self.fail(f, *pos, m),
                }
            }
            Ex::Binary(op, a, b, pos) => {
                let l = self.eval(h, f, a)?;
                if let Ex::Const(c) = &**b {
                    return self.bin(f, *pos, *op, &l, c);
                }
                let r = self.eval(h, f, b)?;
                self.bin(f, *pos, *op, &l, &r)
            }
            Ex::Call(fid, args, pos) => self.eval_call(h, f, *fid, args, *pos),
            Ex::Script(idx, args, pos) => self.eval_script(h, f, *idx, args, *pos),
        }
    }

    #[inline(never)]
    fn eval_call(
        &mut self,
        h: &mut dyn Host,
        f: &Frame,
        fid: crate::FunctionId,
        args: &[Ex],
        pos: Pos,
    ) -> R<Value> {
        // The argument count was checked at compile time, so natives are called
        // directly; up to four arguments live on the stack.
        if args.len() <= 4 {
            let mut buf: [Value; 4] = Default::default();
            for (k, a) in args.iter().enumerate() {
                buf[k] = self.eval(h, f, a)?;
            }
            let vals = &buf[..args.len()];
            let r = match natives::native(fid) {
                Some(nf) => nf(self, h, &f.ctx, Args { vals, func: fid }),
                None => h.call_function(self, &f.ctx, fid, vals),
            };
            return r.map_err(|e| self.loc(f, pos, e));
        }
        let buf = self.eval_args(h, f, args)?;
        let r = self.call_function(h, &f.ctx, fid, &buf);
        self.recycle(buf);
        r.map_err(|e| self.loc(f, pos, e))
    }

    #[inline(never)]
    fn eval_script(
        &mut self,
        h: &mut dyn Host,
        f: &Frame,
        idx: i32,
        args: &[Ex],
        pos: Pos,
    ) -> R<Value> {
        let buf = self.eval_args(h, f, args)?;
        let ctx = Ctx {
            argument_relative: false,
            ..f.ctx
        };
        let r = h.call_script(self, &ctx, idx, &buf);
        self.recycle(buf);
        r.map_err(|e| self.loc(f, pos, e))
    }

    fn eval_args(&mut self, h: &mut dyn Host, f: &Frame, args: &[Ex]) -> R<Vec<Value>> {
        let mut buf = self.arg_pool.pop().unwrap_or_default();
        for a in args {
            match self.eval(h, f, a) {
                Ok(v) => buf.push(v),
                Err(e) => {
                    self.recycle(buf);
                    return Err(e);
                }
            }
        }
        Ok(buf)
    }

    fn recycle(&mut self, mut buf: Vec<Value>) {
        buf.clear();
        self.arg_pool.push(buf);
    }

    // ------------------------------------------------------------ indices and owners

    fn index_value(&mut self, h: &mut dyn Host, f: &Frame, e: &Ex, pos: Pos) -> R<usize> {
        match self.eval(h, f, e)? {
            Value::Real(x) => {
                let n = to_int(x);
                if n < 0 {
                    self.fail(f, pos, "Negative array index.")
                } else if n as usize >= MAX_INDEX {
                    self.fail(f, pos, "Array index must be less than 32000.")
                } else {
                    Ok(n as usize)
                }
            }
            Value::Str(_) => self.fail(f, pos, "Array index must be a number."),
        }
    }

    fn eval_idx(&mut self, h: &mut dyn Host, f: &Frame, idx: &Idx, pos: Pos) -> R<(usize, usize)> {
        match idx {
            Idx::None => Ok((0, 0)),
            Idx::One(a) => Ok((0, self.index_value(h, f, a, pos)?)),
            Idx::Two(a, b) => {
                let i = self.index_value(h, f, a, pos)?;
                Ok((i, self.index_value(h, f, b, pos)?))
            }
        }
    }

    fn owner_id(&self, f: &Frame, v: &Value, pos: Pos) -> R<i32> {
        match v {
            Value::Real(x) => Ok(to_int(*x).clamp(i32::MIN as i64, i32::MAX as i64) as i32),
            Value::Str(_) => self.fail(f, pos, "An instance or object was expected, not a string."),
        }
    }

    fn read_owner(&mut self, h: &mut dyn Host, f: &Frame, o: i32, pos: Pos) -> R<ReadOwner> {
        Ok(match o {
            special::SELF => ReadOwner::Inst(f.ctx.self_id),
            special::OTHER => {
                if f.ctx.other_id == special::NOONE {
                    return self.fail(f, pos, "There is no other instance.");
                }
                ReadOwner::Inst(f.ctx.other_id)
            }
            special::ALL => match h.first_instance(special::ALL) {
                Some(i) => ReadOwner::Inst(i),
                None => return self.fail(f, pos, "There are no instances."),
            },
            special::NOONE => return self.fail(f, pos, "Cannot read a variable of noone."),
            special::GLOBAL => ReadOwner::Global,
            special::LOCAL => ReadOwner::Local,
            x if x >= special::FIRST_INSTANCE_ID => ReadOwner::Inst(x),
            x if x >= 0 => match h.first_instance(x) {
                Some(i) => ReadOwner::Inst(i),
                None => {
                    return self.fail(
                        f,
                        pos,
                        format!(
                            "Unknown variable access: no instance of object {} exists.",
                            x
                        ),
                    )
                }
            },
            x => return self.fail(f, pos, format!("Unknown instance or object {}.", x)),
        })
    }

    /// Instances written by `o.name = v`; `None` means global/local handled by caller.
    fn write_targets(&mut self, h: &mut dyn Host, f: &Frame, o: i32, out: &mut Vec<i32>) {
        match o {
            special::SELF => out.push(f.ctx.self_id),
            special::OTHER => {
                if f.ctx.other_id != special::NOONE {
                    out.push(f.ctx.other_id)
                }
            }
            special::ALL => h.select_instances(special::ALL, out),
            x if x >= special::FIRST_INSTANCE_ID => out.push(x),
            x if x >= 0 => h.select_instances(x, out),
            _ => {}
        }
    }

    fn local_slot(f: &Frame, var: VarId) -> Option<u16> {
        f.prog
            .local_names
            .iter()
            .position(|&v| v == var)
            .map(|p| p as u16)
    }

    fn builtin_index(&self, f: &Frame, b: VariableId, i: usize, j: usize, pos: Pos) -> R<i32> {
        let info = b.info();
        let ok = i == 0
            && match info.array {
                Some((lo, hi)) => (j as i64) >= lo as i64 && (j as i64) <= hi as i64,
                None => j == 0,
            };
        if !ok {
            return self.fail(
                f,
                pos,
                format!("Array index out of bounds for variable {}.", info.name),
            );
        }
        Ok(j as i32)
    }

    // ------------------------------------------------------------ reads

    #[inline]
    fn read_local(&self, f: &Frame, slot: u16, i: usize, j: usize, pos: Pos) -> R<Value> {
        match f.locals[slot as usize].as_ref().and_then(|v| v.get(i, j)) {
            Some(v) => Ok(v.clone()),
            None => self.uninit(f, f.prog.local_names[slot as usize], i, j, pos),
        }
    }

    #[inline]
    fn read_global(&self, f: &Frame, var: VarId, i: usize, j: usize, pos: Pos) -> R<Value> {
        match self.globals.get_elem(var, i, j) {
            Some(v) => Ok(v.clone()),
            None => self.uninit(f, var, i, j, pos),
        }
    }

    #[inline]
    fn read_inst(
        &self,
        h: &mut dyn Host,
        f: &Frame,
        inst: i32,
        var: VarId,
        i: usize,
        j: usize,
        pos: Pos,
    ) -> R<Value> {
        match h.instance_vars(inst) {
            Some(t) => match t.get_elem(var, i, j) {
                Some(v) => Ok(v.clone()),
                None => self.uninit(f, var, i, j, pos),
            },
            None => self.fail(
                f,
                pos,
                format!(
                    "Cannot read variable {}: instance {} does not exist.",
                    self.interner.name(var),
                    inst
                ),
            ),
        }
    }

    #[inline]
    fn read_user(
        &self,
        h: &mut dyn Host,
        f: &Frame,
        var: VarId,
        i: usize,
        j: usize,
        pos: Pos,
    ) -> R<Value> {
        if self.is_globalvar(var) {
            self.read_global(f, var, i, j, pos)
        } else {
            self.read_inst(h, f, f.ctx.self_id, var, i, j, pos)
        }
    }

    fn read_arg(&self, f: &Frame, a: ArgVar, i: usize, j: usize, pos: Pos) -> R<Value> {
        match a {
            ArgVar::Slot(n) => {
                if i == 0 && j == 0 {
                    Ok(f.args[n as usize].clone())
                } else {
                    self.fail(f, pos, format!("Unknown variable argument{}[{}]", n, j))
                }
            }
            ArgVar::Array => {
                if i != 0 || j >= 16 {
                    self.fail(f, pos, "Argument index out of range (0..15).")
                } else {
                    Ok(f.args[j].clone())
                }
            }
            ArgVar::Count => Ok(Value::Real(f.argc as f64)),
            ArgVar::Relative => Ok(Value::from(f.ctx.argument_relative)),
        }
    }

    #[inline(never)]
    fn read_place(&mut self, h: &mut dyn Host, f: &Frame, p: &Place) -> R<Value> {
        if let PlaceKind::Field(owner, name) = &p.kind {
            let ov = self.eval(h, f, owner)?;
            let o = self.owner_id(f, &ov, p.pos)?;
            let (i, j) = self.eval_idx(h, f, &p.idx, p.pos)?;
            return self.read_field(h, f, o, *name, i, j, p.pos);
        }
        let (i, j) = self.eval_idx(h, f, &p.idx, p.pos)?;
        self.read_resolved(h, f, &p.kind, i, j, p.pos)
    }

    fn read_resolved(
        &mut self,
        h: &mut dyn Host,
        f: &Frame,
        k: &PlaceKind,
        i: usize,
        j: usize,
        pos: Pos,
    ) -> R<Value> {
        match k {
            PlaceKind::Local(s) => self.read_local(f, *s, i, j, pos),
            PlaceKind::User(v) => self.read_user(h, f, *v, i, j, pos),
            PlaceKind::SelfUser(v) => self.read_inst(h, f, f.ctx.self_id, *v, i, j, pos),
            PlaceKind::Global(v) => self.read_global(f, *v, i, j, pos),
            PlaceKind::Builtin(b) => {
                let idx = self.builtin_index(f, *b, i, j, pos)?;
                h.get_builtin(&f.ctx, f.ctx.self_id, *b, idx)
                    .map_err(|e| self.loc(f, pos, e))
            }
            PlaceKind::Arg(a) => self.read_arg(f, *a, i, j, pos),
            PlaceKind::Field(..) => unreachable!("handled by read_place"),
        }
    }

    fn read_field(
        &mut self,
        h: &mut dyn Host,
        f: &Frame,
        o: i32,
        name: FieldName,
        i: usize,
        j: usize,
        pos: Pos,
    ) -> R<Value> {
        match name {
            FieldName::Arg(a) => self.read_arg(f, a, i, j, pos),
            FieldName::Builtin(b) if b.info().scope == VarScope::Global => {
                let idx = self.builtin_index(f, b, i, j, pos)?;
                h.get_builtin(&f.ctx, f.ctx.self_id, b, idx)
                    .map_err(|e| self.loc(f, pos, e))
            }
            FieldName::Builtin(b) => {
                let idx = self.builtin_index(f, b, i, j, pos)?;
                match self.read_owner(h, f, o, pos)? {
                    ReadOwner::Inst(inst) => {
                        if inst != f.ctx.self_id
                            && !h.instance_exists(inst)
                            && h.instance_vars(inst).is_none()
                        {
                            return self.fail(f, pos, format!("Unknown instance {}.", inst));
                        }
                        h.get_builtin(&f.ctx, inst, b, idx)
                            .map_err(|e| self.loc(f, pos, e))
                    }
                    _ => self.fail(
                        f,
                        pos,
                        format!("{} is not a global or local variable.", b.name()),
                    ),
                }
            }
            FieldName::User(var) => match self.read_owner(h, f, o, pos)? {
                ReadOwner::Inst(inst) => self.read_inst(h, f, inst, var, i, j, pos),
                ReadOwner::Global => self.read_global(f, var, i, j, pos),
                ReadOwner::Local => match Self::local_slot(f, var) {
                    Some(s) => self.read_local(f, s, i, j, pos),
                    None => self.uninit(f, var, i, j, pos),
                },
            },
        }
    }

    // ------------------------------------------------------------ writes

    #[inline(never)]
    fn assign(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        p: &Place,
        op: Option<BinOp>,
        rhs: &Ex,
    ) -> R<()> {
        if let PlaceKind::Field(owner, name) = &p.kind {
            return self.assign_field(h, f, p, owner, *name, op, rhs);
        }
        // Fast path: `name op= constant` on a real scalar instance variable of self
        // (loop counters), updated in place with one lookup.
        if let (
            Some(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul)),
            Ex::Const(Value::Real(rv)),
            Idx::None,
        ) = (op, rhs, &p.idx)
        {
            let var = match p.kind {
                PlaceKind::User(v) if !self.is_globalvar(v) => Some(v),
                PlaceKind::SelfUser(v) => Some(v),
                _ => None,
            };
            if let Some(var) = var {
                if let Some(Var::Scalar(Value::Real(cur))) =
                    h.instance_vars(f.ctx.self_id).and_then(|t| t.get_mut(var))
                {
                    *cur = match op {
                        BinOp::Add => *cur + *rv,
                        BinOp::Sub => *cur - *rv,
                        _ => *cur * *rv,
                    };
                    return Ok(());
                }
            }
        }
        let (i, j) = self.eval_idx(h, f, &p.idx, p.pos)?;
        let v = match op {
            None => self.eval(h, f, rhs)?,
            Some(op) => {
                let cur = self.read_resolved(h, f, &p.kind, i, j, p.pos)?;
                let r = self.eval(h, f, rhs)?;
                self.bin(f, p.pos, op, &cur, &r)?
            }
        };
        self.write_resolved(h, f, &p.kind, i, j, v, p.pos)
    }

    fn write_local(
        &self,
        f: &mut Frame,
        slot: u16,
        i: usize,
        j: usize,
        v: Value,
        pos: Pos,
    ) -> R<()> {
        let r = match &mut f.locals[slot as usize] {
            Some(var) => var.set(i, j, v),
            none => Var::new_at(i, j, v).map(|var| *none = Some(var)),
        };
        r.or_else(|m| self.fail(f, pos, m))
    }

    fn write_global(
        &mut self,
        f: &Frame,
        var: VarId,
        i: usize,
        j: usize,
        v: Value,
        pos: Pos,
    ) -> R<()> {
        match self.globals.set_elem(var, i, j, v) {
            Ok(()) => Ok(()),
            Err(m) => self.fail(f, pos, m),
        }
    }

    /// Writes an instance variable. A missing instance is a no-op unless `strict`.
    fn write_inst(
        &self,
        h: &mut dyn Host,
        f: &Frame,
        inst: i32,
        var: VarId,
        i: usize,
        j: usize,
        v: Value,
        pos: Pos,
        strict: bool,
    ) -> R<()> {
        match h.instance_vars(inst) {
            Some(t) => match t.set_elem(var, i, j, v) {
                Ok(()) => Ok(()),
                Err(m) => self.fail(f, pos, m),
            },
            None if strict => self.fail(
                f,
                pos,
                format!(
                    "Cannot assign variable {}: instance {} does not exist.",
                    self.interner.name(var),
                    inst
                ),
            ),
            None => Ok(()),
        }
    }

    fn write_arg(&self, f: &mut Frame, a: ArgVar, i: usize, j: usize, v: Value, pos: Pos) -> R<()> {
        match a {
            ArgVar::Slot(n) if i == 0 && j == 0 => f.args[n as usize] = v,
            ArgVar::Array if i == 0 && j < 16 => f.args[j] = v,
            ArgVar::Slot(_) | ArgVar::Array => {
                return self.fail(f, pos, "Argument index out of range (0..15).")
            }
            ArgVar::Count | ArgVar::Relative => {
                return self.fail(f, pos, "Cannot assign to a read-only variable.")
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn write_resolved(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        k: &PlaceKind,
        i: usize,
        j: usize,
        v: Value,
        pos: Pos,
    ) -> R<()> {
        match k {
            PlaceKind::Local(s) => self.write_local(f, *s, i, j, v, pos),
            PlaceKind::User(var) => {
                if self.is_globalvar(*var) {
                    self.write_global(f, *var, i, j, v, pos)
                } else {
                    self.write_inst(h, f, f.ctx.self_id, *var, i, j, v, pos, true)
                }
            }
            PlaceKind::SelfUser(var) => {
                self.write_inst(h, f, f.ctx.self_id, *var, i, j, v, pos, true)
            }
            PlaceKind::Global(var) => self.write_global(f, *var, i, j, v, pos),
            PlaceKind::Builtin(b) => {
                let idx = self.builtin_index(f, *b, i, j, pos)?;
                h.set_builtin(&f.ctx, f.ctx.self_id, *b, idx, v)
                    .map_err(|e| self.loc(f, pos, e))
            }
            PlaceKind::Arg(a) => self.write_arg(f, *a, i, j, v, pos),
            PlaceKind::Field(..) => unreachable!("handled by assign_field"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn assign_field(
        &mut self,
        h: &mut dyn Host,
        f: &mut Frame,
        p: &Place,
        owner: &Ex,
        name: FieldName,
        op: Option<BinOp>,
        rhs: &Ex,
    ) -> R<()> {
        let pos = p.pos;
        let ov = self.eval(h, f, owner)?;
        let o = self.owner_id(f, &ov, pos)?;
        let (i, j) = self.eval_idx(h, f, &p.idx, pos)?;
        let v = match op {
            None => self.eval(h, f, rhs)?,
            Some(op) => {
                let cur = self.read_field(h, f, o, name, i, j, pos)?;
                let r = self.eval(h, f, rhs)?;
                self.bin(f, pos, op, &cur, &r)?
            }
        };
        match name {
            FieldName::Arg(a) => return self.write_arg(f, a, i, j, v, pos),
            FieldName::Builtin(b) if b.info().scope == VarScope::Global => {
                let idx = self.builtin_index(f, b, i, j, pos)?;
                return h
                    .set_builtin(&f.ctx, f.ctx.self_id, b, idx, v)
                    .map_err(|e| self.loc(f, pos, e));
            }
            _ => {}
        }
        match o {
            special::GLOBAL => {
                return match name {
                    FieldName::User(var) => self.write_global(f, var, i, j, v, pos),
                    _ => Ok(()),
                }
            }
            special::LOCAL => {
                return match name {
                    FieldName::User(var) => match Self::local_slot(f, var) {
                        Some(s) => self.write_local(f, s, i, j, v, pos),
                        None => self.fail(
                            f,
                            pos,
                            format!(
                                "{} is not a local variable of this code.",
                                self.interner.name(var)
                            ),
                        ),
                    },
                    _ => Ok(()),
                }
            }
            _ => {}
        }
        let mut targets = self.select_pool.pop().unwrap_or_default();
        self.write_targets(h, f, o, &mut targets);
        let mut result = Ok(());
        for &t in &targets {
            let r = match name {
                FieldName::User(var) => self.write_inst(h, f, t, var, i, j, v.clone(), pos, false),
                FieldName::Builtin(b) => {
                    if t != f.ctx.self_id && !h.instance_exists(t) && h.instance_vars(t).is_none() {
                        continue;
                    }
                    match self.builtin_index(f, b, i, j, pos) {
                        Ok(idx) => h
                            .set_builtin(&f.ctx, t, b, idx, v.clone())
                            .map_err(|e| self.loc(f, pos, e)),
                        Err(e) => Err(e),
                    }
                }
                FieldName::Arg(_) => unreachable!(),
            };
            if r.is_err() {
                result = r;
                break;
            }
        }
        targets.clear();
        self.select_pool.push(targets);
        result
    }
}
