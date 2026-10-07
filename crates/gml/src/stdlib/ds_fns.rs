//! `ds_*` functions (chapter 411).

use super::{n, Args, NResult, NativeFn};
use crate::ds::*;
use crate::error::Error;
use crate::value::{binary, BinOp, Value};
use crate::vm::Vm;
use std::cmp::Ordering;
use std::collections::VecDeque;

fn missing(a: &Args, i: usize) -> Error {
    Error::new(format!(
        "Data structure with index {} does not exist (in {}).",
        crate::value::real_to_string(a.real(i).unwrap_or(-1.0)),
        a.func.name()
    ))
}

fn id(a: &Args, i: usize) -> Result<usize, Error> {
    let v = a.int(i)?;
    if v < 0 {
        Err(missing(a, i))
    } else {
        Ok(v as usize)
    }
}

macro_rules! getter {
    ($name:ident, $field:ident, $t:ty) => {
        fn $name<'v>(vm: &'v mut Vm, a: &Args, i: usize) -> Result<&'v mut $t, Error> {
            let k = id(a, i)?;
            vm.ds.$field.get_mut(k).ok_or_else(|| missing(a, i))
        }
    };
}
getter!(stack, stacks, Vec<Value>);
getter!(queue, queues, VecDeque<Value>);
getter!(list, lists, Vec<Value>);
getter!(map, maps, Vec<(Value, Value)>);
getter!(prio, priorities, Vec<(Value, Value)>);
getter!(grid, grids, Grid);

fn zero() -> NResult {
    Ok(Value::Real(0.0))
}
fn real(x: usize) -> NResult {
    Ok(Value::Real(x as f64))
}

fn bad_string(a: &Args) -> Error {
    Error::new(format!("Invalid data string passed to {}.", a.func.name()))
}

fn shuffle<T>(vm: &mut Vm, v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = ((vm.random() * (i + 1) as f64) as usize).min(i);
        v.swap(i, j);
    }
}

// ------------------------------------------------------------------ priority helpers

fn prio_extreme(ds: &DsStore, v: &[(Value, Value)], want: Ordering) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, (_, p)) in v.iter().enumerate() {
        match best {
            None => best = Some(i),
            Some(b) => {
                if ds.cmp(p, &v[b].1) == want {
                    best = Some(i)
                }
            }
        }
    }
    best
}

// ------------------------------------------------------------------ grid helpers

/// Normalised, clipped inclusive region; `None` if empty.
fn region(g: &Grid, a: &Args, first: usize) -> Result<Option<(usize, usize, usize, usize)>, Error> {
    let (mut x1, mut y1, mut x2, mut y2) = (
        a.int(first)?,
        a.int(first + 1)?,
        a.int(first + 2)?,
        a.int(first + 3)?,
    );
    if x1 > x2 {
        std::mem::swap(&mut x1, &mut x2);
    }
    if y1 > y2 {
        std::mem::swap(&mut y1, &mut y2);
    }
    let x1 = x1.max(0);
    let y1 = y1.max(0);
    let x2 = x2.min(g.w as i64 - 1);
    let y2 = y2.min(g.h as i64 - 1);
    if x1 > x2 || y1 > y2 {
        return Ok(None);
    }
    Ok(Some((x1 as usize, y1 as usize, x2 as usize, y2 as usize)))
}

fn region_cells(g: &Grid, r: Option<(usize, usize, usize, usize)>) -> Vec<usize> {
    let mut out = Vec::new();
    if let Some((x1, y1, x2, y2)) = r {
        for y in y1..=y2 {
            for x in x1..=x2 {
                out.push(x + y * g.w);
            }
        }
    }
    out
}

