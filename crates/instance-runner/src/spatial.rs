//! Acceleration structures for instance queries (implementation detail, no
//! behavioural effect):
//!
//! * [`SpatialIndex`]: a uniform grid over instance bounding boxes, used as the
//!   broad phase of every collision query (place_*, instance_place,
//!   collision_*, position_*, collision events). Instances obtained mutably
//!   through [`Game::inst_mut`] are marked dirty and re-binned lazily before
//!   the next query; structural changes (instance list reordered, sprites
//!   edited, bulk updates) invalidate the whole index, which is then rebuilt
//!   in O(n).
//! * [`ObjLists`]: per-object lists of instance positions (an instance is listed
//!   under its object and every ancestor), so selecting the instances of an
//!   object does not scan the whole instance list.
//!
//! Query results are positions in `Game::instances`, sorted ascending, so callers
//! keep the instance-order semantics of runtime.md.

use crate::game::Game;
use gml::IdBuildHasher;
use std::collections::HashMap;

/// Grid cell size in pixels.
const CELL: i32 = 32;
/// Instances whose box spans more cells than this are kept in a separate list
/// that every query scans.
const MAX_CELLS: i64 = 64;

#[inline]
fn cell_of(v: i32) -> i32 {
    v.div_euclid(CELL)
}

#[inline]
fn key(cx: i32, cy: i32) -> u64 {
    ((cx as u32 as u64) << 32) | cy as u32 as u64
}

/// Where an instance is filed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    /// Not in the index (no collision mask, destroyed or deactivated).
    None,
    /// In the grid cells covered by this inclusive box.
    Grid([i32; 4]),
    /// In the `big` list with this box.
    Big([i32; 4]),
}

#[derive(Default)]
pub struct SpatialIndex {
    /// Whether `slots`/`cells` reflect the instance list (apart from `dirty`).
    valid: bool,
    slots: Vec<Slot>,
    dirty: Vec<u32>,
    is_dirty: Vec<bool>,
    cells: HashMap<u64, Vec<u32>, IdBuildHasher>,
    big: Vec<u32>,
    /// De-duplication stamps for queries.
    stamp: Vec<u32>,
    stamp_now: u32,
}

impl SpatialIndex {
    /// Forgets everything; the next query rebuilds the index.
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Marks the instance at position `k` as possibly changed.
    #[inline]
    pub fn mark(&mut self, k: usize) {
        if !self.valid {
            return;
        }
        if k >= self.is_dirty.len() {
            self.is_dirty.resize(k + 1, false);
        }
        if !self.is_dirty[k] {
            self.is_dirty[k] = true;
            self.dirty.push(k as u32);
        }
    }

    fn slot_for(g: &Game, k: usize) -> Slot {
        let i = &g.instances[k];
        if !i.alive() {
            return Slot::None;
        }
        match g.geo_at(i, i.x, i.y) {
            Some(geo) => {
                let b = geo.bbox;
                let cw = (cell_of(b[2]) as i64 - cell_of(b[0]) as i64 + 1) * (cell_of(b[3]) as i64 - cell_of(b[1]) as i64 + 1);
                if cw > MAX_CELLS {
                    Slot::Big(b)
                } else {
                    Slot::Grid(b)
                }
            }
            None => Slot::None,
        }
    }

    fn insert(&mut self, k: u32, s: Slot) {
        match s {
            Slot::None => {}
            Slot::Big(_) => self.big.push(k),
            Slot::Grid(b) => {
                for cx in cell_of(b[0])..=cell_of(b[2]) {
                    for cy in cell_of(b[1])..=cell_of(b[3]) {
                        self.cells.entry(key(cx, cy)).or_default().push(k);
                    }
                }
            }
        }
    }