fn disk_cells(g: &Grid, a: &Args, first: usize) -> Result<Vec<usize>, Error> {
    let (xm, ym, r) = (a.real(first)?, a.real(first + 1)?, a.real(first + 2)?);
    let mut out = Vec::new();
    if g.w == 0 || g.h == 0 || r < 0.0 {
        return Ok(out);
    }
    let x1 = ((xm - r).floor().max(0.0)) as usize;
    let y1 = ((ym - r).floor().max(0.0)) as usize;
    let x2 = ((xm + r).ceil().min(g.w as f64 - 1.0)).max(-1.0);
    let y2 = ((ym + r).ceil().min(g.h as f64 - 1.0)).max(-1.0);
    if x2 < 0.0 || y2 < 0.0 {
        return Ok(out);
    }
    for y in y1..=y2 as usize {
        for x in x1..=x2 as usize {
            let (dx, dy) = (x as f64 - xm, y as f64 - ym);
            if dx * dx + dy * dy <= r * r {
                out.push(x + y * g.w);
            }
        }
    }
    Ok(out)
}

#[derive(Clone, Copy)]
enum CellOp {
    Set,
    Add,
    Mul,
}

fn apply(cell: &mut Value, op: CellOp, v: &Value) {
    match op {
        CellOp::Set => *cell = v.clone(),
        CellOp::Add => {
            if let Ok(r) = binary(BinOp::Add, cell, v) {
                *cell = r
            }
        }
        CellOp::Mul => {
            if let (Value::Real(x), Value::Real(y)) = (&*cell, v) {
                *cell = Value::Real(x * y)
            }
        }
    }
}

fn grid_cell_op(vm: &mut Vm, a: &Args, op: CellOp) -> NResult {
    let (x, y) = (a.int(1)?, a.int(2)?);
    let v = a.val(3).clone();
    let g = grid(vm, a, 0)?;
    if let Some(c) = g.get_mut(x, y) {
        apply(c, op, &v);
    }
    zero()
}

fn grid_region_op(vm: &mut Vm, a: &Args, op: CellOp) -> NResult {
    let v = a.val(5).clone();
    let g = grid(vm, a, 0)?;
    let cells = region_cells(g, region(g, a, 1)?);
    for c in cells {
        apply(&mut g.cells[c], op, &v);
    }
    zero()
}

fn grid_disk_op(vm: &mut Vm, a: &Args, op: CellOp) -> NResult {
    let v = a.val(4).clone();
    let g = grid(vm, a, 0)?;
    for c in disk_cells(g, a, 1)? {
        apply(&mut g.cells[c], op, &v);
    }
    zero()
}

fn grid_grid_op(vm: &mut Vm, a: &Args, op: CellOp) -> NResult {
    let (xpos, ypos) = (a.int(6)?, a.int(7)?);
    let src = grid(vm, a, 1)?;
    // Read the source block first (the grids may be the same).
    let (sx1, sy1, sx2, sy2) = (a.int(2)?, a.int(3)?, a.int(4)?, a.int(5)?);
    let (sx1, sx2) = if sx1 <= sx2 { (sx1, sx2) } else { (sx2, sx1) };
    let (sy1, sy2) = if sy1 <= sy2 { (sy1, sy2) } else { (sy2, sy1) };
    let mut block = Vec::new();
    for y in sy1..=sy2 {
        for x in sx1..=sx2 {
            if let Some(v) = src.get(x, y) {
                block.push((x - sx1, y - sy1, v.clone()));
            }
        }
    }
    let dst = grid(vm, a, 0)?;
    for (dx, dy, v) in block {
        if let Some(c) = dst.get_mut(xpos + dx, ypos + dy) {
            apply(c, op, &v);
        }
    }
    zero()
}

#[derive(Clone, Copy)]
enum Agg {
    Sum,
    Min,
    Max,
    Mean,
}