    fn remove(&mut self, k: u32, s: Slot) {
        match s {
            Slot::None => {}
            Slot::Big(_) => {
                if let Some(p) = self.big.iter().position(|&x| x == k) {
                    self.big.swap_remove(p);
                }
            }
            Slot::Grid(b) => {
                for cx in cell_of(b[0])..=cell_of(b[2]) {
                    for cy in cell_of(b[1])..=cell_of(b[3]) {
                        if let Some(v) = self.cells.get_mut(&key(cx, cy)) {
                            if let Some(p) = v.iter().position(|&x| x == k) {
                                v.swap_remove(p);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Brings the index up to date with the instance list.
    pub fn flush(&mut self, g: &Game) {
        let n = g.instances.len();
        if !self.valid {
            self.cells.clear();
            self.big.clear();
            self.dirty.clear();
            self.is_dirty.clear();
            self.is_dirty.resize(n, false);
            self.slots.clear();
            self.slots.reserve(n);
            for k in 0..n {
                let s = Self::slot_for(g, k);
                self.slots.push(s);
                self.insert(k as u32, s);
            }
            self.valid = true;
            return;
        }
        if self.slots.len() < n {
            // new instances appended since the last flush
            for k in self.slots.len()..n {
                self.slots.push(Slot::None);
                self.mark(k);
            }
        } else if self.slots.len() > n {
            // the list shrank without an invalidation: be safe
            self.valid = false;
            return self.flush(g);
        }
        let dirty = std::mem::take(&mut self.dirty);
        for &k in &dirty {
            let k = k as usize;
            if k < self.is_dirty.len() {
                self.is_dirty[k] = false;
            }
            if k >= n {
                continue;
            }
            let new = Self::slot_for(g, k);
            let old = self.slots[k];
            if new != old {
                self.remove(k as u32, old);
                self.insert(k as u32, new);
                self.slots[k] = new;
            }
        }
        let mut dirty = dirty;
        dirty.clear();
        self.dirty = dirty;
    }

    /// The bounding box of the instance at position `k` as of the last
    /// [`flush`](Self::flush), or `None` if it has no collision mask (or is dead).
    #[inline]
    pub fn cached_bbox(&self, k: usize) -> Option<[i32; 4]> {
        match self.slots.get(k) {
            Some(Slot::Grid(b) | Slot::Big(b)) => Some(*b),
            _ => None,
        }
    }

    /// Positions of indexed instances whose box overlaps `b` (inclusive),
    /// sorted ascending. Call [`flush`](Self::flush) first.
    pub fn query(&mut self, b: [i32; 4], out: &mut Vec<u32>) {
        out.clear();
        if b[0] > b[2] || b[1] > b[3] {
            return;
        }
        self.stamp_now = self.stamp_now.wrapping_add(1);
        if self.stamp_now == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.stamp_now = 1;
        }
        if self.stamp.len() < self.slots.len() {
            self.stamp.resize(self.slots.len(), 0);
        }
        let overlaps = |s: &[i32; 4]| s[0] <= b[2] && s[2] >= b[0] && s[1] <= b[3] && s[3] >= b[1];
        let (cx0, cx1, cy0, cy1) = (cell_of(b[0]), cell_of(b[2]), cell_of(b[1]), cell_of(b[3]));
        let ncells = (cx1 as i64 - cx0 as i64 + 1) * (cy1 as i64 - cy0 as i64 + 1);
        if ncells > self.cells.len() as i64 * 2 + 16 {
            // a huge query box: scanning the instances is cheaper than the cells
            for (k, s) in self.slots.iter().enumerate() {
                if let Slot::Grid(sb) | Slot::Big(sb) = s {
                    if overlaps(sb) {
                        out.push(k as u32);
                    }
                }
            }
            return;
        }
        for cx in cx0..=cx1 {
            for cy in cy0..=cy1 {
                if let Some(v) = self.cells.get(&key(cx, cy)) {
                    for &k in v {
                        let ku = k as usize;
                        if self.stamp[ku] == self.stamp_now {
                            continue;
                        }
                        self.stamp[ku] = self.stamp_now;
                        if let Slot::Grid(sb) = &self.slots[ku] {
                            if overlaps(sb) {
                                out.push(k);
                            }
                        }
                    }
                }
            }
        }
        for &k in &self.big {
            if let Slot::Big(sb) = &self.slots[k as usize] {
                if overlaps(sb) {
                    out.push(k);
                }
            }
        }
        out.sort_unstable();
    }
}

/// Per-object instance position lists (see module docs).
#[derive(Default)]
pub struct ObjLists {
    valid: bool,
    /// object index → ascending positions in `Game::instances` of instances whose
    /// object is that object or a descendant.
    lists: Vec<Vec<u32>>,
}

impl ObjLists {
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    fn rebuild(&mut self, g: &Game) {
        let n = g.objects.len();
        for l in self.lists.iter_mut() {
            l.clear();
        }
        self.lists.resize_with(n, Vec::new);
        for (k, i) in g.instances.iter().enumerate() {
            if let Some(chain) = g.ancestors.get(i.object_index.max(0) as usize) {
                if i.object_index < 0 {
                    continue;
                }
                for &a in chain {
                    self.lists[a as usize].push(k as u32);
                }
            }
        }
        self.valid = true;
    }

    /// Records an instance appended at position `k` (keeps lists sorted).
    pub fn appended(&mut self, g: &Game, k: usize) {
        if !self.valid {
            return;
        }
        let obj = g.instances[k].object_index;
        if obj < 0 {
            return;
        }
        if let Some(chain) = g.ancestors.get(obj as usize) {
            for &a in chain {
                if let Some(l) = self.lists.get_mut(a as usize) {
                    l.push(k as u32);
                }
            }
        }
    }

    /// Records that the instance at position `k` changed object from `old`.
    pub fn changed(&mut self, g: &Game, k: usize, old: i32) {
        if !self.valid {
            return;
        }
        let k32 = k as u32;
        if old >= 0 {
            if let Some(chain) = g.ancestors.get(old as usize) {
                for &a in chain {
                    if let Some(l) = self.lists.get_mut(a as usize) {
                        if let Ok(p) = l.binary_search(&k32) {
                            l.remove(p);
                        }
                    }
                }
            }
        }
        let new = g.instances[k].object_index;
        if new >= 0 {
            if let Some(chain) = g.ancestors.get(new as usize) {
                for &a in chain {
                    if let Some(l) = self.lists.get_mut(a as usize) {
                        if let Err(p) = l.binary_search(&k32) {
                            l.insert(p, k32);
                        }
                    }
                }
            }
        }
    }

    /// Calls `f` with the position of every instance of `obj` (incl. descendants)
    /// in instance order; stops when `f` returns false.
    pub fn for_each(&mut self, g: &Game, obj: i32, mut f: impl FnMut(usize) -> bool) {
        if !self.valid {
            self.rebuild(g);
        }
        if let Some(l) = self.lists.get(obj.max(0) as usize) {
            if obj < 0 {
                return;
            }
            for &k in l {
                if (k as usize) < g.instances.len() && !f(k as usize) {
                    break;
                }
            }
        }
    }
}