fn aggregate(g: &Grid, cells: &[usize], agg: Agg) -> f64 {
    let nums: Vec<f64> = cells.iter().filter_map(|&c| g.cells[c].as_real()).collect();
    if nums.is_empty() {
        return 0.0;
    }
    match agg {
        Agg::Sum => nums.iter().sum(),
        Agg::Mean => nums.iter().sum::<f64>() / nums.len() as f64,
        Agg::Min => nums.iter().copied().fold(f64::INFINITY, f64::min),
        Agg::Max => nums.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

fn grid_region_agg(vm: &mut Vm, a: &Args, agg: Agg) -> NResult {
    let g = grid(vm, a, 0)?;
    let cells = region_cells(g, region(g, a, 1)?);
    Ok(Value::Real(aggregate(g, &cells, agg)))
}

fn grid_disk_agg(vm: &mut Vm, a: &Args, agg: Agg) -> NResult {
    let g = grid(vm, a, 0)?;
    let cells = disk_cells(g, a, 1)?;
    Ok(Value::Real(aggregate(g, &cells, agg)))
}

/// Finds a value in the cells; returns its (x, y).
fn grid_find(vm: &mut Vm, a: &Args, disk: bool) -> Result<Option<(usize, usize)>, Error> {
    let vi = if disk { 4 } else { 5 };
    let v = a.val(vi).clone();
    let k = id(a, 0)?;
    let g = vm.ds.grids.get(k).ok_or_else(|| missing(a, 0))?;
    let cells = if disk {
        disk_cells(g, a, 1)?
    } else {
        region_cells(g, region(g, a, 1)?)
    };
    Ok(cells
        .into_iter()
        .find(|&c| vm.ds.eq(&g.cells[c], &v))
        .map(|c| (c % g.w, c / g.w)))
}

fn write_seq<'a>(tag: u16, items: impl ExactSizeIterator<Item = &'a Value>) -> NResult {
    let mut w = Writer::default();
    w.tag(tag);
    w.count(items.len());
    for v in items {
        w.value(v);
    }
    Ok(Value::from(w.0))
}

fn read_seq(a: &Args, tag: u16) -> Result<Vec<Value>, Error> {
    let s = a.str(1)?;
    let mut r = Reader::new(s);
    let parse = |r: &mut Reader| -> Option<Vec<Value>> {
        r.tag(tag)?;
        let n = r.count()?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(r.value()?);
        }
        r.done().then_some(v)
    };
    parse(&mut r).ok_or_else(|| bad_string(a))
}

fn write_pairs(tag: u16, items: &[(Value, Value)]) -> NResult {
    let mut w = Writer::default();
    w.tag(tag);
    w.count(items.len());
    for (k, v) in items {
        w.value(k);
        w.value(v);
    }
    Ok(Value::from(w.0))
}

fn read_pairs(a: &Args, tag: u16) -> Result<Vec<(Value, Value)>, Error> {
    let s = a.str(1)?;
    let mut r = Reader::new(s);
    let parse = |r: &mut Reader| -> Option<Vec<(Value, Value)>> {
        r.tag(tag)?;
        let n = r.count()?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let k = r.value()?;
            v.push((k, r.value()?));
        }
        r.done().then_some(v)
    };
    parse(&mut r).ok_or_else(|| bad_string(a))
}

fn map_insert(ds: &DsStore, m: &mut Vec<(Value, Value)>, k: Value, v: Value) {
    let at = m.partition_point(|(e, _)| ds.cmp(e, &k) != Ordering::Greater);
    m.insert(at, (k, v));
}

pub(super) fn register(t: &mut Vec<(&'static str, NativeFn)>) {
    t.extend([
        n("ds_set_precision", |vm, _, _, a| {
            vm.ds.precision = a.real(0)?.abs();
            zero()
        }),
        // ---------------------------------------------------------- stacks
        n("ds_stack_create", |vm, _, _, _| {
            real(vm.ds.stacks.insert(Vec::new()))
        }),
        n("ds_stack_destroy", |vm, _, _, a| {
            let k = id(&a, 0)?;
            vm.ds.stacks.remove(k).ok_or_else(|| missing(&a, 0))?;
            zero()
        }),
        n("ds_stack_clear", |vm, _, _, a| {
            stack(vm, &a, 0)?.clear();
            zero()
        }),
        n("ds_stack_copy", |vm, _, _, a| {
            let src = stack(vm, &a, 1)?.clone();
            *stack(vm, &a, 0)? = src;
            zero()
        }),
        n("ds_stack_size", |vm, _, _, a| real(stack(vm, &a, 0)?.len())),
        n("ds_stack_empty", |vm, _, _, a| {
            Ok(Value::from(stack(vm, &a, 0)?.is_empty()))
        }),
        n("ds_stack_push", |vm, _, _, a| {
            stack(vm, &a, 0)?.push(a.val(1).clone());
            zero()
        }),
        n("ds_stack_pop", |vm, _, _, a| {
            Ok(stack(vm, &a, 0)?.pop().unwrap_or_default())
        }),
        n("ds_stack_top", |vm, _, _, a| {
            Ok(stack(vm, &a, 0)?.last().cloned().unwrap_or_default())
        }),
        n("ds_stack_write", |vm, _, _, a| {
            write_seq(TAG_STACK, stack(vm, &a, 0)?.iter())
        }),
        n("ds_stack_read", |vm, _, _, a| {
            let v = read_seq(&a, TAG_STACK)?;
            *stack(vm, &a, 0)? = v;
            zero()
        }),
        // ---------------------------------------------------------- queues
        n("ds_queue_create", |vm, _, _, _| {
            real(vm.ds.queues.insert(VecDeque::new()))
        }),
        n("ds_queue_destroy", |vm, _, _, a| {
            let k = id(&a, 0)?;
            vm.ds.queues.remove(k).ok_or_else(|| missing(&a, 0))?;
            zero()
        }),
        n("ds_queue_clear", |vm, _, _, a| {
            queue(vm, &a, 0)?.clear();
            zero()
        }),
        n("ds_queue_copy", |vm, _, _, a| {
            let src = queue(vm, &a, 1)?.clone();
            *queue(vm, &a, 0)? = src;
            zero()
        }),
        n("ds_queue_size", |vm, _, _, a| real(queue(vm, &a, 0)?.len())),
        n("ds_queue_empty", |vm, _, _, a| {
            Ok(Value::from(queue(vm, &a, 0)?.is_empty()))
        }),
        n("ds_queue_enqueue", |vm, _, _, a| {
            queue(vm, &a, 0)?.push_back(a.val(1).clone());
            zero()
        }),
        n("ds_queue_dequeue", |vm, _, _, a| {
            Ok(queue(vm, &a, 0)?.pop_front().unwrap_or_default())
        }),
        n("ds_queue_head", |vm, _, _, a| {
            Ok(queue(vm, &a, 0)?.front().cloned().unwrap_or_default())
        }),
        n("ds_queue_tail", |vm, _, _, a| {
            Ok(queue(vm, &a, 0)?.back().cloned().unwrap_or_default())
        }),
        n("ds_queue_write", |vm, _, _, a| {
            write_seq(TAG_QUEUE, queue(vm, &a, 0)?.iter())
        }),
        n("ds_queue_read", |vm, _, _, a| {
            let v = read_seq(&a, TAG_QUEUE)?;
            *queue(vm, &a, 0)? = v.into();
            zero()
        }),
        // ---------------------------------------------------------- lists
        n("ds_list_create", |vm, _, _, _| {
            real(vm.ds.lists.insert(Vec::new()))
        }),
        n("ds_list_destroy", |vm, _, _, a| {
            let k = id(&a, 0)?;
            vm.ds.lists.remove(k).ok_or_else(|| missing(&a, 0))?;
            zero()
        }),
        n("ds_list_clear", |vm, _, _, a| {
            list(vm, &a, 0)?.clear();
            zero()
        }),
        n("ds_list_copy", |vm, _, _, a| {
            let src = list(vm, &a, 1)?.clone();
            *list(vm, &a, 0)? = src;
            zero()
        }),
        n("ds_list_size", |vm, _, _, a| real(list(vm, &a, 0)?.len())),
        n("ds_list_empty", |vm, _, _, a| {
            Ok(Value::from(list(vm, &a, 0)?.is_empty()))
        }),
        n("ds_list_add", |vm, _, _, a| {
            list(vm, &a, 0)?.push(a.val(1).clone());
            zero()
        }),
        n("ds_list_insert", |vm, _, _, a| {
            let p = a.int(1)?;
            let l = list(vm, &a, 0)?;
            if p >= 0 && p as usize <= l.len() {
                l.insert(p as usize, a.val(2).clone());
            }
            zero()
        }),
        n("ds_list_replace", |vm, _, _, a| {
            let p = a.int(1)?;
            let l = list(vm, &a, 0)?;
            if p >= 0 && (p as usize) < l.len() {
                l[p as usize] = a.val(2).clone();
            }
            zero()
        }),
        n("ds_list_delete", |vm, _, _, a| {
            let p = a.int(1)?;
            let l = list(vm, &a, 0)?;
            if p >= 0 && (p as usize) < l.len() {
                l.remove(p as usize);
            }
            zero()
        }),
        n("ds_list_find_index", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let l = vm.ds.lists.get(k).ok_or_else(|| missing(&a, 0))?;
            let v = a.val(1);
            Ok(Value::Real(
                l.iter()
                    .position(|x| vm.ds.eq(x, v))
                    .map(|p| p as f64)
                    .unwrap_or(-1.0),
            ))
        }),
        n("ds_list_find_value", |vm, _, _, a| {
            let p = a.int(1)?;
            let l = list(vm, &a, 0)?;
            Ok(if p >= 0 {
                l.get(p as usize).cloned().unwrap_or_default()
            } else {
                Value::default()
            })
        }),
        n("ds_list_sort", |vm, _, _, a| {
            let asc = a.val(1).is_true();
            let l = list(vm, &a, 0)?;
            if asc {
                l.sort_by(|x, y| x.total_cmp(y));
            } else {
                l.sort_by(|x, y| y.total_cmp(x));
            }
            zero()
        }),
        n("ds_list_shuffle", |vm, _, _, a| {
            let mut l = std::mem::take(list(vm, &a, 0)?);
            shuffle(vm, &mut l);
            *list(vm, &a, 0)? = l;
            zero()
        }),
        n("ds_list_write", |vm, _, _, a| {
            write_seq(TAG_LIST, list(vm, &a, 0)?.iter())
        }),
        n("ds_list_read", |vm, _, _, a| {
            let v = read_seq(&a, TAG_LIST)?;
            *list(vm, &a, 0)? = v;
            zero()
        }),
        // ---------------------------------------------------------- maps
        n("ds_map_create", |vm, _, _, _| {
            real(vm.ds.maps.insert(Vec::new()))
        }),
        n("ds_map_destroy", |vm, _, _, a| {
            let k = id(&a, 0)?;
            vm.ds.maps.remove(k).ok_or_else(|| missing(&a, 0))?;
            zero()
        }),
        n("ds_map_clear", |vm, _, _, a| {
            map(vm, &a, 0)?.clear();
            zero()
        }),
        n("ds_map_copy", |vm, _, _, a| {
            let src = map(vm, &a, 1)?.clone();
            *map(vm, &a, 0)? = src;
            zero()
        }),
        n("ds_map_size", |vm, _, _, a| real(map(vm, &a, 0)?.len())),
        n("ds_map_empty", |vm, _, _, a| {
            Ok(Value::from(map(vm, &a, 0)?.is_empty()))
        }),
        n("ds_map_add", |vm, _, _, a| {
            let mut m = std::mem::take(map(vm, &a, 0)?);
            map_insert(&vm.ds, &mut m, a.val(1).clone(), a.val(2).clone());
            *map(vm, &a, 0)? = m;
            zero()
        }),
        n("ds_map_replace", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let m = vm.ds.maps.get(k).ok_or_else(|| missing(&a, 0))?;
            if let Some(p) = vm.ds.map_find(m, a.val(1)) {
                vm.ds.maps.get_mut(k).unwrap()[p].1 = a.val(2).clone();
            }
            zero()
        }),
        n("ds_map_delete", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let m = vm.ds.maps.get(k).ok_or_else(|| missing(&a, 0))?;
            if let Some(p) = vm.ds.map_find(m, a.val(1)) {
                vm.ds.maps.get_mut(k).unwrap().remove(p);
            }
            zero()
        }),
        n("ds_map_exists", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let m = vm.ds.maps.get(k).ok_or_else(|| missing(&a, 0))?;
            Ok(Value::from(vm.ds.map_find(m, a.val(1)).is_some()))
        }),
        n("ds_map_find_value", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let m = vm.ds.maps.get(k).ok_or_else(|| missing(&a, 0))?;
            Ok(vm
                .ds
                .map_find(m, a.val(1))
                .map(|p| m[p].1.clone())
                .unwrap_or_default())
        }),
        n("ds_map_find_previous", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let m = vm.ds.maps.get(k).ok_or_else(|| missing(&a, 0))?;
            let key = a.val(1);
            let p = m.partition_point(|(e, _)| vm.ds.cmp(e, key) == Ordering::Less);
            Ok(if p > 0 {
                m[p - 1].0.clone()
            } else {
                Value::default()
            })
        }),
        n("ds_map_find_next", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let m = vm.ds.maps.get(k).ok_or_else(|| missing(&a, 0))?;
            let key = a.val(1);
            let p = m.partition_point(|(e, _)| vm.ds.cmp(e, key) != Ordering::Greater);
            Ok(m.get(p).map(|e| e.0.clone()).unwrap_or_default())
        }),
        n("ds_map_find_first", |vm, _, _, a| {
            Ok(map(vm, &a, 0)?
                .first()
                .map(|e| e.0.clone())
                .unwrap_or_default())
        }),
        n("ds_map_find_last", |vm, _, _, a| {
            Ok(map(vm, &a, 0)?
                .last()
                .map(|e| e.0.clone())
                .unwrap_or_default())
        }),
        n("ds_map_write", |vm, _, _, a| {
            write_pairs(TAG_MAP, map(vm, &a, 0)?)
        }),
        n("ds_map_read", |vm, _, _, a| {
            let mut v = read_pairs(&a, TAG_MAP)?;
            v.sort_by(|x, y| x.0.total_cmp(&y.0));
            *map(vm, &a, 0)? = v;
            zero()
        }),
        // ---------------------------------------------------------- priority queues
        n("ds_priority_create", |vm, _, _, _| {
            real(vm.ds.priorities.insert(Vec::new()))
        }),
        n("ds_priority_destroy", |vm, _, _, a| {
            let k = id(&a, 0)?;
            vm.ds.priorities.remove(k).ok_or_else(|| missing(&a, 0))?;
            zero()
        }),
        n("ds_priority_clear", |vm, _, _, a| {
            prio(vm, &a, 0)?.clear();
            zero()
        }),
        n("ds_priority_copy", |vm, _, _, a| {
            let src = prio(vm, &a, 1)?.clone();
            *prio(vm, &a, 0)? = src;
            zero()
        }),
        n("ds_priority_size", |vm, _, _, a| {
            real(prio(vm, &a, 0)?.len())
        }),
        n("ds_priority_empty", |vm, _, _, a| {
            Ok(Value::from(prio(vm, &a, 0)?.is_empty()))
        }),
        n("ds_priority_add", |vm, _, _, a| {
            prio(vm, &a, 0)?.push((a.val(1).clone(), a.val(2).clone()));
            zero()
        }),
        n("ds_priority_change_priority", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let q = vm.ds.priorities.get(k).ok_or_else(|| missing(&a, 0))?;
            if let Some(p) = q.iter().position(|(v, _)| vm.ds.eq(v, a.val(1))) {
                vm.ds.priorities.get_mut(k).unwrap()[p].1 = a.val(2).clone();
            }
            zero()
        }),
        n("ds_priority_find_priority", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let q = vm.ds.priorities.get(k).ok_or_else(|| missing(&a, 0))?;
            Ok(q.iter()
                .find(|(v, _)| vm.ds.eq(v, a.val(1)))
                .map(|e| e.1.clone())
                .unwrap_or_default())
        }),
        n("ds_priority_delete_value", |vm, _, _, a| {
            let k = id(&a, 0)?;
            let q = vm.ds.priorities.get(k).ok_or_else(|| missing(&a, 0))?;
            if let Some(p) = q.iter().position(|(v, _)| vm.ds.eq(v, a.val(1))) {
                vm.ds.priorities.get_mut(k).unwrap().remove(p);
            }
            zero()
        }),
        n("ds_priority_delete_min", |vm, _, _, a| {
            prio_take(vm, &a, Ordering::Less, true)
        }),
        n("ds_priority_find_min", |vm, _, _, a| {
            prio_take(vm, &a, Ordering::Less, false)
        }),
        n("ds_priority_delete_max", |vm, _, _, a| {
            prio_take(vm, &a, Ordering::Greater, true)
        }),
        n("ds_priority_find_max", |vm, _, _, a| {
            prio_take(vm, &a, Ordering::Greater, false)
        }),
        n("ds_priority_write", |vm, _, _, a| {
            write_pairs(TAG_PRIORITY, prio(vm, &a, 0)?)
        }),
        n("ds_priority_read", |vm, _, _, a| {
            let v = read_pairs(&a, TAG_PRIORITY)?;
            *prio(vm, &a, 0)? = v;
            zero()
        }),
        // ---------------------------------------------------------- grids
        n("ds_grid_create", |vm, _, _, a| {
            let (w, h) = (a.int(0)?, a.int(1)?);
            if w < 0 || h < 0 || (w as u128) * (h as u128) > 100_000_000 {
                return Err(a.invalid());
            }
            real(vm.ds.grids.insert(Grid::new(w as usize, h as usize)))
        }),
        n("ds_grid_destroy", |vm, _, _, a| {
            let k = id(&a, 0)?;
            vm.ds.grids.remove(k).ok_or_else(|| missing(&a, 0))?;
            zero()
        }),
        n("ds_grid_copy", |vm, _, _, a| {
            let src = grid(vm, &a, 1)?.clone();
            *grid(vm, &a, 0)? = src;
            zero()
        }),
        n("ds_grid_resize", |vm, _, _, a| {
            let (w, h) = (a.int(1)?, a.int(2)?);
            if w < 0 || h < 0 || (w as u128) * (h as u128) > 100_000_000 {
                return Err(a.invalid());
            }
            grid(vm, &a, 0)?.resize(w as usize, h as usize);
            zero()
        }),
        n("ds_grid_width", |vm, _, _, a| real(grid(vm, &a, 0)?.w)),
        n("ds_grid_height", |vm, _, _, a| real(grid(vm, &a, 0)?.h)),
        n("ds_grid_clear", |vm, _, _, a| {
            let v = a.val(1).clone();
            grid(vm, &a, 0)?
                .cells
                .iter_mut()
                .for_each(|c| *c = v.clone());
            zero()
        }),
        n("ds_grid_set", |vm, _, _, a| {
            grid_cell_op(vm, &a, CellOp::Set)
        }),
        n("ds_grid_add", |vm, _, _, a| {
            grid_cell_op(vm, &a, CellOp::Add)
        }),
        n("ds_grid_multiply", |vm, _, _, a| {
            grid_cell_op(vm, &a, CellOp::Mul)
        }),
        n("ds_grid_set_region", |vm, _, _, a| {
            grid_region_op(vm, &a, CellOp::Set)
        }),
        n("ds_grid_add_region", |vm, _, _, a| {
            grid_region_op(vm, &a, CellOp::Add)
        }),
        n("ds_grid_multiply_region", |vm, _, _, a| {
            grid_region_op(vm, &a, CellOp::Mul)
        }),
        n("ds_grid_set_disk", |vm, _, _, a| {
            grid_disk_op(vm, &a, CellOp::Set)
        }),
        n("ds_grid_add_disk", |vm, _, _, a| {
            grid_disk_op(vm, &a, CellOp::Add)
        }),
        n("ds_grid_multiply_disk", |vm, _, _, a| {
            grid_disk_op(vm, &a, CellOp::Mul)
        }),
        n("ds_grid_set_grid_region", |vm, _, _, a| {
            grid_grid_op(vm, &a, CellOp::Set)
        }),
        n("ds_grid_add_grid_region", |vm, _, _, a| {
            grid_grid_op(vm, &a, CellOp::Add)
        }),
        n("ds_grid_multiply_grid_region", |vm, _, _, a| {
            grid_grid_op(vm, &a, CellOp::Mul)
        }),
        n("ds_grid_get", |vm, _, _, a| {
            let (x, y) = (a.int(1)?, a.int(2)?);
            Ok(grid(vm, &a, 0)?.get(x, y).cloned().unwrap_or_default())
        }),
        n("ds_grid_get_sum", |vm, _, _, a| {
            grid_region_agg(vm, &a, Agg::Sum)
        }),
        n("ds_grid_get_max", |vm, _, _, a| {
            grid_region_agg(vm, &a, Agg::Max)
        }),
        n("ds_grid_get_min", |vm, _, _, a| {
            grid_region_agg(vm, &a, Agg::Min)
        }),
        n("ds_grid_get_mean", |vm, _, _, a| {
            grid_region_agg(vm, &a, Agg::Mean)
        }),
        n("ds_grid_get_disk_sum", |vm, _, _, a| {
            grid_disk_agg(vm, &a, Agg::Sum)
        }),
        n("ds_grid_get_disk_min", |vm, _, _, a| {
            grid_disk_agg(vm, &a, Agg::Min)
        }),
        n("ds_grid_get_disk_max", |vm, _, _, a| {
            grid_disk_agg(vm, &a, Agg::Max)
        }),
        n("ds_grid_get_disk_mean", |vm, _, _, a| {
            grid_disk_agg(vm, &a, Agg::Mean)
        }),
        n("ds_grid_value_exists", |vm, _, _, a| {
            Ok(Value::from(grid_find(vm, &a, false)?.is_some()))
        }),
        n("ds_grid_value_x", |vm, _, _, a| {
            Ok(Value::Real(
                grid_find(vm, &a, false)?
                    .map(|p| p.0 as f64)
                    .unwrap_or(-1.0),
            ))
        }),
        n("ds_grid_value_y", |vm, _, _, a| {
            Ok(Value::Real(
                grid_find(vm, &a, false)?
                    .map(|p| p.1 as f64)
                    .unwrap_or(-1.0),
            ))
        }),
        n("ds_grid_value_disk_exists", |vm, _, _, a| {
            Ok(Value::from(grid_find(vm, &a, true)?.is_some()))
        }),
        n("ds_grid_value_disk_x", |vm, _, _, a| {
            Ok(Value::Real(
                grid_find(vm, &a, true)?.map(|p| p.0 as f64).unwrap_or(-1.0),
            ))
        }),
        n("ds_grid_value_disk_y", |vm, _, _, a| {
            Ok(Value::Real(
                grid_find(vm, &a, true)?.map(|p| p.1 as f64).unwrap_or(-1.0),
            ))
        }),
        n("ds_grid_shuffle", |vm, _, _, a| {
            let mut cells = std::mem::take(&mut grid(vm, &a, 0)?.cells);
            shuffle(vm, &mut cells);
            grid(vm, &a, 0)?.cells = cells;
            zero()
        }),
        n("ds_grid_write", |vm, _, _, a| {
            let g = grid(vm, &a, 0)?;
            let mut w = Writer::default();
            w.tag(TAG_GRID);
            w.count(g.w);
            w.count(g.h);
            for v in &g.cells {
                w.value(v);
            }
            Ok(Value::from(w.0))
        }),
        n("ds_grid_read", |vm, _, _, a| {
            let s = a.str(1)?;
            let mut r = Reader::new(s);
            let parse = |r: &mut Reader| -> Option<Grid> {
                r.tag(TAG_GRID)?;
                let (w, h) = (r.count()?, r.count()?);
                let mut cells = Vec::with_capacity(w.checked_mul(h)?.min(1 << 20));
                for _ in 0..w * h {
                    cells.push(r.value()?);
                }
                r.done().then_some(Grid { w, h, cells })
            };
            let g = parse(&mut r).ok_or_else(|| bad_string(&a))?;
            *grid(vm, &a, 0)? = g;
            zero()
        }),
    ]);
}

fn prio_take(vm: &mut Vm, a: &Args, want: Ordering, remove: bool) -> NResult {
    let k = id(a, 0)?;
    let q = vm.ds.priorities.get(k).ok_or_else(|| missing(a, 0))?;
    match prio_extreme(&vm.ds, q, want) {
        Some(p) => {
            if remove {
                Ok(vm.ds.priorities.get_mut(k).unwrap().remove(p).0)
            } else {
                Ok(q[p].0.clone())
            }
        }
        None => zero(),
    }
}
